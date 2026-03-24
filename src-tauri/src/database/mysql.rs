use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use mysql_async::{prelude::*, Pool, Opts, Row, Value, OptsBuilder};
use tokio::sync::Mutex;
use tracing::{info, instrument, debug, error};

use super::traits::*;
use super::sql_helpers::{build_where_clause, ParamStyle};
use crate::utils::sql_sanitize::{escape_mysql_id, escape_string_literal};

/// MySQL 数据库驱动状态
struct MySqlState {
    pool: Option<Pool>,
    config: Option<ConnectionConfig>,
}

/// MySQL 数据库驱动 - 基于 mysql_async 的原生实现
pub struct MySqlDatabase {
    state: Mutex<MySqlState>,
}

impl MySqlDatabase {
    pub fn new() -> Self {
        Self { 
            state: Mutex::new(MySqlState { pool: None, config: None })
        }
    }

    fn create_opts(config: &ConnectionConfig) -> Opts {
        let builder = OptsBuilder::default()
            .ip_or_hostname(config.host.clone())
            .tcp_port(config.port)
            .user(Some(config.username.clone()))
            .pass(Some(config.password.clone()))
            .db_name(config.database.as_deref())
            .prefer_socket(false)
            .tcp_keepalive(Some(60000u32));
        
        builder.into()
    }
}

#[async_trait]
impl DatabaseOperations for MySqlDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let opts = Self::create_opts(config);
        let pool = Pool::new(opts);
        match pool.get_conn().await {
            Ok(mut conn) => {
                conn.query_drop("SELECT 1").await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
                pool.disconnect().await.ok();
                Ok(true)
            },
            Err(e) => {
                error!("MySQL 连接测试失败: {}", e);
                Err(DbError::ConnectionFailed(e.to_string()))
            }
        }
    }

    async fn connect(&self, config: ConnectionConfig) -> DbResult<()> {
        let opts = Self::create_opts(&config);
        let pool = Pool::new(opts);
        
        // 尝试获取一个连接以验证配置是否真的可用
        pool.get_conn().await.map_err(|e| DbError::ConnectionFailed(format!("无法建立初始连接: {}", e)))?;
        
        let mut state = self.state.lock().await;
        state.pool = Some(pool);
        state.config = Some(config);
        Ok(())
    }

    async fn disconnect(&self) -> DbResult<()> {
        let mut state = self.state.lock().await;
        if let Some(pool) = state.pool.take() {
            pool.disconnect().await.map_err(|e| DbError::Other(e.to_string()))?;
        }
        state.config = None;
        Ok(())
    }

    async fn switch_database(&self, database: &str) -> DbResult<()> {
        let mut state = self.state.lock().await;
        let mut config = state.config.clone().ok_or(DbError::Other("未找到初始配置".into()))?;
        
        if config.database.as_deref() == Some(database) {
            return Ok(());
        }
        
        info!(new_db = %database, "MySQL 正在重连以切换数据库...");
        config.database = Some(database.to_string());
        
        let opts = Self::create_opts(&config);
        let pool = Pool::new(opts);
        
        if let Some(old_pool) = state.pool.replace(pool) {
            tokio::spawn(async move { old_pool.disconnect().await.ok(); });
        }
        state.config = Some(config);
        Ok(())
    }

    #[instrument(skip(self, sql))]
    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let _start_total = Instant::now();
        // 克隆 Pool 后立即释放锁（mysql_async::Pool 内部是 Arc，clone 廉价）
        let pool = {
            let state = self.state.lock().await;
            state.pool.as_ref().ok_or(DbError::not_connected())?.clone()
        };
        let mut conn = pool.get_conn().await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;

        debug!(sql = %sql.replace('\n', " "), "执行 MySQL 查询");

        // 手动拆分 SQL 语句
        let sqls: Vec<&str> = sql.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let mut results = Vec::new();

        for s in sqls {
            let start_stmt = Instant::now();
            let rows: Vec<Row> = conn.query(s).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
            
            let mut columns = Vec::new();
            if let Some(first_row) = rows.first() {
                columns = first_row.columns().iter().map(|c| c.name_str().to_string()).collect();
            }
            
            let mut final_rows = Vec::new();
            for row in rows {
                let mut row_map = HashMap::new();
                for (i, col_name) in columns.iter().enumerate() {
                    let value: Value = row.get(i).unwrap_or(Value::NULL);
                    let json_val = match value {
                        Value::NULL => serde_json::Value::Null,
                        Value::Bytes(b) => serde_json::Value::String(String::from_utf8_lossy(&b).into_owned()),
                        Value::Int(i) => serde_json::Value::Number(i.into()),
                        Value::UInt(u) => serde_json::Value::Number(u.into()),
                        Value::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(f as f64).unwrap_or(serde_json::Number::from(0))),
                        Value::Double(d) => serde_json::Value::Number(serde_json::Number::from_f64(d).unwrap_or(serde_json::Number::from(0))),
                        Value::Date(y, m, d, h, i, s, ms) => serde_json::Value::String(format!("{}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}", y, m, d, h, i, s, ms)),
                        Value::Time(neg, d, h, m, s, ms) => serde_json::Value::String(format!("{}{}:{:02}:{:02}:{:02}.{:03}", if neg { "-" } else { "" }, d, h, m, s, ms)),
                    };
                    row_map.insert(col_name.clone(), json_val);
                }
                final_rows.push(row_map);
            }
            
            results.push(QueryResult {
                columns,
                rows: final_rows,
                affected_rows: conn.affected_rows(),
                execution_time_ms: start_stmt.elapsed().as_millis(),
            });
        }

        if results.is_empty() {
            results.push(QueryResult::empty(0));
        }

        Ok(results)
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let results = self.execute_query("SHOW DATABASES", None).await?;
        if let Some(res) = results.first() {
            Ok(res.rows.iter().map(|r| DatabaseInfo { 
                name: r.values().next().and_then(|v| v.as_str()).unwrap_or("").to_string(), 
                charset: None, collation: None 
            }).collect())
        } else {
            Ok(vec![])
        }
    }

    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let sql = if let Some(db) = database {
            format!("SELECT TABLE_NAME, TABLE_COMMENT, TABLE_ROWS, DATA_LENGTH FROM information_schema.TABLES WHERE TABLE_SCHEMA = {}", escape_string_literal(db))
        } else {
            "SHOW TABLES".to_string()
        };
        
        let results = self.execute_query(&sql, None).await?;
        if let Some(res) = results.first() {
            Ok(res.rows.iter().map(|r| TableInfo {
                name: r.get("TABLE_NAME").or_else(|| r.values().next()).and_then(|v| v.as_str()).unwrap_or("").to_string(),
                schema: None,
                table_type: "TABLE".into(),
                engine: None,
                rows: r.get("TABLE_ROWS").and_then(|v| v.as_u64()),
                size_mb: r.get("DATA_LENGTH").and_then(|v| v.as_f64()).map(|s| s / 1024.0 / 1024.0),
                comment: r.get("TABLE_COMMENT").and_then(|v| v.as_str()).map(|s| s.to_string()),
            }).collect())
        } else {
            Ok(vec![])
        }
    }

    async fn get_table_structure(&self, table: &str, _schema: Option<&str>, database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let schema = database.unwrap_or("");
        let sql = format!("SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT, COLUMN_KEY, EXTRA, COLUMN_COMMENT FROM information_schema.COLUMNS WHERE TABLE_NAME = {} AND TABLE_SCHEMA = {} ORDER BY ORDINAL_POSITION", escape_string_literal(table), escape_string_literal(schema));
        
        let results = self.execute_query(&sql, None).await?;
        if let Some(res) = results.first() {
            Ok(res.rows.iter().map(|r| {
                let key = r.get("COLUMN_KEY").and_then(|v| v.as_str()).unwrap_or("");
                ColumnInfo {
                    name: r.get("COLUMN_NAME").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    data_type: r.get("DATA_TYPE").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    nullable: r.get("IS_NULLABLE").and_then(|v| v.as_str()) == Some("YES"),
                    default_value: r.get("COLUMN_DEFAULT").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    is_primary_key: key == "PRI",
                    is_auto_increment: r.get("EXTRA").and_then(|v| v.as_str()).unwrap_or("").contains("auto_increment"),
                    comment: r.get("COLUMN_COMMENT").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    character_maximum_length: None,
                    numeric_precision: None,
                    numeric_scale: None,
                }
            }).collect())
        } else {
            Ok(vec![])
        }
    }

    async fn update_data(&self, table: &str, _schema: Option<&str>, column: &str, value: Option<String>, where_conditions: HashMap<String, serde_json::Value>) -> DbResult<()> {
        let pool = {
            let state = self.state.lock().await;
            state.pool.as_ref().ok_or(DbError::not_connected())?.clone()
        };
        let mut conn = pool.get_conn().await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;

        let wc = build_where_clause(&where_conditions, escape_mysql_id, ParamStyle::QuestionMark);

        let mut p_vec = Vec::new();
        p_vec.push(mysql_async::Value::from(value));
        for pv in &wc.param_values {
            p_vec.push(mysql_async::Value::from(pv.clone()));
        }

        let params = mysql_async::Params::Positional(p_vec);
        let sql = format!("UPDATE {} SET {} = ? WHERE {}", escape_mysql_id(table), escape_mysql_id(column), wc.sql);

        conn.exec_drop(sql, params).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    async fn delete_data(&self, table: &str, _schema: Option<&str>, where_conditions: HashMap<String, serde_json::Value>) -> DbResult<()> {
        let pool = {
            let state = self.state.lock().await;
            state.pool.as_ref().ok_or(DbError::not_connected())?.clone()
        };
        let mut conn = pool.get_conn().await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;

        let wc = build_where_clause(&where_conditions, escape_mysql_id, ParamStyle::QuestionMark);

        let mut p_vec = Vec::new();
        for pv in &wc.param_values {
            p_vec.push(mysql_async::Value::from(pv.clone()));
        }

        let sql = format!("DELETE FROM {} WHERE {}", escape_mysql_id(table), wc.sql);
        conn.exec_drop(sql, mysql_async::Params::Positional(p_vec)).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    async fn get_indexes(&self, table: &str, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let results = self.execute_query(&format!("SHOW INDEX FROM {}", escape_mysql_id(table)), None).await?;
        let mut map: HashMap<String, IndexInfo> = HashMap::new();
        
        if let Some(res) = results.first() {
            for r in &res.rows {
                let name = r.get("Key_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let col = r.get("Column_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let non_unique = r.get("Non_unique").and_then(|v| v.as_i64()).unwrap_or(1);
                
                let entry = map.entry(name.clone()).or_insert(IndexInfo {
                    name,
                    columns: vec![],
                    is_unique: non_unique == 0,
                    is_primary: r.get("Key_name").and_then(|v| v.as_str()) == Some("PRIMARY"),
                    index_type: r.get("Index_type").and_then(|v| v.as_str()).unwrap_or("BTREE").to_string(),
                });
                entry.columns.push(col);
            }
        }
        Ok(map.into_values().collect())
    }

    async fn get_foreign_keys(&self, table: &str, _schema: Option<&str>) -> DbResult<Vec<ForeignKeyInfo>> {
        let sql = format!(
            "SELECT kcu.CONSTRAINT_NAME, kcu.COLUMN_NAME, kcu.REFERENCED_TABLE_NAME, kcu.REFERENCED_COLUMN_NAME, rc.UPDATE_RULE, rc.DELETE_RULE FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE kcu JOIN INFORMATION_SCHEMA.REFERENTIAL_CONSTRAINTS rc ON kcu.CONSTRAINT_NAME = rc.CONSTRAINT_NAME AND kcu.CONSTRAINT_SCHEMA = rc.CONSTRAINT_SCHEMA WHERE kcu.TABLE_NAME = {} AND kcu.REFERENCED_TABLE_NAME IS NOT NULL",
            escape_string_literal(table)
        );
        let results = self.execute_query(&sql, None).await?;
        let mut fks = Vec::new();
        if let Some(res) = results.first() {
            for r in &res.rows {
                fks.push(ForeignKeyInfo {
                    name: r.get("CONSTRAINT_NAME").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    column_name: r.get("COLUMN_NAME").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    referenced_table_name: r.get("REFERENCED_TABLE_NAME").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    referenced_column_name: r.get("REFERENCED_COLUMN_NAME").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    update_rule: r.get("UPDATE_RULE").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    delete_rule: r.get("DELETE_RULE").and_then(|v| v.as_str()).map(|s| s.to_string()),
                });
            }
        }
        Ok(fks)
    }

    async fn alter_table(&self, table: &str, _schema: Option<&str>, database: Option<&str>, changes: Vec<TableChange>) -> DbResult<()> {
        let pool = {
            let state = self.state.lock().await;
            state.pool.as_ref().ok_or(DbError::not_connected())?.clone()
        };
        let mut conn = pool.get_conn().await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        let db_name = database.unwrap_or("");

        let mut sql_parts = Vec::new();
        for change in changes {
            match change {
                TableChange::AddColumn(col) => {
                    let mut part = format!("ADD COLUMN {} {}", escape_mysql_id(&col.name), col.data_type);
                    if let Some(l) = col.character_maximum_length { part.push_str(&format!("({})", l)); }
                    if !col.nullable { part.push_str(" NOT NULL"); }
                    if let Some(ref d) = col.default_value { part.push_str(&format!(" DEFAULT '{}'", d.replace("'", "''"))); }
                    if let Some(ref c) = col.comment { part.push_str(&format!(" COMMENT '{}'", c.replace("'", "''"))); }
                    sql_parts.push(part);
                },
                TableChange::ModifyColumn { old_name, new_column } => {
                    let mut part = if old_name != new_column.name {
                        format!("CHANGE COLUMN {} {} {}", escape_mysql_id(&old_name), escape_mysql_id(&new_column.name), new_column.data_type)
                    } else {
                        format!("MODIFY COLUMN {} {}", escape_mysql_id(&new_column.name), new_column.data_type)
                    };
                    if let Some(l) = new_column.character_maximum_length { part.push_str(&format!("({})", l)); }
                    if !new_column.nullable { part.push_str(" NOT NULL"); }
                    if let Some(ref d) = new_column.default_value { part.push_str(&format!(" DEFAULT '{}'", d.replace("'", "''"))); }
                    if let Some(ref c) = new_column.comment { part.push_str(&format!(" COMMENT '{}'", c.replace("'", "''"))); }
                    sql_parts.push(part);
                },
                TableChange::DropColumn(name) => {
                    sql_parts.push(format!("DROP COLUMN {}", escape_mysql_id(&name)));
                },
                TableChange::AddIndex(idx) => {
                    let cols = idx.columns.iter().map(|c| escape_mysql_id(c)).collect::<Vec<_>>().join(", ");
                    let unique = if idx.is_unique { "UNIQUE " } else { "" };
                    sql_parts.push(format!("ADD {}INDEX {} ({})", unique, escape_mysql_id(&idx.name), cols));
                },
                TableChange::DropIndex(name) => {
                    sql_parts.push(format!("DROP INDEX {}", escape_mysql_id(&name)));
                },
                TableChange::AddForeignKey(fk) => {
                    sql_parts.push(format!(
                        "ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}.{} ({}) ON UPDATE {} ON DELETE {}",
                        escape_mysql_id(&fk.name), escape_mysql_id(&fk.column_name), escape_mysql_id(db_name), escape_mysql_id(&fk.referenced_table_name), escape_mysql_id(&fk.referenced_column_name),
                        fk.update_rule.as_deref().unwrap_or("NO ACTION"),
                        fk.delete_rule.as_deref().unwrap_or("NO ACTION")
                    ));
                },
                TableChange::DropForeignKey(name) => {
                    sql_parts.push(format!("DROP FOREIGN KEY {}", escape_mysql_id(&name)));
                },
            }
        }

        if !sql_parts.is_empty() {
            let sql = format!("ALTER TABLE {}.{} {}", escape_mysql_id(db_name), escape_mysql_id(table), sql_parts.join(", "));
            debug!(sql = %sql, "执行 MySQL ALTER TABLE");
            conn.query_drop(&sql).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        }
        
        Ok(())
    }

    async fn get_table_ddl(&self, table: &str, _schema: Option<&str>) -> DbResult<String> {
        let sql = format!("SHOW CREATE TABLE {}", escape_mysql_id(table));
        let results = self.execute_query(&sql, None).await?;
        if let Some(res) = results.first() {
            if let Some(row) = res.rows.first() {
                return Ok(row.get("Create Table").or_else(|| row.values().nth(1)).and_then(|v| v.as_str()).unwrap_or("").to_string());
            }
        }
        Err(DbError::Other("无法获取 DDL".into()))
    }

    async fn explain_query(&self, sql: &str, database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let explain_sql = format!("EXPLAIN {}", sql);
        self.execute_query(&explain_sql, database).await
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
