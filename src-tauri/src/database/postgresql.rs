use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use tokio_postgres::{Client, NoTls};
use tracing::{info, instrument, error, debug};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use tokio::sync::Mutex;

use super::traits::*;

/// PostgreSQL 驱动状态容器 - 用于内部互斥
struct PgState {
    client: Option<Client>,
    config: Option<ConnectionConfig>,
}

/// PostgreSQL 数据库驱动 - 基于 tokio-postgres 的底层实现 (具备内部并发能力)
pub struct PostgreSqlDatabase {
    state: Mutex<PgState>,
}

impl PostgreSqlDatabase {
    pub fn new() -> Self {
        Self { 
            state: Mutex::new(PgState { client: None, config: None })
        }
    }

    async fn create_client(config: &ConnectionConfig) -> DbResult<Client> {
        let db_name = config.database.as_deref().unwrap_or("postgres");
        let conn_str = format!(
            "host={} port={} user={} password={} dbname={}",
            config.host, config.port, config.username, config.password, db_name
        );
        
        if config.ssl {
            let connector = TlsConnector::builder().danger_accept_invalid_certs(true).build().map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            let connector = MakeTlsConnector::new(connector);
            let (client, connection) = tokio_postgres::connect(&conn_str, connector).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = connection.await { error!("PostgreSQL SSL 连接异常: {}", e); } });
            Ok(client)
        } else {
            let (client, connection) = tokio_postgres::connect(&conn_str, NoTls).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            tokio::spawn(async move { if let Err(e) = connection.await { error!("PostgreSQL 连接异常: {}", e); } });
            Ok(client)
        }
    }
}

#[async_trait]
impl DatabaseOperations for PostgreSqlDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let client = Self::create_client(config).await?;
        client.query("SELECT 1", &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(true)
    }

    async fn connect(&self, config: ConnectionConfig) -> DbResult<()> {
        let client = Self::create_client(&config).await?;
        let mut state = self.state.lock().await;
        state.client = Some(client);
        state.config = Some(config);
        Ok(())
    }

    async fn disconnect(&self) -> DbResult<()> {
        let mut state = self.state.lock().await;
        state.client = None;
        state.config = None;
        Ok(())
    }

    async fn switch_database(&self, database: &str) -> DbResult<()> {
        let mut state = self.state.lock().await;
        let mut config = state.config.clone().ok_or(DbError::Other("未找到初始配置".into()))?;
        
        if config.database.as_deref() == Some(database) {
            return Ok(());
        }
        
        info!(new_db = %database, "PostgreSQL 正在物理切换数据库连接...");
        config.database = Some(database.to_string());
        
        let client = Self::create_client(&config).await?;
        state.client = Some(client);
        state.config = Some(config);
        Ok(())
    }

    #[instrument(skip(self, sql))]
    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let start = Instant::now();
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        
        debug!(sql = %sql.replace('\n', " "), "执行查询");

        // 1. 执行 simple_query (文本协议)，它能自动处理多条语句
        let messages = client.simple_query(sql).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        
        let mut results = Vec::new();
        let mut current_columns = Vec::new();
        let mut current_rows = Vec::new();

        for msg in messages {
            match msg {
                tokio_postgres::SimpleQueryMessage::RowDescription(columns) => {
                    // 获取真实的列名
                    current_columns = columns.iter().map(|c| c.name().to_string()).collect();
                },
                tokio_postgres::SimpleQueryMessage::Row(row) => {
                    // 如果由于某种原因没有获取到 RowDescription，则使用占位符
                    if current_columns.is_empty() {
                        for i in 0..row.len() {
                            current_columns.push(format!("column_{}", i + 1));
                        }
                    }
                    
                    let mut row_map = HashMap::new();
                    for i in 0..row.len() {
                        let col_name = current_columns.get(i).cloned().unwrap_or_else(|| format!("column_{}", i + 1));
                        let val = row.get(i).map(|s| serde_json::Value::String(s.to_string())).unwrap_or(serde_json::Value::Null);
                        row_map.insert(col_name, val);
                    }
                    current_rows.push(row_map);
                },
                tokio_postgres::SimpleQueryMessage::CommandComplete(count) => {
                    // 语句执行完成，打包当前结果集
                    results.push(QueryResult {
                        columns: current_columns.clone(),
                        rows: current_rows.clone(),
                        affected_rows: count,
                        execution_time_ms: start.elapsed().as_millis(),
                    });
                    // 重置临时容器，准备下一个结果集
                    current_columns.clear();
                    current_rows.clear();
                },
                _ => {}
            }
        }

        // 处理没有显式 CommandComplete 的剩余数据（防御性编程）
        if !current_rows.is_empty() || !current_columns.is_empty() {
            results.push(QueryResult {
                columns: current_columns,
                rows: current_rows,
                affected_rows: 0,
                execution_time_ms: start.elapsed().as_millis(),
            });
        }

        // 如果一条结果都没有（比如执行了空语句），返回一个空的成功结果
        if results.is_empty() {
            results.push(QueryResult {
                columns: vec![],
                rows: vec![],
                affected_rows: 0,
                execution_time_ms: start.elapsed().as_millis(),
            });
        }

        Ok(results)
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let rows = client.query("SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname", &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| DatabaseInfo { name: r.get(0), charset: None, collation: None }).collect())
    }

    async fn get_schemas(&self, _db: Option<&str>) -> DbResult<Vec<SchemaInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let sql = "SELECT nspname, pg_catalog.pg_get_userbyid(nspowner) FROM pg_catalog.pg_namespace WHERE nspname NOT LIKE 'pg_%' AND nspname != 'information_schema' ORDER BY nspname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| SchemaInfo { name: r.get(0), owner: r.try_get(1).ok(), comment: None }).collect())
    }

    async fn get_tables(&self, _db: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let sql = "SELECT n.nspname, c.relname, obj_description(c.oid) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'r' AND n.nspname NOT IN ('pg_catalog', 'information_schema') ORDER BY n.nspname, c.relname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| TableInfo { name: r.get(1), schema: Some(r.get(0)), table_type: "TABLE".into(), engine: None, rows: None, size_mb: None, comment: r.try_get(2).ok() }).collect())
    }

    async fn get_views(&self, _db: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let sql = "SELECT n.nspname, c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'v' AND n.nspname NOT IN ('pg_catalog', 'information_schema') ORDER BY n.nspname, c.relname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| TableInfo { name: r.get(1), schema: Some(r.get(0)), table_type: "VIEW".into(), engine: None, rows: None, size_mb: None, comment: None }).collect())
    }

    async fn get_table_structure(&self, table: &str, schema: Option<&str>, _db: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let schema_name = schema.unwrap_or("public");
        
        let sql = "
            SELECT a.attname, format_type(a.atttypid, a.atttypmod), CASE WHEN a.attnotnull THEN 'NO' ELSE 'YES' END, pg_get_expr(d.adbin, d.adrelid), CASE WHEN a.attlen = -1 THEN 0 ELSE a.attlen END
            FROM pg_attribute a JOIN pg_class c ON a.attrelid = c.oid JOIN pg_namespace n ON c.relnamespace = n.oid LEFT JOIN pg_attrdef d ON a.attrelid = d.adrelid AND a.attnum = d.adnum
            WHERE c.relname = $1 AND n.nspname = $2 AND a.attnum > 0 AND NOT a.attisdropped ORDER BY a.attnum;
        ";
        let rows = client.query(sql, &[&table, &schema_name]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        
        let pk_sql = "SELECT a.attname FROM pg_index i JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey) WHERE i.indrelid = ($1::text)::regclass AND i.indisprimary";
        let pk_rows = client.query(pk_sql, &[&format!("{}.{}", schema_name, table)]).await.unwrap_or_default();
        let pk_cols: Vec<String> = pk_rows.into_iter().map(|r| r.get(0)).collect();

        Ok(rows.into_iter().map(|r| {
            let name: String = r.get(0);
            let data_type: String = r.get(1);
            let nullable: String = r.get(2);
            let is_pk = pk_cols.contains(&name);
            let max_len: i32 = r.get(4);
            ColumnInfo {
                name, data_type, nullable: nullable == "YES",
                default_value: r.try_get(3).ok(), is_primary_key: is_pk, is_auto_increment: false,
                comment: None, character_maximum_length: if max_len > 0 { Some(max_len as i64) } else { None }, 
                numeric_precision: None, numeric_scale: None,
            }
        }).collect())
    }

    async fn update_data(&self, table: &str, schema: Option<&str>, column: &str, value: Option<String>, where_clause: &str) -> DbResult<()> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        
        let schema_name = schema.unwrap_or("public");
        let val_str = match value {
            Some(v) => format!("'{}'", v.replace("'", "''")),
            None => "NULL".to_string(),
        };

        let sql = format!("UPDATE \"{}\".\"{}\" SET \"{}\" = {} WHERE {}", schema_name, table, column, val_str, where_clause);
        debug!(sql = %sql, "执行 PostgreSQL 更新");
        
        client.batch_execute(&sql).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(())
    }

    async fn get_indexes(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let schema_name = schema.unwrap_or("public");
        let sql = "SELECT i.relname, a.attname, ix.indisunique, ix.indisprimary FROM pg_class t JOIN pg_index ix ON t.oid = ix.indrelid JOIN pg_class i ON i.oid = ix.indexrelid JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) JOIN pg_namespace n ON n.oid = t.relnamespace WHERE t.relname = $1 AND n.nspname = $2";
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

    async fn get_schema_indexes(&self, _database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let schema_name = schema.unwrap_or("public");
        let sql = "SELECT i.relname, a.attname, ix.indisunique, ix.indisprimary FROM pg_index ix JOIN pg_class i ON i.oid = ix.indexrelid JOIN pg_class t ON t.oid = ix.indrelid JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) JOIN pg_namespace n ON n.oid = i.relnamespace WHERE n.nspname = $1";
        let rows = client.query(sql, &[&schema_name]).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        let mut map: HashMap<String, IndexInfo> = HashMap::new();
        for r in rows {
            let name: String = r.get(0);
            let col: String = r.get(1);
            let entry = map.entry(name.clone()).or_insert(IndexInfo { name, columns: vec![], is_unique: r.get(2), is_primary: r.get(3), index_type: "BTREE".into() });
            entry.columns.push(col);
        }
        Ok(map.into_values().collect())
    }

    async fn get_table_ddl(&self, table: &str, schema: Option<&str>) -> DbResult<String> {
        let schema_name = schema.unwrap_or("public");
        let columns = self.get_table_structure(table, Some(schema_name), None).await?;
        
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        
        // 1. 检查是否为视图
        let view_sql = "SELECT pg_get_viewdef(c.oid, true) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = $1 AND c.relname = $2 AND c.relkind = 'v'";
        let view_rows = client.query(view_sql, &[&schema_name, &table]).await.unwrap_or_default();
        
        if let Some(row) = view_rows.first() {
            let definition: String = row.get(0);
            return Ok(format!("CREATE OR REPLACE VIEW \"{}\".\"{}\" AS\n{}", schema_name, table, definition));
        }

        // 2. 否则按表处理，重构 CREATE TABLE 语句
        let mut ddl = format!("CREATE TABLE \"{}\".\"{}\" (\n", schema_name, table);
        let mut col_defs = Vec::new();
        let mut pks = Vec::new();

        for col in columns {
            let mut def = format!("  \"{}\" {}", col.name, col.data_type);
            if !col.nullable { def.push_str(" NOT NULL"); }
            if let Some(ref d) = col.default_value { def.push_str(&format!(" DEFAULT {}", d)); }
            col_defs.push(def);
            if col.is_primary_key { pks.push(format!("\"{}\"", col.name)); }
        }

        ddl.push_str(&col_defs.join(",\n"));
        if !pks.is_empty() {
            ddl.push_str(&format!(",\n  PRIMARY KEY ({})", pks.join(", ")));
        }
        ddl.push_str("\n);");

        Ok(ddl)
    }

    async fn explain_query(&self, sql: &str, database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let explain_sql = format!("EXPLAIN (ANALYZE, COSTS, VERBOSE, BUFFERS, FORMAT JSON) {}", sql);
        self.execute_query(&explain_sql, database).await
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
