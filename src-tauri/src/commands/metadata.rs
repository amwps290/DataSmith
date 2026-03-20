use crate::database::{ColumnInfo, DatabaseInfo, TableInfo, QueryResult, DatabaseType, SchemaInfo, FunctionInfo};
use crate::AppState;
use tauri::State;
use tracing::instrument;

/// 根据数据库类型生成表引用 SQL
fn format_table_reference(db_type: DatabaseType, database: &str, table: &str, schema: Option<&str>) -> String {
    match db_type {
        DatabaseType::PostgreSQL => {
            let schema_name = schema.unwrap_or("public");
            format!("\"{}\".\"{}\"", schema_name, table)
        }
        DatabaseType::MySQL => {
            format!("`{}`.`{}`", database, table)
        }
        DatabaseType::SQLite => {
            format!("\"{}\"", table)
        }
        _ => {
            format!("\"{}\".\"{}\"", database, table)
        }
    }
}

/// 获取数据库列表
#[tauri::command]
#[instrument(skip(state))]
pub async fn get_databases(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<DatabaseInfo>, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .get_databases(&connection_id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取表列表
#[tauri::command]
#[instrument(skip(state))]
pub async fn get_tables(
    connection_id: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .get_tables(&connection_id, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 获取表结构
#[tauri::command]
pub async fn get_table_structure(
    connection_id: String,
    table: String,
    schema: Option<String>,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<ColumnInfo>, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .get_table_structure(&connection_id, &table, schema.as_deref(), database.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 查看表数据
#[tauri::command]
pub async fn view_table_data(
    connection_id: String,
    table: String,
    database: String,
    schema: Option<String>,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let limit_clause = match limit {
        Some(l) => format!(" LIMIT {}", l),
        None => " LIMIT 1000".to_string(),
    };
    
    // 获取数据库类型
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // 根据数据库类型生成不同的 SQL
    let sql = match db_type {
        crate::database::DatabaseType::PostgreSQL => {
            // PostgreSQL 使用 schema.table 格式
            let schema_name = schema.unwrap_or_else(|| "public".to_string());
            format!("SELECT * FROM \"{}\".\"{}\"{}", schema_name, table, limit_clause)
        }
        crate::database::DatabaseType::MySQL => {
            // MySQL 使用 `database`.`table` 格式
            format!("SELECT * FROM `{}`.`{}`{}", database, table, limit_clause)
        }
        crate::database::DatabaseType::SQLite => {
            // SQLite 直接使用表名
            format!("SELECT * FROM \"{}\"{}", table, limit_clause)
        }
        _ => {
            // 默认使用标准 SQL
            format!("SELECT * FROM \"{}\".\"{}\"{}", database, table, limit_clause)
        }
    };
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 清空表数据
#[tauri::command]
pub async fn truncate_table(
    connection_id: String,
    table: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let table_ref = format_table_reference(db_type, &database, &table, schema.as_deref());
    let sql = format!("TRUNCATE TABLE {}", table_ref);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 删除表
#[tauri::command]
pub async fn drop_table(
    connection_id: String,
    table: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let table_ref = format_table_reference(db_type, &database, &table, schema.as_deref());
    let sql = format!("DROP TABLE {}", table_ref);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 获取视图列表
#[tauri::command]
pub async fn get_views(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .get_views(&connection_id, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 获取存储过程列表
#[tauri::command]
pub async fn get_procedures(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT ROUTINE_NAME, ROUTINE_TYPE, CREATED, LAST_ALTERED, ROUTINE_COMMENT
         FROM information_schema.ROUTINES 
         WHERE ROUTINE_SCHEMA = '{}' AND ROUTINE_TYPE = 'PROCEDURE'
         ORDER BY ROUTINE_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 获取函数列表
#[tauri::command]
pub async fn get_functions(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT ROUTINE_NAME, ROUTINE_TYPE, CREATED, LAST_ALTERED, ROUTINE_COMMENT
         FROM information_schema.ROUTINES 
         WHERE ROUTINE_SCHEMA = '{}' AND ROUTINE_TYPE = 'FUNCTION'
         ORDER BY ROUTINE_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 获取触发器列表
#[tauri::command]
pub async fn get_triggers(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT TRIGGER_NAME, EVENT_MANIPULATION, EVENT_OBJECT_TABLE, 
                ACTION_TIMING, CREATED
         FROM information_schema.TRIGGERS 
         WHERE TRIGGER_SCHEMA = '{}'
         ORDER BY TRIGGER_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 获取事件列表
#[tauri::command]
pub async fn get_events(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT EVENT_NAME, STATUS, EVENT_TYPE, EXECUTE_AT, 
                INTERVAL_VALUE, INTERVAL_FIELD, CREATED, LAST_ALTERED
         FROM information_schema.EVENTS 
         WHERE EVENT_SCHEMA = '{}'
         ORDER BY EVENT_NAME",
        database.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}

/// 删除视图
#[tauri::command]
pub async fn drop_view(
    connection_id: String,
    view: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let view_ref = format_table_reference(db_type, &database, &view, schema.as_deref());
    let sql = format!("DROP VIEW {}", view_ref);
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 获取视图定义
#[tauri::command]
pub async fn get_view_definition(
    connection_id: String,
    view: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = state.connection_manager.lock().await;
    
    let sql = format!(
        "SELECT VIEW_DEFINITION FROM information_schema.VIEWS 
         WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}'",
        database.replace("'", "''"),
        view.replace("'", "''")
    );
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    if let Some(row) = result.rows.first() {
        if let Some(definition) = row.get("VIEW_DEFINITION") {
            if let serde_json::Value::String(def) = definition {
                return Ok(def.clone());
            }
        }
    }
    
    Err("未找到视图定义".to_string())
}

/// 删除存储过程
#[tauri::command]
pub async fn drop_procedure(
    connection_id: String,
    procedure: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let sql = match db_type {
        DatabaseType::PostgreSQL => {
            let schema_name = schema.unwrap_or_else(|| "public".to_string());
            format!("DROP PROCEDURE \"{}\".\"{}\"", schema_name, procedure)
        }
        DatabaseType::MySQL => {
            format!("DROP PROCEDURE `{}`.`{}`", database, procedure)
        }
        _ => {
            format!("DROP PROCEDURE \"{}\"", procedure)
        }
    };
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 删除函数
#[tauri::command]
pub async fn drop_function(
    connection_id: String,
    function: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let sql = match db_type {
        DatabaseType::PostgreSQL => {
            let schema_name = schema.unwrap_or_else(|| "public".to_string());
            format!("DROP FUNCTION \"{}\".\"{}\"", schema_name, function)
        }
        DatabaseType::MySQL => {
            format!("DROP FUNCTION `{}`.`{}`", database, function)
        }
        _ => {
            format!("DROP FUNCTION \"{}\"", function)
        }
    };
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 删除触发器
#[tauri::command]
pub async fn drop_trigger(
    connection_id: String,
    trigger: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let sql = match db_type {
        DatabaseType::PostgreSQL => {
            let schema_name = schema.unwrap_or_else(|| "public".to_string());
            format!("DROP TRIGGER \"{}\".\"{}\"", schema_name, trigger)
        }
        DatabaseType::MySQL => {
            format!("DROP TRIGGER `{}`.`{}`", database, trigger)
        }
        _ => {
            format!("DROP TRIGGER \"{}\"", trigger)
        }
    };
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 删除事件
#[tauri::command]
pub async fn drop_event(
    connection_id: String,
    event: String,
    database: String,
    _schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // 注意：PostgreSQL 不支持 EVENT，这是 MySQL 特有的功能
    let sql = match db_type {
        DatabaseType::MySQL => {
            format!("DROP EVENT `{}`.`{}`", database, event)
        }
        _ => {
            return Err("该数据库类型不支持 EVENT".to_string());
        }
    };
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())
}

/// 获取表索引
#[tauri::command]
pub async fn get_table_indexes(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;

    let sql = match db_type {
        DatabaseType::MySQL => {
            format!(
                "SELECT DISTINCT INDEX_NAME as index_name, COLUMN_NAME as column_name, 
                        INDEX_TYPE as index_type, NON_UNIQUE as non_unique
                 FROM information_schema.STATISTICS 
                 WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}'
                 ORDER BY INDEX_NAME, SEQ_IN_INDEX",
                database.replace("'", "''"),
                table.replace("'", "''")
            )
        },
        DatabaseType::PostgreSQL => {
            let schema_name = schema.unwrap_or_else(|| "public".to_string());
            format!(
                "SELECT
                    i.relname as index_name,
                    a.attname as column_name,
                    t.relname as table_name,
                    ix.indisunique as is_unique,
                    am.amname as index_type
                FROM
                    pg_class t,
                    pg_class i,
                    pg_index ix,
                    pg_attribute a,
                    pg_am am,
                    pg_namespace n
                WHERE
                    t.oid = ix.indrelid
                    AND i.oid = ix.indexrelid
                    AND a.attrelid = t.oid
                    AND a.attnum = ANY(ix.indkey)
                    AND t.relkind = 'r'
                    AND i.relam = am.oid
                    AND t.relnamespace = n.oid
                    AND n.nspname = '{}'
                    AND t.relname = '{}'
                ORDER BY
                    t.relname,
                    i.relname",
                schema_name.replace("'", "''"),
                table.replace("'", "''")
            )
        },
        DatabaseType::SQLite => {
            format!("PRAGMA index_list('{}')", table.replace("'", "''"))
        },
        _ => return Ok(Vec::new()),
    };
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    // 统一字段名以适配前端
    let processed_rows = result.rows.into_iter().map(|row| {
        let mut new_row = std::collections::HashMap::new();
        match db_type {
            DatabaseType::PostgreSQL => {
                new_row.insert("index_name".to_string(), row.get("index_name").cloned().unwrap_or(serde_json::Value::Null));
                new_row.insert("column_name".to_string(), row.get("column_name").cloned().unwrap_or(serde_json::Value::Null));
                new_row.insert("index_type".to_string(), row.get("index_type").cloned().unwrap_or(serde_json::Value::Null));
                let is_unique = row.get("is_unique").and_then(|v| v.as_bool()).unwrap_or(false);
                new_row.insert("non_unique".to_string(), serde_json::Value::Number(serde_json::Number::from(!is_unique as u8)));
            },
            DatabaseType::SQLite => {
                new_row.insert("index_name".to_string(), row.get("name").cloned().unwrap_or(serde_json::Value::Null));
                new_row.insert("non_unique".to_string(), serde_json::Value::Number(serde_json::Number::from(!row.get("unique").and_then(|v| v.as_bool()).unwrap_or(false) as u8)));
                new_row.insert("index_type".to_string(), serde_json::Value::String("B-TREE".to_string()));
                new_row.insert("column_name".to_string(), serde_json::Value::String("-".to_string())); // SQLite 需额外查询列
            },
            _ => {
                // MySQL 保持原样
                for (k, v) in row {
                    new_row.insert(k, v);
                }
            }
        }
        serde_json::Value::Object(new_row.into_iter().collect())
    }).collect();

    Ok(processed_rows)
}

/// 获取表外键
#[tauri::command]
pub async fn get_table_foreign_keys(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;

    let sql = match db_type {
        DatabaseType::MySQL => {
            format!(
                "SELECT CONSTRAINT_NAME as constraint_name, COLUMN_NAME as column_name,
                        REFERENCED_TABLE_NAME as referenced_table_name,
                        REFERENCED_COLUMN_NAME as referenced_column_name
                 FROM information_schema.KEY_COLUMN_USAGE 
                 WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}'
                       AND REFERENCED_TABLE_NAME IS NOT NULL
                 ORDER BY CONSTRAINT_NAME",
                database.replace("'", "''"),
                table.replace("'", "''")
            )
        },
        DatabaseType::PostgreSQL => {
            let schema_name = schema.unwrap_or_else(|| "public".to_string());
            format!(
                "SELECT
                    tc.constraint_name, 
                    kcu.column_name, 
                    ccu.table_name AS referenced_table_name,
                    ccu.column_name AS referenced_column_name 
                FROM 
                    information_schema.table_constraints AS tc 
                    JOIN information_schema.key_column_usage AS kcu
                      ON tc.constraint_name = kcu.constraint_name
                      AND tc.table_schema = kcu.table_schema
                    JOIN information_schema.constraint_column_usage AS ccu
                      ON ccu.constraint_name = tc.constraint_name
                      AND ccu.table_schema = tc.table_schema
                WHERE tc.constraint_type = 'FOREIGN KEY' 
                    AND tc.table_schema = '{}'
                    AND tc.table_name = '{}'",
                schema_name.replace("'", "''"),
                table.replace("'", "''")
            )
        },
        DatabaseType::SQLite => {
            format!("PRAGMA foreign_key_list('{}')", table.replace("'", "''"))
        },
        _ => return Ok(Vec::new()),
    };
    
    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    // 统一字段名以适配前端
    let processed_rows = result.rows.into_iter().map(|row| {
        let mut new_row = std::collections::HashMap::new();
        match db_type {
            DatabaseType::SQLite => {
                new_row.insert("constraint_name".to_string(), serde_json::Value::String(format!("fk_{}_{}", table, row.get("from").and_then(|v| v.as_str()).unwrap_or("col"))));
                new_row.insert("column_name".to_string(), row.get("from").cloned().unwrap_or(serde_json::Value::Null));
                new_row.insert("referenced_table_name".to_string(), row.get("table").cloned().unwrap_or(serde_json::Value::Null));
                new_row.insert("referenced_column_name".to_string(), row.get("to").cloned().unwrap_or(serde_json::Value::Null));
            },
            _ => {
                // MySQL 和 PostgreSQL 的字段名基本一致或已在 SQL 中 alias
                for (k, v) in row {
                    new_row.insert(k, v);
                }
            }
        }
        serde_json::Value::Object(new_row.into_iter().collect())
    }).collect();

    Ok(processed_rows)
}

/// 获取创建表的DDL语句
#[tauri::command]
pub async fn get_create_table_ddl(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = state.connection_manager.lock().await;
    
    println!("=== 获取 DDL: {}.{}.{:?} ===", database, table, schema);

    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    match db_type {
        DatabaseType::PostgreSQL => {
            let schema_name = schema.clone().unwrap_or_else(|| "public".to_string());
            
            // 1. 首先尝试检查是否为视图
            let view_sql = format!(
                "SELECT pg_get_viewdef(c.oid, true) as ddl 
                 FROM pg_class c 
                 JOIN pg_namespace n ON n.oid = c.relnamespace 
                 WHERE n.nspname = '{}' AND c.relname = '{}' AND c.relkind = 'v'",
                schema_name.replace("'", "''"), table.replace("'", "''")
            );
            
            if let Ok(result) = manager.execute_query(&connection_id, &view_sql, Some(&database)).await {
                if let Some(row) = result.rows.first() {
                    if let Some(serde_json::Value::String(definition)) = row.get("ddl") {
                        println!("识别为视图 DDL");
                        return Ok(format!("CREATE OR REPLACE VIEW {}.{} AS\n{}", schema_name, table, definition));
                    }
                }
            }

            // 2. 如果不是视图，尝试拼接表 DDL
            let table_sql = format!(
                "SELECT 'CREATE TABLE ' || quote_ident(table_schema) || '.' || quote_ident(table_name) || ' (\\n' || 
                string_agg('  ' || quote_ident(column_name) || ' ' || 
                (CASE 
                    WHEN data_type = 'character varying' THEN 'varchar' || (CASE WHEN character_maximum_length IS NOT NULL THEN '(' || character_maximum_length || ')' ELSE '' END)
                    WHEN data_type = 'character' THEN 'char' || (CASE WHEN character_maximum_length IS NOT NULL THEN '(' || character_maximum_length || ')' ELSE '' END)
                    WHEN data_type = 'numeric' THEN 'numeric' || (CASE WHEN numeric_precision IS NOT NULL THEN '(' || numeric_precision || ',' || numeric_scale || ')' ELSE '' END)
                    WHEN data_type = 'USER-DEFINED' THEN udt_name
                    ELSE data_type 
                END) || 
                (CASE WHEN is_nullable = 'NO' THEN ' NOT NULL' ELSE '' END) ||
                (CASE WHEN column_default IS NOT NULL THEN ' DEFAULT ' || column_default ELSE '' END), ',\\n') || 
                '\\n);' as ddl 
                FROM information_schema.columns 
                WHERE table_schema = '{}' AND table_name = '{}' 
                GROUP BY table_schema, table_name", 
                schema_name.replace("'", "''"), table.replace("'", "''")
            );

            match manager.execute_query(&connection_id, &table_sql, Some(&database)).await {
                Ok(result) => {
                    if let Some(row) = result.rows.first() {
                        if let Some(serde_json::Value::String(ddl)) = row.get("ddl") {
                            println!("生成表 DDL 成功");
                            return Ok(ddl.clone());
                        }
                    }
                    println!("查询返回空结果 (表不存在或无列)");
                    Err(format!("未找到表 {}.{} 的列信息", schema_name, table))
                },
                Err(e) => {
                    println!("查询 DDL 报错: {}", e);
                    Err(format!("获取 DDL 失败: {}", e))
                }
            }
        }
        DatabaseType::MySQL => {
            let sql = format!("SHOW CREATE TABLE `{}`.`{}`", database, table);
            let result = manager
                .execute_query(&connection_id, &sql, Some(&database))
                .await
                .map_err(|e| e.to_string())?;
            
            if let Some(row) = result.rows.first() {
                for (key, value) in row {
                    if key.to_lowercase().contains("create") {
                        if let serde_json::Value::String(ddl) = value {
                            return Ok(ddl.clone());
                        }
                    }
                }
            }
            Err("获取 MySQL DDL 失败".to_string())
        }
        DatabaseType::SQLite => {
            let sql = format!("SELECT sql as ddl FROM sqlite_master WHERE name = '{}'", table.replace("'", "''"));
            let result = manager
                .execute_query(&connection_id, &sql, Some(&database))
                .await
                .map_err(|e| e.to_string())?;
            
            if let Some(row) = result.rows.first() {
                if let Some(serde_json::Value::String(ddl)) = row.get("ddl") {
                    return Ok(ddl.clone());
                }
            }
            Err("获取 SQLite DDL 失败".to_string())
        }
        _ => Err("暂不支持该数据库类型的 DDL 获取".to_string())
    }
}

/// 自动补全数据结构
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AutoCompleteData {
    pub databases: Vec<String>,
    pub tables: Vec<TableSuggestion>,
    pub functions: Vec<FunctionSuggestion>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableSuggestion {
    pub name: String,
    pub schema: Option<String>,
    pub database: String,
    pub columns: Vec<ColumnSuggestion>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionSuggestion {
    pub name: String,
    pub schema: Option<String>,
    pub database: String,
    pub return_type: Option<String>,
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnSuggestion {
    pub name: String,
    pub data_type: String,
}

/// 获取自动补全数据
#[tauri::command]
pub async fn get_autocomplete_data(
    connection_id: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<AutoCompleteData, String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库列表
    let databases_info = manager
        .get_databases(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let databases: Vec<String> = databases_info.iter().map(|db| db.name.clone()).collect();
    
    // 获取表和列信息
    let mut tables = Vec::new();
    let mut functions = Vec::new();
    
    // 如果指定了数据库，只获取该数据库的表和函数
    // 否则获取所有数据库的信息
    let target_databases: Vec<String> = if let Some(db) = database {
        vec![db]
    } else {
        databases.clone()
    };
    
    for db_name in target_databases.iter() {
        // 获取该数据库的所有表
        let tables_info = manager
            .get_tables(&connection_id, Some(db_name))
            .await
            .unwrap_or_default();
        
        for table_info in tables_info {
            // 获取表的列信息
            let columns_info = manager
                .get_table_structure(&connection_id, &table_info.name, table_info.schema.as_deref(), Some(db_name))
                .await
                .unwrap_or_default();
            
            let columns: Vec<ColumnSuggestion> = columns_info
                .iter()
                .map(|col| ColumnSuggestion {
                    name: col.name.clone(),
                    data_type: col.data_type.clone(),
                })
                .collect();
            
            tables.push(TableSuggestion {
                name: table_info.name,
                schema: table_info.schema,
                database: db_name.clone(),
                columns,
            });
        }

        // 获取该数据库的所有函数
        let functions_info = manager
            .get_functions(&connection_id, Some(db_name), None)
            .await
            .unwrap_or_default();
            
        for func in functions_info {
            functions.push(FunctionSuggestion {
                name: func.name,
                schema: func.schema,
                database: db_name.clone(),
                return_type: func.return_type,
                arguments: func.arguments,
            });
        }
    }
    
    // SQL 关键字列表
    let keywords = vec![
        "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER", "DROP",
        "TABLE", "DATABASE", "INDEX", "VIEW", "PROCEDURE", "FUNCTION", "TRIGGER",
        "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "ON", "AS", "AND", "OR", "NOT",
        "IN", "BETWEEN", "LIKE", "IS", "NULL", "ORDER", "BY", "GROUP", "HAVING",
        "LIMIT", "OFFSET", "DISTINCT", "COUNT", "SUM", "AVG", "MAX", "MIN",
        "ASC", "DESC", "SET", "VALUES", "INTO", "DEFAULT", "PRIMARY", "KEY",
        "FOREIGN", "REFERENCES", "UNIQUE", "CHECK", "CONSTRAINT", "CASCADE",
        "AUTO_INCREMENT", "UNSIGNED", "ZEROFILL", "BINARY", "COLLATE", "CHARSET",
        "ENGINE", "COMMENT", "IF", "EXISTS", "TEMPORARY", "TRUNCATE",
        "RENAME", "MODIFY", "CHANGE", "ADD", "COLUMN", "AFTER", "FIRST",
        "UNION", "ALL", "CASE", "WHEN", "THEN", "ELSE", "END",
        "CAST", "CONVERT", "SUBSTRING", "CONCAT", "LENGTH", "TRIM",
        "UPPER", "LOWER", "REPLACE", "DATE", "TIME", "TIMESTAMP", "NOW",
        "YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND",
        "INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "DECIMAL", "NUMERIC",
        "FLOAT", "DOUBLE", "REAL", "VARCHAR", "CHAR", "TEXT", "BLOB",
        "DATE", "DATETIME", "TIMESTAMP", "TIME", "YEAR", "BOOLEAN", "BOOL",
        "GRANT", "REVOKE", "COMMIT", "ROLLBACK", "SAVEPOINT", "START", "TRANSACTION",
        "BEGIN", "USE", "SHOW", "DESCRIBE", "DESC", "EXPLAIN",
    ].iter().map(|s| s.to_string()).collect();
    
    Ok(AutoCompleteData {
        databases,
        tables,
        functions,
        keywords,
    })
}

/// 获取 schema 列表
#[tauri::command]
pub async fn get_schemas(
    connection_id: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SchemaInfo>, String> {
    let manager = state.connection_manager.lock().await;

    manager
        .get_schemas(&connection_id, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 获取指定 schema 下的表列表
#[tauri::command]
pub async fn get_schema_tables(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String> {
    let manager = state.connection_manager.lock().await;

    // 获取所有表，然后过滤指定 schema 的表
    let all_tables = manager
        .get_tables(&connection_id, Some(&database))
        .await
        .map_err(|e| e.to_string())?;

    Ok(all_tables
        .into_iter()
        .filter(|t| t.schema.as_deref() == Some(&schema))
        .collect())
}

/// 获取指定 schema 下的视图列表
#[tauri::command]
pub async fn get_schema_views(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String> {
    let manager = state.connection_manager.lock().await;

    // 获取所有视图，然后过滤指定 schema 的视图
    let all_views = manager
        .get_views(&connection_id, Some(&database))
        .await
        .map_err(|e| e.to_string())?;

    Ok(all_views
        .into_iter()
        .filter(|v| v.schema.as_deref() == Some(&schema))
        .collect())
}

/// 获取指定 schema 下的函数列表
#[tauri::command]
pub async fn get_schema_functions(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<FunctionInfo>, String> {
    let manager = state.connection_manager.lock().await;

    manager
        .get_functions(&connection_id, Some(&database), Some(&schema))
        .await
        .map_err(|e| e.to_string())
}

/// 获取指定 schema 下的聚合函数列表
#[tauri::command]
pub async fn get_schema_aggregate_functions(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<FunctionInfo>, String> {
    let manager = state.connection_manager.lock().await;

    manager
        .get_aggregate_functions(&connection_id, Some(&database), Some(&schema))
        .await
        .map_err(|e| e.to_string())
}

/// 获取指定 schema 下的索引列表
#[tauri::command]
pub async fn get_schema_indexes(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;

    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;

    let sql = match db_type {
        DatabaseType::PostgreSQL => {
            format!(
                "SELECT
                    i.relname as index_name,
                    t.relname as table_name,
                    a.attname as column_name,
                    ix.indisunique as is_unique,
                    ix.indisprimary as is_primary,
                    am.amname as index_type
                 FROM pg_class t
                 JOIN pg_index ix ON t.oid = ix.indrelid
                 JOIN pg_class i ON i.oid = ix.indexrelid
                 JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
                 JOIN pg_am am ON i.relam = am.oid
                 JOIN pg_namespace n ON n.oid = t.relnamespace
                 WHERE n.nspname = '{}'
                 ORDER BY i.relname, a.attnum",
                schema.replace("'", "''")
            )
        }
        _ => {
            return Err("该数据库类型不支持按 schema 获取索引".to_string());
        }
    };

    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}
/// 获取数据库的扩展列表 (PostgreSQL)
#[tauri::command]
pub async fn get_database_extensions(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let manager = state.connection_manager.lock().await;

    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;

    let sql = match db_type {
        DatabaseType::PostgreSQL => {
            "SELECT
                e.extname as name,
                e.extversion as version,
                n.nspname as schema,
                obj_description(e.oid, 'pg_extension') as comment
             FROM pg_extension e
             JOIN pg_namespace n ON n.oid = e.extnamespace
             ORDER BY e.extname".to_string()
        }
        _ => {
            return Err("该数据库类型不支持扩展".to_string());
        }
    };

    let result = manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;

    Ok(result.rows.into_iter().map(|row| serde_json::Value::Object(
        row.into_iter().collect()
    )).collect())
}
