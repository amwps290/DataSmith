use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use std::sync::Mutex;
use rusqlite::{Connection, types::ValueRef};
use tracing::{debug, info, instrument};

use super::traits::*;

/// SQLite 数据库驱动 - 基于 rusqlite 的底层实现 (线程安全包装)
pub struct SqliteDatabase {
    connection: Option<Mutex<Connection>>,
    path: Option<String>,
}

impl SqliteDatabase {
    pub fn new() -> Self {
        Self {
            connection: None,
            path: None,
        }
    }

    /// 创建新的 SQLite 数据库文件
    pub fn create_database_file(path: &str) -> DbResult<()> {
        if Path::new(path).exists() {
            return Err(DbError::Other("文件已存在".to_string()));
        }
        
        Connection::open(path).map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        info!(path = %path, "成功创建 SQLite 数据库文件");
        Ok(())
    }

    /// 将 rusqlite 的行转换为 JSON
    fn row_to_map(row: &rusqlite::Row) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        let column_names = row.as_ref().column_names();
        
        for (i, name) in column_names.iter().enumerate() {
            let value = match row.get_ref_unwrap(i) {
                ValueRef::Null => serde_json::Value::Null,
                ValueRef::Integer(i) => serde_json::Value::Number(i.into()),
                ValueRef::Real(f) => serde_json::Number::from_f64(f).map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null),
                ValueRef::Text(s) => {
                    let s_str = std::str::from_utf8(s).unwrap_or("");
                    serde_json::Value::String(s_str.to_string())
                },
                ValueRef::Blob(b) => {
                    let hex = b.iter().map(|byte| format!("{:02x}", byte)).collect::<String>();
                    serde_json::Value::String(format!("\\x{}", hex))
                }
            };
            map.insert(name.to_string(), value);
        }
        map
    }
}

#[async_trait]
impl DatabaseOperations for SqliteDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let path = &config.host;
        match Connection::open(path) {
            Ok(conn) => {
                conn.execute("SELECT 1", []).map_err(|e| DbError::QueryFailed(e.to_string()))?;
                Ok(true)
            }
            Err(e) => Err(DbError::ConnectionFailed(e.to_string())),
        }
    }

    async fn connect(&mut self, config: ConnectionConfig) -> DbResult<()> {
        let path = config.host.clone();
        let conn = Connection::open(&path).map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        self.connection = Some(Mutex::new(conn));
        self.path = Some(path);
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.connection = None;
        Ok(())
    }

    #[instrument(skip(self, sql))]
    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<QueryResult> {
        let start = Instant::now();
        let mutex = self.connection.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let conn = mutex.lock().map_err(|e| DbError::Other(e.to_string()))?;
        
        let upper_sql = sql.trim().to_uppercase();
        let is_select = upper_sql.starts_with("SELECT") || upper_sql.starts_with("PRAGMA") || upper_sql.starts_with("WITH");

        if is_select {
            let mut stmt = conn.prepare(sql).map_err(|e| DbError::QueryFailed(e.to_string()))?;
            let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
            
            let rows_iter = stmt.query_map([], |row| {
                Ok(Self::row_to_map(row))
            }).map_err(|e| DbError::QueryFailed(e.to_string()))?;

            let mut rows = Vec::new();
            for row_result in rows_iter {
                rows.push(row_result.map_err(|e| DbError::QueryFailed(e.to_string()))?);
            }

            Ok(QueryResult {
                columns: column_names,
                rows,
                affected_rows: 0,
                execution_time_ms: start.elapsed().as_millis(),
            })
        } else {
            let affected = conn.execute(sql, []).map_err(|e| DbError::QueryFailed(e.to_string()))?;
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: affected as u64,
                execution_time_ms: start.elapsed().as_millis(),
            })
        }
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let name = self.path.as_ref()
            .and_then(|p| Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("main")
            .to_string();
        Ok(vec![DatabaseInfo { name, charset: Some("UTF-8".to_string()), collation: None }])
    }

    async fn get_tables(&self, _database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let sql = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name";
        let res = self.execute_query(sql, None).await?;
        Ok(res.rows.into_iter().map(|r| TableInfo {
            name: r.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            schema: None, table_type: "TABLE".to_string(), engine: None, rows: None, size_mb: None, comment: None,
        }).collect())
    }

    async fn get_views(&self, _database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let sql = "SELECT name FROM sqlite_master WHERE type='view' ORDER BY name";
        let res = self.execute_query(sql, None).await?;
        Ok(res.rows.into_iter().map(|r| TableInfo {
            name: r.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            schema: None, table_type: "VIEW".to_string(), engine: None, rows: None, size_mb: None, comment: None,
        }).collect())
    }

    async fn get_table_structure(&self, table: &str, _schema: Option<&str>, _database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let sql = format!("PRAGMA table_info('{}')", table.replace("'", "''"));
        let res = self.execute_query(&sql, None).await?;
        Ok(res.rows.into_iter().map(|r| ColumnInfo {
            name: r.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            data_type: r.get("type").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            nullable: r.get("notnull").and_then(|v| v.as_i64()).map(|n| n == 0).unwrap_or(true),
            default_value: r.get("dflt_value").and_then(|v| v.as_str()).map(|s| s.to_string()),
            is_primary_key: r.get("pk").and_then(|v| v.as_i64()).map(|n| n > 0).unwrap_or(false),
            is_auto_increment: false, comment: None, character_maximum_length: None, numeric_precision: None, numeric_scale: None,
        }).collect())
    }

    async fn get_indexes(&self, table: &str, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let sql = format!("PRAGMA index_list('{}')", table.replace("'", "''"));
        let res = self.execute_query(&sql, None).await?;
        let mut indexes = Vec::new();
        for r in res.rows {
            let name = r.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let is_unique = r.get("unique").and_then(|v| v.as_i64()).map(|n| n > 0).unwrap_or(false);
            let col_sql = format!("PRAGMA index_info('{}')", name.replace("'", "''"));
            let col_res = self.execute_query(&col_sql, None).await?;
            let columns = col_res.rows.into_iter()
                .map(|cr| cr.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string())
                .collect();
            indexes.push(IndexInfo { name, columns, is_unique, is_primary: false, index_type: "BTREE".into() });
        }
        Ok(indexes)
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
