use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio_postgres::{Client, NoTls};
use tracing::{info, instrument, error, debug};
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use tokio::sync::Mutex;

use super::traits::*;
use super::constants::PG_DEFAULT_SCHEMA;
use crate::utils::sql_sanitize::{escape_pg_id, escape_string_literal};

/// PostgreSQL 驱动状态容器 - 用于内部互斥
struct PgState {
    client: Option<Arc<Client>>,
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

    /// libpq 连接字符串值转义：用单引号包裹，转义反斜杠和单引号
    fn escape_connstr_value(val: &str) -> String {
        format!("'{}'", val.replace('\\', "\\\\").replace('\'', "\\'"))
    }

    /// 获取已连接客户端的 Arc 引用（不持有锁），若未连接则返回错误
    async fn get_client_arc(&self) -> DbResult<Arc<Client>> {
        let state = self.state.lock().await;
        state.client.as_ref().cloned().ok_or(DbError::not_connected())
    }

    async fn create_client(config: &ConnectionConfig) -> DbResult<Client> {
        let db_name = config.database.as_deref().unwrap_or("postgres");
        let conn_str = format!(
            "host={} port={} user={} password={} dbname={}",
            Self::escape_connstr_value(&config.host),
            config.port,
            Self::escape_connstr_value(&config.username),
            Self::escape_connstr_value(&config.password),
            Self::escape_connstr_value(db_name)
        );
        
        debug!(conn = %conn_str.replace(&config.password, "******"), "正在建立 PostgreSQL 物理连接...");

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

    fn json_to_pg_literal(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(v) => {
                if *v { "TRUE".to_string() } else { "FALSE".to_string() }
            }
            serde_json::Value::Number(n) => {
                if let Some(v) = n.as_i64() {
                    v.to_string()
                } else if let Some(v) = n.as_u64() {
                    v.to_string()
                } else if let Some(v) = n.as_f64() {
                    v.to_string()
                } else {
                    "NULL".to_string()
                }
            }
            serde_json::Value::String(v) => escape_string_literal(v),
            serde_json::Value::Array(_) | serde_json::Value::Object(_) => escape_string_literal(&value.to_string()),
        }
    }

    fn string_to_pg_literal(value: Option<String>) -> String {
        match value {
            Some(v) => escape_string_literal(&v),
            None => "NULL".to_string(),
        }
    }

    fn build_literal_where_clause(where_conditions: &HashMap<String, serde_json::Value>) -> String {
        let mut entries = where_conditions.iter().collect::<Vec<_>>();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        entries
            .into_iter()
            .map(|(column, value)| {
                if value.is_null() {
                    format!("{} IS NULL", escape_pg_id(column))
                } else {
                    format!("{} = {}", escape_pg_id(column), Self::json_to_pg_literal(value))
                }
            })
            .collect::<Vec<_>>()
            .join(" AND ")
    }

    fn format_pg_error(error: tokio_postgres::Error) -> String {
        if let Some(db_error) = error.as_db_error() {
            let mut parts = vec![db_error.message().to_string()];

            if let Some(detail) = db_error.detail() {
                parts.push(format!("detail: {}", detail));
            }
            if let Some(hint) = db_error.hint() {
                parts.push(format!("hint: {}", hint));
            }
            if let Some(position) = db_error.position() {
                parts.push(format!("position: {:?}", position));
            }
            if let Some(table) = db_error.table() {
                parts.push(format!("table: {}", table));
            }
            if let Some(column) = db_error.column() {
                parts.push(format!("column: {}", column));
            }
            if let Some(constraint) = db_error.constraint() {
                parts.push(format!("constraint: {}", constraint));
            }

            return parts.join(" | ");
        }

        error.to_string()
    }
}

#[async_trait]
impl DatabaseOperations for PostgreSqlDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let client = Self::create_client(config).await?;
        client.query("SELECT 1", &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(true)
    }

    async fn connect(&self, config: ConnectionConfig) -> DbResult<()> {
        let client = Self::create_client(&config).await?;
        let mut state = self.state.lock().await;
        state.client = Some(Arc::new(client));
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
        state.client = Some(Arc::new(client));
        state.config = Some(config);
        Ok(())
    }

    #[instrument(skip(self, sql))]
    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let start = Instant::now();
        let client = self.get_client_arc().await?;
        
        debug!(sql = %sql.replace('\n', " "), "执行查询");

        // 1. 执行 simple_query (文本协议)，它能自动处理多条语句
        let messages = client.simple_query(sql).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        
        let mut results = Vec::new();
        let mut current_columns = Vec::new();
        let mut current_rows = Vec::new();

        for msg in messages {
            match msg {
                tokio_postgres::SimpleQueryMessage::RowDescription(columns) => {
                    current_columns = columns.iter().map(|c| c.name().to_string()).collect();
                },
                tokio_postgres::SimpleQueryMessage::Row(row) => {
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
                    results.push(QueryResult {
                        columns: current_columns.clone(),
                        rows: current_rows.clone(),
                        affected_rows: count,
                        execution_time_ms: start.elapsed().as_millis(),
                    });
                    current_columns.clear();
                    current_rows.clear();
                },
                _ => {}
            }
        }

        if !current_rows.is_empty() || !current_columns.is_empty() {
            results.push(QueryResult {
                columns: current_columns,
                rows: current_rows,
                affected_rows: 0,
                execution_time_ms: start.elapsed().as_millis(),
            });
        }

        if results.is_empty() {
            results.push(QueryResult::empty(start.elapsed().as_millis()));
        }

        Ok(results)
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let client = self.get_client_arc().await?;
        let rows = client.query("SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname", &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(rows.into_iter().map(|r| DatabaseInfo { name: r.get(0), charset: None, collation: None }).collect())
    }

    async fn get_schemas(&self, _db: Option<&str>) -> DbResult<Vec<SchemaInfo>> {
        let client = self.get_client_arc().await?;
        let sql = "SELECT nspname, pg_catalog.pg_get_userbyid(nspowner) FROM pg_catalog.pg_namespace WHERE nspname NOT LIKE 'pg_%' AND nspname != 'information_schema' ORDER BY nspname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(rows.into_iter().map(|r| SchemaInfo { name: r.get(0), owner: r.try_get(1).ok(), comment: None }).collect())
    }

    async fn get_tables(&self, _db: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let client = self.get_client_arc().await?;
        let sql = "SELECT n.nspname, c.relname, obj_description(c.oid) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'r' AND n.nspname NOT IN ('pg_catalog', 'information_schema') ORDER BY n.nspname, c.relname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(rows.into_iter().map(|r| TableInfo { name: r.get(1), schema: Some(r.get(0)), table_type: "TABLE".into(), engine: None, rows: None, size_mb: None, comment: r.try_get(2).ok() }).collect())
    }

    async fn get_views(&self, _db: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let client = self.get_client_arc().await?;
        let sql = "SELECT n.nspname, c.relname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'v' AND n.nspname NOT IN ('pg_catalog', 'information_schema') ORDER BY n.nspname, c.relname";
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(rows.into_iter().map(|r| TableInfo { name: r.get(1), schema: Some(r.get(0)), table_type: "VIEW".into(), engine: None, rows: None, size_mb: None, comment: None }).collect())
    }

    async fn get_table_structure(&self, table: &str, schema: Option<&str>, _db: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        
        let sql = "
            SELECT a.attname, format_type(a.atttypid, a.atttypmod), CASE WHEN a.attnotnull THEN 'NO' ELSE 'YES' END, pg_get_expr(d.adbin, d.adrelid), CASE WHEN a.attlen = -1 THEN 0 ELSE a.attlen END
            FROM pg_attribute a JOIN pg_class c ON a.attrelid = c.oid JOIN pg_namespace n ON c.relnamespace = n.oid LEFT JOIN pg_attrdef d ON a.attrelid = d.adrelid AND a.attnum = d.adnum
            WHERE c.relname = $1 AND n.nspname = $2 AND a.attnum > 0 AND NOT a.attisdropped ORDER BY a.attnum;
        ";
        let rows = client.query(sql, &[&table, &schema_name]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        
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

    async fn update_data(&self, table: &str, schema: Option<&str>, column: &str, value: Option<String>, where_conditions: HashMap<String, serde_json::Value>) -> DbResult<()> {
        let client = self.get_client_arc().await?;

        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let where_sql = Self::build_literal_where_clause(&where_conditions);
        let value_sql = Self::string_to_pg_literal(value);

        let sql = format!(
            "UPDATE {}.{} SET {} = {} WHERE {}",
            escape_pg_id(schema_name), escape_pg_id(table), escape_pg_id(column), value_sql, where_sql
        );

        debug!(sql = %sql, "执行 PostgreSQL 更新");
        client.execute(&sql, &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(())
    }

    async fn insert_data(&self, table: &str, schema: Option<&str>, data: HashMap<String, serde_json::Value>) -> DbResult<()> {
        if data.is_empty() {
            return Err(DbError::ConfigError("插入数据不能为空".into()));
        }

        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);

        let mut entries: Vec<(String, serde_json::Value)> = data.into_iter().collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let columns = entries.iter().map(|(name, _)| escape_pg_id(name)).collect::<Vec<_>>().join(", ");
        let values = entries.iter().map(|(_, value)| Self::json_to_pg_literal(value)).collect::<Vec<_>>().join(", ");

        let sql = format!(
            "INSERT INTO {}.{} ({}) VALUES ({})",
            escape_pg_id(schema_name),
            escape_pg_id(table),
            columns,
            values
        );
        debug!(sql = %sql, "执行 PostgreSQL 插入");

        client.execute(&sql, &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(())
    }

    async fn delete_data(&self, table: &str, schema: Option<&str>, where_conditions: HashMap<String, serde_json::Value>) -> DbResult<()> {
        let client = self.get_client_arc().await?;

        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let where_sql = Self::build_literal_where_clause(&where_conditions);

        let sql = format!(
            "DELETE FROM {}.{} WHERE {}",
            escape_pg_id(schema_name), escape_pg_id(table), where_sql
        );

        debug!(sql = %sql, "执行 PostgreSQL 删除");
        client.execute(&sql, &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(())
    }

    async fn truncate_table(&self, table: &str, schema: Option<&str>) -> DbResult<()> {
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let sql = format!(
            "TRUNCATE TABLE {}.{} RESTART IDENTITY",
            escape_pg_id(schema_name),
            escape_pg_id(table)
        );

        debug!(sql = %sql, "执行 PostgreSQL 清空表");
        client.batch_execute(&sql).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(())
    }

    async fn get_indexes(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let sql = "SELECT i.relname, a.attname, ix.indisunique, ix.indisprimary FROM pg_class t JOIN pg_index ix ON t.oid = ix.indrelid JOIN pg_class i ON i.oid = ix.indexrelid JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) JOIN pg_namespace n ON n.oid = t.relnamespace WHERE t.relname = $1 AND n.nspname = $2";
        let rows = client.query(sql, &[&table, &schema_name]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
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
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let sql = "SELECT i.relname, a.attname, ix.indisunique, ix.indisprimary FROM pg_index ix JOIN pg_class i ON i.oid = ix.indexrelid JOIN pg_class t ON t.oid = ix.indrelid JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey) JOIN pg_namespace n ON n.oid = i.relnamespace WHERE n.nspname = $1";
        let rows = client.query(sql, &[&schema_name]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        let mut map: HashMap<String, IndexInfo> = HashMap::new();
        for r in rows {
            let name: String = r.get(0);
            let col: String = r.get(1);
            let entry = map.entry(name.clone()).or_insert(IndexInfo { name, columns: vec![], is_unique: r.get(2), is_primary: r.get(3), index_type: "BTREE".into() });
            entry.columns.push(col);
        }
        debug!(count = map.len(), sc = %schema_name, "已获取 PostgreSQL 索引");
        Ok(map.into_values().collect())
    }

    async fn get_foreign_keys(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<ForeignKeyInfo>> {
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        
        let sql = "
            SELECT
                conname AS constraint_name,
                a.attname AS column_name,
                rt.relname AS referenced_table_name,
                ra.attname AS referenced_column_name,
                confupdtype AS update_rule,
                confdeltype AS delete_rule
            FROM pg_constraint c
            JOIN pg_namespace n ON n.oid = c.connamespace
            JOIN pg_class t ON t.oid = c.conrelid
            JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(c.conkey)
            JOIN pg_class rt ON rt.oid = c.confrelid
            JOIN pg_attribute ra ON ra.attrelid = rt.oid AND ra.attnum = ANY(c.confkey)
            WHERE c.contype = 'f' AND t.relname = $1 AND n.nspname = $2
        ";
        
        let rows = client.query(sql, &[&table, &schema_name]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        
        Ok(rows.into_iter().map(|r| {
            let u_rule: i8 = r.get(4);
            let d_rule: i8 = r.get(5);
            ForeignKeyInfo {
                name: r.get(0),
                column_name: r.get(1),
                referenced_table_name: r.get(2),
                referenced_column_name: r.get(3),
                update_rule: Some(match u_rule as u8 as char {
                    'c' => "CASCADE", 'n' => "SET NULL", 'd' => "SET DEFAULT", 'r' => "RESTRICT", _ => "NO ACTION"
                }.into()),
                delete_rule: Some(match d_rule as u8 as char {
                    'c' => "CASCADE", 'n' => "SET NULL", 'd' => "SET DEFAULT", 'r' => "RESTRICT", _ => "NO ACTION"
                }.into()),
            }
        }).collect())
    }

    async fn alter_table(&self, table: &str, schema: Option<&str>, _database: Option<&str>, changes: Vec<TableChange>) -> DbResult<()> {
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        
        let mut sql_parts = Vec::new();
        for change in &changes {
            match change {
                TableChange::AddColumn(col) => {
                    let mut part = format!("ADD COLUMN {} {}", escape_pg_id(&col.name), col.data_type);
                    if !col.nullable { part.push_str(" NOT NULL"); }
                    if let Some(ref d) = col.default_value { part.push_str(&format!(" DEFAULT {}", escape_string_literal(d))); }
                    sql_parts.push(part);
                },
                TableChange::ModifyColumn { old_name, new_column } => {
                    if old_name != &new_column.name {
                        sql_parts.push(format!("RENAME COLUMN {} TO {}", escape_pg_id(old_name), escape_pg_id(&new_column.name)));
                    }
                    sql_parts.push(format!("ALTER COLUMN {} TYPE {}", escape_pg_id(&new_column.name), new_column.data_type));
                    if new_column.nullable {
                        sql_parts.push(format!("ALTER COLUMN {} DROP NOT NULL", escape_pg_id(&new_column.name)));
                    } else {
                        sql_parts.push(format!("ALTER COLUMN {} SET NOT NULL", escape_pg_id(&new_column.name)));
                    }
                },
                TableChange::ReorderColumn { .. } => {
                    return Err(DbError::Other("PostgreSQL 暂不支持调整字段顺序".into()));
                },
                TableChange::DropColumn(name) => {
                    sql_parts.push(format!("DROP COLUMN {}", escape_pg_id(name)));
                },
                TableChange::AddIndex(idx) => {
                    let cols = idx.columns.iter().map(|c| escape_pg_id(c)).collect::<Vec<_>>().join(", ");
                    let unique = if idx.is_unique { "UNIQUE " } else { "" };
                    sql_parts.push(format!("ADD {}INDEX {} ({})", unique, escape_pg_id(&idx.name), cols));
                },
                TableChange::DropIndex(name) => {
                    // PostgreSQL DROP INDEX 是独立命令，不能放在 ALTER TABLE 中
                    debug!(name = %name, "准备删除 PostgreSQL 索引");
                },
                TableChange::AddForeignKey(fk) => {
                    sql_parts.push(format!(
                        "ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}.{} ({}) ON UPDATE {} ON DELETE {}",
                        escape_pg_id(&fk.name), escape_pg_id(&fk.column_name), escape_pg_id(schema_name), escape_pg_id(&fk.referenced_table_name), escape_pg_id(&fk.referenced_column_name),
                        fk.update_rule.as_deref().unwrap_or("NO ACTION"),
                        fk.delete_rule.as_deref().unwrap_or("NO ACTION")
                    ));
                },
                TableChange::DropForeignKey(name) => {
                    sql_parts.push(format!("DROP CONSTRAINT {}", escape_pg_id(name)));
                },
            }
        }

        // 处理 ALTER TABLE 内部变更
        if !sql_parts.is_empty() {
            let alter_sql = format!("ALTER TABLE {}.{} {}", escape_pg_id(schema_name), escape_pg_id(table), sql_parts.join(", "));
            debug!(sql = %alter_sql, "执行 PostgreSQL ALTER TABLE");
            client.batch_execute(&alter_sql).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        }

        // 处理独立的 DROP INDEX 变更
        for change in &changes {
            if let TableChange::DropIndex(name) = change {
                let drop_idx_sql = format!("DROP INDEX {}.{}", escape_pg_id(schema_name), escape_pg_id(name));
                debug!(sql = %drop_idx_sql, "执行 PostgreSQL DROP INDEX");
                client.batch_execute(&drop_idx_sql).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
            }
        }
        
        Ok(())
    }

    async fn get_table_ddl(&self, table: &str, schema: Option<&str>) -> DbResult<String> {
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let columns = self.get_table_structure(table, Some(schema_name), None).await?;
        
        let client = self.get_client_arc().await?;

        let view_sql = "SELECT pg_get_viewdef(c.oid, true) FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE n.nspname = $1 AND c.relname = $2 AND c.relkind = 'v'";
        let view_rows = client.query(view_sql, &[&schema_name, &table]).await.unwrap_or_default();
        
        if let Some(row) = view_rows.first() {
            let definition: String = row.get(0);
            return Ok(format!("CREATE OR REPLACE VIEW {}.{} AS\n{}", escape_pg_id(schema_name), escape_pg_id(table), definition));
        }

        let mut ddl = format!("CREATE TABLE {}.{} (\n", escape_pg_id(schema_name), escape_pg_id(table));
        let mut col_defs = Vec::new();
        let mut pks = Vec::new();

        for col in columns {
            let mut def = format!("  {} {}", escape_pg_id(&col.name), col.data_type);
            if !col.nullable { def.push_str(" NOT NULL"); }
            if let Some(ref d) = col.default_value { def.push_str(&format!(" DEFAULT {}", escape_string_literal(d))); }
            col_defs.push(def);
            if col.is_primary_key { pks.push(escape_pg_id(&col.name)); }
        }

        ddl.push_str(&col_defs.join(",\n"));
        if !pks.is_empty() {
            ddl.push_str(&format!(",\n  PRIMARY KEY ({})", pks.join(", ")));
        }
        ddl.push_str("\n);");

        Ok(ddl)
    }

    async fn get_view_definition(&self, view: &str, schema: Option<&str>) -> DbResult<String> {
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let sql = "
            SELECT pg_get_viewdef(c.oid, true)
            FROM pg_class c
            JOIN pg_namespace n ON n.oid = c.relnamespace
            WHERE n.nspname = $1
              AND c.relname = $2
              AND c.relkind IN ('v', 'm')
        ";
        let rows = client.query(sql, &[&schema_name, &view]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;

        if let Some(row) = rows.first() {
            let definition: String = row.get(0);
            return Ok(format!(
                "CREATE OR REPLACE VIEW {}.{} AS\n{}",
                escape_pg_id(schema_name),
                escape_pg_id(view),
                definition
            ));
        }

        Err(DbError::Other("无法获取视图定义".into()))
    }

    async fn get_functions(&self, _database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let sql = "SELECT p.proname, n.nspname, pg_catalog.pg_get_function_result(p.oid), pg_catalog.pg_get_function_arguments(p.oid), l.lanname, obj_description(p.oid, 'pg_proc') FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace JOIN pg_language l ON l.oid = p.prolang WHERE n.nspname = $1 AND p.prokind != 'a' ORDER BY p.proname";
        
        debug!(sc = %schema_name, "正在查询 PostgreSQL 函数...");
        let rows = client.query(sql, &[&schema_name]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        debug!(count = rows.len(), "已获取函数列表");
        
        Ok(rows.into_iter().map(|r| FunctionInfo { name: r.get(0), schema: Some(r.get(1)), return_type: r.try_get(2).ok(), arguments: r.try_get(3).ok(), language: r.try_get(4).ok(), function_type: "function".into(), comment: r.try_get(5).ok() }).collect())
    }

    async fn get_aggregate_functions(&self, _database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let client = self.get_client_arc().await?;
        let schema_name = schema.unwrap_or(PG_DEFAULT_SCHEMA);
        let sql = "SELECT p.proname, n.nspname, pg_catalog.pg_get_function_result(p.oid), pg_catalog.pg_get_function_arguments(p.oid), obj_description(p.oid, 'pg_proc') FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = $1 AND p.prokind = 'a' ORDER BY p.proname";
        let rows = client.query(sql, &[&schema_name]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        Ok(rows.into_iter().map(|r| FunctionInfo { name: r.get(0), schema: Some(r.get(1)), return_type: r.try_get(2).ok(), arguments: r.try_get(3).ok(), language: None, function_type: "aggregate".into(), comment: r.try_get(4).ok() }).collect())
    }

    async fn get_extensions(&self, _database: Option<&str>) -> DbResult<Vec<ExtensionInfo>> {
        let client = self.get_client_arc().await?;
        let sql = "SELECT extname, extversion, n.nspname, obj_description(e.oid, 'pg_extension') FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace ORDER BY extname";
        
        debug!("正在查询 PostgreSQL 扩展...");
        let rows = client.query(sql, &[]).await.map_err(|e| DbError::QueryFailed(Self::format_pg_error(e)))?;
        debug!(count = rows.len(), "已获取扩展列表");
        
        Ok(rows.into_iter().map(|r| ExtensionInfo { name: r.get(0), version: r.get(1), schema: Some(r.get(2)), comment: r.try_get(3).ok() }).collect())
    }

    async fn explain_query(&self, sql: &str, database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let explain_sql = format!("EXPLAIN (ANALYZE, COSTS, VERBOSE, BUFFERS, FORMAT JSON) {}", sql);
        self.execute_query(&explain_sql, database).await
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
