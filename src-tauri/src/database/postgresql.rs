use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use tokio_postgres::{Client, NoTls, Row, types::ToSql, Config, SimpleQueryMessage};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;

use super::traits::*;

/// PostgreSQL 数据库连接 - 生产级协议实现
pub struct PostgreSqlDatabase {
    client: Option<Client>,
    config: Option<ConnectionConfig>,
}

impl PostgreSqlDatabase {
    pub fn new() -> Self {
        Self { client: None, config: None }
    }

    fn create_pg_config(config: &ConnectionConfig) -> Config {
        let mut pg_config = Config::new();
        pg_config.host(&config.host);
        pg_config.port(config.port);
        pg_config.user(&config.username);
        pg_config.password(&config.password);
        if let Some(ref db) = config.database {
            if !db.trim().is_empty() { pg_config.dbname(db); }
        }
        pg_config.connect_timeout(std::time::Duration::from_secs(10));
        pg_config
    }
}

#[async_trait]
impl DatabaseOperations for PostgreSqlDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let pg_config = Self::create_pg_config(config);
        
        let client = if config.ssl {
            let connector = TlsConnector::builder().danger_accept_invalid_certs(true).build().map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            let make_connector = MakeTlsConnector::new(connector);
            let (c, conn) = pg_config.connect(make_connector).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = conn.await { eprintln!("PG SSL Connection Error: {}", e); } });
            c
        } else {
            let (c, conn) = pg_config.connect(NoTls).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = conn.await { eprintln!("PG Connection Error: {}", e); } });
            c
        };
        
        client.simple_query("SELECT 1").await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(true)
    }

    async fn connect(&mut self, config: ConnectionConfig) -> DbResult<()> {
        let pg_config = Self::create_pg_config(&config);
        
        let client = if config.ssl {
            let connector = TlsConnector::builder().danger_accept_invalid_certs(true).build().map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            let make_connector = MakeTlsConnector::new(connector);
            let (c, conn) = pg_config.connect(make_connector).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = conn.await { eprintln!("PG SSL Connection Error: {}", e); } });
            c
        } else {
            let (c, conn) = pg_config.connect(NoTls).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = conn.await { eprintln!("PG Connection Error: {}", e); } });
            c
        };
        
        self.client = Some(client);
        self.config = Some(config);
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> { self.client = None; Ok(()) }

    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<QueryResult> {
        let start = Instant::now();
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("Not connected".into()))?;
        
        let results = client.simple_query(sql).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut final_columns = Vec::new();
        let mut final_rows = Vec::new();
        let mut total_affected = 0;

        for msg in results {
            match msg {
                SimpleQueryMessage::Row(row) => {
                    if final_columns.is_empty() {
                        for i in 0..row.len() { final_columns.push(format!("col_{}", i)); }
                    }
                    let mut row_map = HashMap::new();
                    for i in 0..row.len() {
                        let val = row.get(i).map(|s| serde_json::Value::String(s.to_string())).unwrap_or(serde_json::Value::Null);
                        row_map.insert(format!("col_{}", i), val);
                    }
                    final_rows.push(row_map);
                },
                SimpleQueryMessage::CommandComplete(count) => { total_affected += count; },
                _ => {}
            }
        }

        // 尝试修正列名 (使用二进制协议获取元数据)
        if !final_rows.is_empty() {
            if let Ok(meta_rows) = client.query(sql, &[]).await {
                if !meta_rows.is_empty() {
                    final_columns.clear();
                    for col in meta_rows[0].columns() { final_columns.push(col.name().to_string()); }
                    let mut mapped_rows = Vec::new();
                    for old_row in final_rows {
                        let mut new_row = HashMap::new();
                        for (i, name) in final_columns.iter().enumerate() {
                            if let Some(val) = old_row.get(&format!("col_{}", i)) {
                                new_row.insert(name.clone(), val.clone());
                            }
                        }
                        mapped_rows.push(new_row);
                    }
                    final_rows = mapped_rows;
                }
            }
        }

        let row_count = final_rows.len() as u64;
        Ok(QueryResult {
            columns: final_columns,
            rows: final_rows,
            affected_rows: row_count.max(total_affected),
            execution_time_ms: start.elapsed().as_millis(),
        })
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("Not connected".into()))?;
        let rows = client.query("SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname", &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| DatabaseInfo { name: r.get(0), charset: None, collation: None }).collect())
    }

    async fn get_schemas(&self, _db: Option<&str>) -> DbResult<Vec<SchemaInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("Not connected".into()))?;
        let sql = "SELECT nspname, pg_catalog.pg_get_userbyid(nspowner) FROM pg_catalog.pg_namespace WHERE nspname NOT LIKE 'pg_%' AND nspname != 'information_schema' ORDER BY nspname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| SchemaInfo { name: r.get(0), owner: r.try_get(1).ok(), comment: None }).collect())
    }

    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("Not connected".into()))?;
        let sql = "SELECT n.nspname, c.relname, obj_description(c.oid) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'r' AND n.nspname NOT IN ('pg_catalog', 'information_schema') ORDER BY n.nspname, c.relname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| TableInfo { name: r.get(1), schema: Some(r.get(0)), table_type: "TABLE".into(), engine: None, rows: None, size_mb: None, comment: r.try_get(2).ok() }).collect())
    }

    async fn get_views(&self, _db: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("Not connected".into()))?;
        let sql = "SELECT n.nspname, c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'v' AND n.nspname NOT IN ('pg_catalog', 'information_schema') ORDER BY n.nspname, c.relname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| TableInfo { name: r.get(1), schema: Some(r.get(0)), table_type: "VIEW".into(), engine: None, rows: None, size_mb: None, comment: None }).collect())
    }

    async fn get_table_structure(&self, table: &str, schema: Option<&str>, _db: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("Not connected".into()))?;
        let schema_name = schema.unwrap_or("public");
        let sql = "SELECT a.attname, pg_catalog.format_type(a.atttypid, a.atttypmod), NOT a.attnotnull FROM pg_catalog.pg_attribute a JOIN pg_catalog.pg_class c ON a.attrelid = c.oid JOIN pg_catalog.pg_namespace n ON c.relnamespace = n.oid WHERE n.nspname = $1 AND c.relname = $2 AND a.attnum > 0 AND NOT a.attisdropped";
        let rows = client.query(sql, &[&schema_name, &table]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| ColumnInfo { name: r.get(0), data_type: r.get(1), nullable: r.get(2), default_value: None, is_primary_key: false, is_auto_increment: false, comment: None, character_maximum_length: None, numeric_precision: None, numeric_scale: None }).collect())
    }

    async fn get_functions(&self, _db: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("Not connected".into()))?;
        let schema_name = schema.unwrap_or("public");
        let sql = "SELECT p.proname, n.nspname, pg_catalog.pg_get_function_result(p.oid) FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = $1 AND p.prokind = 'f'";
        let rows = client.query(sql, &[&schema_name]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| FunctionInfo { name: r.get(0), schema: Some(r.get(1)), return_type: r.try_get(2).ok(), arguments: None, language: None, function_type: "FUNCTION".into(), comment: None }).collect())
    }

    async fn switch_database(&mut self, db: &str) -> DbResult<()> {
        if let Some(mut config) = self.config.clone() {
            config.database = Some(db.to_string());
            self.connect(config).await
        } else { Err(DbError::ConnectionFailed("No config".into())) }
    }
    
    async fn get_indexes(&self, _t: &str, _s: Option<&str>) -> DbResult<Vec<IndexInfo>> { Ok(vec![]) }
    async fn get_aggregate_functions(&self, _db: Option<&str>, _s: Option<&str>) -> DbResult<Vec<FunctionInfo>> { Ok(vec![]) }
    fn as_any(&self) -> &dyn std::any::Any { self }
}
