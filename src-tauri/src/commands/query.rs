use crate::database::QueryResult;
use crate::utils::sql_formatter::SqlFormatter;
use crate::AppState;
use tauri::State;

/// 执行 SQL 查询
#[tauri::command]
pub async fn execute_query(
    connection_id: String,
    sql: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    manager
        .execute_query(&connection_id, &sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 分页执行 SQL 查询
#[tauri::command]
pub async fn execute_query_paged(
    connection_id: String,
    sql: String,
    database: Option<String>,
    page: u32,
    page_size: u32,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库类型以确定分页语法
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
        
    let mut final_sql = sql.trim().to_string();
    
    // 检查是否是 SELECT 语句，且没有已经包含 LIMIT
    // 使用正则匹配，忽略前面的空格和注释
    let is_select = regex::Regex::new(r"(?i)^(?:\s*|--.*?\n|/\*.*?\*/)*SELECT").unwrap().is_match(&final_sql);
    let upper_sql = final_sql.to_uppercase();
    let has_limit = upper_sql.contains(" LIMIT ") || upper_sql.contains("\nLIMIT ");
    
    if is_select && !has_limit {
        let offset = (page - 1) * page_size;
        
        match db_type {
            crate::database::DatabaseType::MySQL | 
            crate::database::DatabaseType::PostgreSQL | 
            crate::database::DatabaseType::SQLite => {
                // 确保语句最后没有分号
                while final_sql.ends_with(';') || final_sql.ends_with('\n') || final_sql.ends_with('\r') || final_sql.ends_with(' ') {
                    final_sql.pop();
                }
                final_sql = format!("{} LIMIT {} OFFSET {}", final_sql, page_size, offset);
            },
            _ => {} // 其他数据库暂不支持自动分页改写
        }
    }
    
    manager
        .execute_query(&connection_id, &final_sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 批量执行 SQL 查询
#[tauri::command]
pub async fn execute_query_batch(
    connection_id: String,
    sqls: Vec<String>,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<QueryResult>, String> {
    let mut results = Vec::new();
    
    for sql in sqls {
        let result = execute_query(connection_id.clone(), sql, database.clone(), state.clone()).await?;
        results.push(result);
    }
    
    Ok(results)
}

/// 更新表数据
#[tauri::command]
pub async fn update_table_data(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    column: String,
    value: Option<String>,
    where_clause: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库类型
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // 使用 SqlFormatter 构建 UPDATE SQL 语句
    let sql = SqlFormatter::format_update(
        &db_type,
        &database,
        &table,
        schema.as_deref(),
        &column,
        value.as_deref(),
        &where_clause,
    );
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

/// 插入表数据
#[tauri::command]
pub async fn insert_table_data(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    data: std::collections::HashMap<String, Option<String>>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库类型
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let columns: Vec<String> = data.keys().cloned().collect();
    let values: Vec<String> = data.values().map(|v| {
        if let Some(val) = v {
            format!("'{}'", val.replace("'", "''"))
        } else {
            "NULL".to_string()
        }
    }).collect();
    
    // 使用 SqlFormatter 构建 INSERT SQL 语句
    let sql = SqlFormatter::format_insert(
        &db_type,
        &database,
        &table,
        schema.as_deref(),
        &columns,
        &values,
    );
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

/// 删除表数据
#[tauri::command]
pub async fn delete_table_data(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    where_clause: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = state.connection_manager.lock().await;
    
    // 获取数据库类型
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // 使用 SqlFormatter 构建 DELETE SQL 语句
    let sql = SqlFormatter::format_delete(
        &db_type,
        &database,
        &table,
        schema.as_deref(),
        &where_clause,
    );
    
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(())
}

