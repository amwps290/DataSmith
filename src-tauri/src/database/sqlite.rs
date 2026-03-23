use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use rusqlite::{Connection, params};
use tokio::sync::Mutex;
use tracing::{debug, instrument};
use std::fs::File;
use std::path::Path;

use super::traits::*;

/// SQLite 数据库驱动状态
struct SqliteState {
    conn: Option<Connection>,
    path: Option<String>,
}

/// SQLite 数据库驱动 - 基于 rusqlite 的原生实现
pub struct SqliteDatabase {
    state: Mutex<SqliteState>,
}

impl SqliteDatabase {
    pub fn new() -> Self {
        Self { 
            state: Mutex::new(SqliteState { conn: None, path: None })
        }
    }

    /// 创建新的 SQLite 数据库文件 (由命令层调用)
    pub fn create_database_file(path: &str) -> DbResult<()> {
        if !Path::new(path).exists() {
            File::create(path).map_err(|e| DbError::Other(format!("无法创建文件: {}", e)))?;
        }
        Ok(())
    }
}

#[async_trait]
impl DatabaseOperations for SqliteDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let _conn = Connection::open(&config.host).map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        Ok(true)
    }

    async fn connect(&self, config: ConnectionConfig) -> DbResult<()> {
        let conn = Connection::open(&config.host).map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        let mut state = self.state.lock().await;
        state.conn = Some(conn);
        state.path = Some(config.host);
        Ok(())
    }

    async fn disconnect(&self) -> DbResult<()> {
        let mut state = self.state.lock().await;
        state.conn = None;
        Ok(())
    }

    #[instrument(skip(self, sql))]
    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let start = Instant::now();
        let state = self.state.lock().await;
        let conn = state.conn.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;

        // 简化的多语句支持逻辑：按分号初步分割 (复杂 SQL 可能需要解析器)
        let sqls: Vec<&str> = sql.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let mut results = Vec::new();

        for s in sqls {
            let mut stmt = conn.prepare(s).map_err(|e| DbError::QueryFailed(e.to_string()))?;
            let column_count = stmt.column_count();
            let column_names: Vec<String> = stmt.column_names().into_iter().map(|n| n.to_string()).collect();

            let mut rows = Vec::new();
            if column_count > 0 {
                let mut query_rows = stmt.query(params![]).map_err(|e| DbError::QueryFailed(e.to_string()))?;
                while let Some(row) = query_rows.next().map_err(|e| DbError::QueryFailed(e.to_string()))? {
                    let mut row_map = HashMap::new();
                    for i in 0..column_count {
                        let value: serde_json::Value = match row.get_ref(i).unwrap() {
                            rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                            rusqlite::types::ValueRef::Integer(i) => serde_json::Value::Number(i.into()),
                            rusqlite::types::ValueRef::Real(f) => serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap()),
                            rusqlite::types::ValueRef::Text(t) => serde_json::Value::String(String::from_utf8_lossy(t).into_owned()),
                            rusqlite::types::ValueRef::Blob(b) => serde_json::Value::String(format!("BLOB ({} bytes)", b.len())),
                        };
                        row_map.insert(column_names[i].clone(), value);
                    }
                    rows.push(row_map);
                }
            }

            results.push(QueryResult {
                columns: column_names,
                rows,
                affected_rows: conn.changes() as u64,
                execution_time_ms: start.elapsed().as_millis(),
            });
        }

        if results.is_empty() {
            results.push(QueryResult { columns: vec![], rows: vec![], affected_rows: 0, execution_time_ms: 0 });
        }

        Ok(results)
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        Ok(vec![DatabaseInfo { name: "main".into(), charset: None, collation: None }])
    }

    async fn get_tables(&self, _database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let state = self.state.lock().await;
        let conn = state.conn.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'").map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let rows = stmt.query_map([], |row| Ok(TableInfo { name: row.get(0)?, schema: None, table_type: "TABLE".into(), engine: None, rows: None, size_mb: None, comment: None })).map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.map(|r| r.unwrap()).collect())
    }

    async fn get_table_structure(&self, table: &str, _schema: Option<&str>, _database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let state = self.state.lock().await;
        let conn = state.conn.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let mut stmt = conn.prepare(&format!("PRAGMA table_info('{}')", table.replace("'", "''"))).map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let rows = stmt.query_map([], |row| {
            let pk: i64 = row.get(5)?;
            Ok(ColumnInfo {
                name: row.get(1)?, data_type: row.get(2)?, nullable: row.get::<usize, i64>(3)? == 0,
                default_value: row.get(4).ok(), is_primary_key: pk > 0,
                is_auto_increment: false, comment: None, character_maximum_length: None, numeric_precision: None, numeric_scale: None,
            })
        }).map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.map(|r| r.unwrap()).collect())
    }

    async fn update_data(&self, table: &str, _schema: Option<&str>, column: &str, value: Option<String>, where_conditions: HashMap<String, serde_json::Value>) -> DbResult<()> {
        let state = self.state.lock().await;
        let conn = state.conn.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;

        let mut where_parts = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        
        params.push(Box::new(value));

        for (col, val) in where_conditions {
            if val.is_null() {
                where_parts.push(format!("\"{}\" IS NULL", col));
            } else {
                where_parts.push(format!("\"{}\" = ?", col));
                match val {
                    serde_json::Value::String(s) => params.push(Box::new(s)),
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() { params.push(Box::new(i)) }
                        else { params.push(Box::new(n.as_f64().unwrap_or(0.0))) }
                    },
                    serde_json::Value::Bool(b) => params.push(Box::new(b)),
                    _ => params.push(Box::new(val.to_string())),
                }
            }
        }

        let sql = format!("UPDATE \"{}\" SET \"{}\" = ? WHERE {}", table, column, where_parts.join(" AND "));
        debug!(sql = %sql, "执行 SQLite 参数化更新");
        
        let p_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, p_refs.as_slice()).map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    async fn delete_data(&self, table: &str, _schema: Option<&str>, where_conditions: HashMap<String, serde_json::Value>) -> DbResult<()> {
        let state = self.state.lock().await;
        let conn = state.conn.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;

        let mut where_parts = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        for (col, val) in where_conditions {
            if val.is_null() {
                where_parts.push(format!("\"{}\" IS NULL", col));
            } else {
                where_parts.push(format!("\"{}\" = ?", col));
                match val {
                    serde_json::Value::String(s) => params.push(Box::new(s)),
                    _ => params.push(Box::new(val.to_string())),
                }
            }
        }

        let sql = format!("DELETE FROM \"{}\" WHERE {}", table, where_parts.join(" AND "));
        debug!(sql = %sql, "执行 SQLite 参数化删除");
        
        let p_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, p_refs.as_slice()).map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    async fn get_indexes(&self, table: &str, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let res_vec = self.execute_query(&format!("PRAGMA index_list('{}')", table.replace("'", "''")), None).await?;
        let mut indexes = Vec::new();
        if let Some(res) = res_vec.first() {
            for r in &res.rows {
                let name = r.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let col_res_vec = self.execute_query(&format!("PRAGMA index_info('{}')", name.replace("'", "''")), None).await?;
                if let Some(col_res) = col_res_vec.first() {
                    let columns = col_res.rows.iter().map(|cr| cr.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string()).collect();
                    indexes.push(IndexInfo { 
                        name, columns, 
                        is_unique: r.get("unique").and_then(|v| v.as_i64()).map(|n| n > 0).or_else(|| r.get("unique").and_then(|v| v.as_str()).map(|s| s == "1")).unwrap_or(false), 
                        is_primary: false, index_type: "BTREE".to_string() 
                    });
                }
            }
        }
        Ok(indexes)
    }

    async fn get_foreign_keys(&self, table: &str, _schema: Option<&str>) -> DbResult<Vec<ForeignKeyInfo>> {
        let sql = format!("PRAGMA foreign_key_list('{}')", table.replace("'", "''"));
        let results = self.execute_query(&sql, None).await?;
        let mut fks = Vec::new();
        if let Some(res) = results.first() {
            for r in &res.rows {
                fks.push(ForeignKeyInfo {
                    name: format!("fk_{}_{}", table, r.get("from").and_then(|v| v.as_str()).unwrap_or("")),
                    column_name: r.get("from").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    referenced_table_name: r.get("table").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    referenced_column_name: r.get("to").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    update_rule: r.get("on_update").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    delete_rule: r.get("on_delete").and_then(|v| v.as_str()).map(|s| s.to_string()),
                });
            }
        }
        Ok(fks)
    }

    async fn alter_table(&self, table: &str, _schema: Option<&str>, _database: Option<&str>, changes: Vec<TableChange>) -> DbResult<()> {
        let state = self.state.lock().await;
        let conn = state.conn.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;

        for change in changes {
            match change {
                TableChange::AddColumn(col) => {
                    let mut sql = format!("ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}", table, col.name, col.data_type);
                    if !col.nullable { sql.push_str(" NOT NULL"); }
                    if let Some(ref d) = col.default_value { sql.push_str(&format!(" DEFAULT {}", d)); }
                    
                    debug!(sql = %sql, "执行 SQLite ALTER TABLE ADD COLUMN");
                    conn.execute(&sql, []).map_err(|e| DbError::QueryFailed(e.to_string()))?;
                },
                _ => return Err(DbError::Other("SQLite 暂仅支持添加列操作。修改/删除列需要重构表，暂未实现。".into())),
            }
        }
        
        Ok(())
    }

    async fn get_table_ddl(&self, table: &str, _schema: Option<&str>) -> DbResult<String> {
        let sql = format!("SELECT sql FROM sqlite_master WHERE name = '{}'", table.replace("'", "''"));
        let results = self.execute_query(&sql, None).await?;
        if let Some(res) = results.first() {
            if let Some(row) = res.rows.first() {
                return Ok(row.get("sql").and_then(|v| v.as_str()).unwrap_or("").to_string());
            }
        }
        Err(DbError::Other("无法获取 DDL".into()))
    }

    async fn explain_query(&self, sql: &str, database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let explain_sql = format!("EXPLAIN QUERY PLAN {}", sql);
        self.execute_query(&explain_sql, database).await
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
