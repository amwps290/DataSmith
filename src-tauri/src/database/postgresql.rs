use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use tokio_postgres::{Client, NoTls};
use tracing::{instrument};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;

use super::traits::*;

/// PostgreSQL 数据库驱动 - 基于 tokio-postgres 的底层实现
pub struct PostgreSqlDatabase {
    client: Option<Client>,
}

impl PostgreSqlDatabase {
    pub fn new() -> Self {
        Self { client: None }
    }
}

#[async_trait]
impl DatabaseOperations for PostgreSqlDatabase {
    #[instrument(skip(self, config))]
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let conn_str = format!(
            "host={} port={} user={} password={} dbname={}",
            config.host,
            config.port,
            config.username,
            config.password,
            config.database.as_deref().unwrap_or("postgres")
        );
        
        if config.ssl {
            let connector = TlsConnector::builder().danger_accept_invalid_certs(true).build().map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            let connector = MakeTlsConnector::new(connector);
            let (client, connection) = tokio_postgres::connect(&conn_str, connector).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = connection.await { eprintln!("connection error: {}", e); } });
            client.query("SELECT 1", &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        } else {
            let (client, connection) = tokio_postgres::connect(&conn_str, NoTls).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = connection.await { eprintln!("connection error: {}", e); } });
            client.query("SELECT 1", &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        }
        Ok(true)
    }

    #[instrument(skip(self, config))]
    async fn connect(&mut self, config: ConnectionConfig) -> DbResult<()> {
        let conn_str = format!(
            "host={} port={} user={} password={} dbname={}",
            config.host,
            config.port,
            config.username,
            config.password,
            config.database.as_deref().unwrap_or("postgres")
        );

        if config.ssl {
            let connector = TlsConnector::builder().danger_accept_invalid_certs(true).build().map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            let connector = MakeTlsConnector::new(connector);
            let (client, connection) = tokio_postgres::connect(&conn_str, connector).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = connection.await { eprintln!("connection error: {}", e); } });
            self.client = Some(client);
        } else {
            let (client, connection) = tokio_postgres::connect(&conn_str, NoTls).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = connection.await { eprintln!("connection error: {}", e); } });
            self.client = Some(client);
        }
        Ok(())
    }

    async fn disconnect(&mut self) -> DbResult<()> {
        self.client = None;
        Ok(())
    }

    #[instrument(skip(self, sql))]
    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<QueryResult> {
        let start = Instant::now();
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        
        let messages = client.simple_query(sql).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        
        let mut final_columns = Vec::new();
        let mut final_rows = Vec::new();
        let mut total_affected = 0;

        for msg in messages {
            match msg {
                tokio_postgres::SimpleQueryMessage::Row(row) => {
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
                tokio_postgres::SimpleQueryMessage::CommandComplete(count) => { total_affected += count; },
                _ => {}
            }
        }

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

        let duration = start.elapsed().as_millis();
        Ok(QueryResult {
            columns: final_columns,
            rows: final_rows,
            affected_rows: total_affected,
            execution_time_ms: duration as u128,
        })
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let rows = client.query("SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname", &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| DatabaseInfo { name: r.get(0), charset: None, collation: None }).collect())
    }

    async fn get_schemas(&self, _db: Option<&str>) -> DbResult<Vec<SchemaInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let sql = "SELECT nspname, pg_catalog.pg_get_userbyid(nspowner) FROM pg_catalog.pg_namespace WHERE nspname NOT LIKE 'pg_%' AND nspname != 'information_schema' ORDER BY nspname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| SchemaInfo { name: r.get(0), owner: r.try_get(1).ok(), comment: None }).collect())
    }

    async fn get_tables(&self, _db: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let sql = "SELECT n.nspname, c.relname, obj_description(c.oid) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'r' AND n.nspname NOT IN ('pg_catalog', 'information_schema') ORDER BY n.nspname, c.relname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| TableInfo { name: r.get(1), schema: Some(r.get(0)), table_type: "TABLE".into(), engine: None, rows: None, size_mb: None, comment: r.try_get(2).ok() }).collect())
    }

    async fn get_views(&self, _db: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let sql = "SELECT n.nspname, c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'v' AND n.nspname NOT IN ('pg_catalog', 'information_schema') ORDER BY n.nspname, c.relname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| TableInfo { name: r.get(1), schema: Some(r.get(0)), table_type: "VIEW".into(), engine: None, rows: None, size_mb: None, comment: None }).collect())
    }

    async fn get_table_structure(&self, table: &str, schema: Option<&str>, _db: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let schema_name = schema.unwrap_or("public");
        
        // 改用更精准的底层查询，利用 format_type 获取包括 geometry 在内的真实类型名
        let sql = "
            SELECT 
                a.attname as column_name,
                format_type(a.atttypid, a.atttypmod) as data_type,
                CASE WHEN a.attnotnull THEN 'NO' ELSE 'YES' END as is_nullable,
                pg_get_expr(d.adbin, d.adrelid) as column_default,
                CASE WHEN a.attlen = -1 THEN 0 ELSE a.attlen END as character_maximum_length
            FROM pg_attribute a
            JOIN pg_class c ON a.attrelid = c.oid
            JOIN pg_namespace n ON c.relnamespace = n.oid
            LEFT JOIN pg_attrdef d ON a.attrelid = d.adrelid AND a.attnum = d.adnum
            WHERE c.relname = $1 
              AND n.nspname = $2 
              AND a.attnum > 0 
              AND NOT a.attisdropped
            ORDER BY a.attnum;
        ";
        
        let rows = client.query(sql, &[&table, &schema_name]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        
        let pk_sql = "SELECT a.attname FROM pg_index i JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey) WHERE i.indrelid = ($1::text)::regclass AND i.indisprimary";
        let pk_rows = client.query(pk_sql, &[&format!("{}.{}", schema_name, table)]).await.unwrap_or_default();
        let pk_cols: Vec<String> = pk_rows.into_iter().map(|r| r.get(0)).collect();

        Ok(rows.into_iter().map(|r| {
            let name: String = r.get(0);
            let is_pk = pk_cols.contains(&name);
            let max_len: i32 = r.get(4);
            
            ColumnInfo {
                name, 
                data_type: r.get(1), 
                nullable: r.get::<_, String>(2) == "YES",
                default_value: r.try_get(3).ok(), 
                is_primary_key: is_pk, 
                is_auto_increment: false,
                comment: None, 
                character_maximum_length: if max_len > 0 { Some(max_len as i64) } else { None }, 
                numeric_precision: None, 
                numeric_scale: None,
            }
        }).collect())
    }

    async fn get_indexes(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let schema_name = schema.unwrap_or("public");
        let sql = "SELECT i.relname as index_name, a.attname as column_name, ix.indisunique as is_unique, ix.indisprimary as is_primary FROM pg_class t JOIN pg_index ix ON t.oid = ix.indrelid JOIN pg_class i ON i.oid = ix.indexrelid JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) JOIN pg_namespace n ON n.oid = t.relnamespace WHERE t.relname = $1 AND n.nspname = $2";
        let rows = client.query(sql, &[&table, &schema_name]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        
        let mut map: HashMap<String, IndexInfo> = HashMap::new();
        for r in rows {
            let name: String = r.get(0);
            let col: String = r.get(1);
            let entry = map.entry(name.clone()).or_insert(IndexInfo { name, columns: vec![], is_unique: r.get(2), is_primary: r.get(3), index_type: "BTREE".into() });
            entry.columns.push(col);
        }
        Ok(map.into_values().collect())
    }

    async fn get_functions(&self, _database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let schema_name = schema.unwrap_or("public");
        let sql = "SELECT p.proname, n.nspname, pg_catalog.pg_get_function_result(p.oid), pg_catalog.pg_get_function_arguments(p.oid), l.lanname, obj_description(p.oid, 'pg_proc') FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace JOIN pg_language l ON l.oid = p.prolang WHERE n.nspname = $1 AND p.prokind != 'a' ORDER BY p.proname";
        let rows = client.query(sql, &[&schema_name]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| FunctionInfo { name: r.get(0), schema: Some(r.get(1)), return_type: Some(r.get(2)), arguments: Some(r.get(3)), language: Some(r.get(4)), function_type: "function".into(), comment: r.try_get(5).ok() }).collect())
    }

    async fn get_aggregate_functions(&self, _database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let schema_name = schema.unwrap_or("public");
        let sql = "SELECT p.proname, n.nspname, pg_catalog.pg_get_function_result(p.oid), pg_catalog.pg_get_function_arguments(p.oid), obj_description(p.oid, 'pg_proc') FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = $1 AND p.prokind = 'a' ORDER BY p.proname";
        let rows = client.query(sql, &[&schema_name]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| FunctionInfo { name: r.get(0), schema: Some(r.get(1)), return_type: Some(r.get(2)), arguments: Some(r.get(3)), language: None, function_type: "aggregate".into(), comment: r.try_get(4).ok() }).collect())
    }

    async fn get_extensions(&self, _database: Option<&str>) -> DbResult<Vec<ExtensionInfo>> {
        let client = self.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let sql = "SELECT extname, extversion, n.nspname, obj_description(e.oid, 'pg_extension') FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace ORDER BY extname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| ExtensionInfo { name: r.get(0), version: r.get(1), schema: Some(r.get(2)), comment: r.try_get(3).ok() }).collect())
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
