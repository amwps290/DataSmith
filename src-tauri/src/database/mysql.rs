use async_trait::async_trait;
use std::collections::HashMap;
use std::time::Instant;
use sqlx::{MySqlPool, mysql::MySqlPoolOptions, Row, Column, ValueRef};
use tokio::sync::Mutex;
use tracing::{info, instrument};

use super::traits::*;

/// MySQL 数据库驱动状态
struct MySqlState {
    pool: Option<MySqlPool>,
    config: Option<ConnectionConfig>,
}

/// MySQL 数据库驱动 - 基于 sqlx 的实现 (内部可变性)
pub struct MySqlDatabase {
    state: Mutex<MySqlState>,
}

impl MySqlDatabase {
    pub fn new() -> Self {
        Self { 
            state: Mutex::new(MySqlState { pool: None, config: None })
        }
    }

    async fn create_pool(config: &ConnectionConfig) -> DbResult<MySqlPool> {
        let db_name = config.database.as_deref().unwrap_or("");
        let url = format!(
            "mysql://{}:{}@{}:{}/{}",
            config.username, config.password, config.host, config.port, db_name
        );

        MySqlPoolOptions::new()
            .max_connections(config.pool_size)
            .acquire_timeout(std::time::Duration::from_secs(config.connection_timeout))
            .connect(&url)
            .await
            .map_err(|e| DbError::ConnectionFailed(e.to_string()))
    }
}

#[async_trait]
impl DatabaseOperations for MySqlDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let pool = Self::create_pool(config).await?;
        sqlx::query("SELECT 1").execute(&pool).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(true)
    }

    async fn connect(&self, config: ConnectionConfig) -> DbResult<()> {
        let pool = Self::create_pool(&config).await?;
        let mut state = self.state.lock().await;
        state.pool = Some(pool);
        state.config = Some(config);
        Ok(())
    }

    async fn disconnect(&self) -> DbResult<()> {
        let mut state = self.state.lock().await;
        state.pool = None;
        state.config = None;
        Ok(())
    }

    async fn switch_database(&self, database: &str) -> DbResult<()> {
        let mut state = self.state.lock().await;
        let mut config = state.config.clone().ok_or(DbError::Other("未找到初始配置".into()))?;
        
        if config.database.as_deref() == Some(database) {
            return Ok(());
        }
        
        info!(new_db = %database, "MySQL 正在切换数据库...");
        config.database = Some(database.to_string());
        
        let pool = Self::create_pool(&config).await?;
        state.pool = Some(pool);
        state.config = Some(config);
        Ok(())
    }

    #[instrument(skip(self, sql))]
    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let start = Instant::now();
        let state = self.state.lock().await;
        let pool = state.pool.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;

        // MySQL 通过 sqlx fetch_all 目前一次只返回一个结果集
        // 如果要支持多结果集，后续可以调研 fetch_many
        let rows = sqlx::query(sql)
            .fetch_all(pool)
            .await
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut columns = Vec::new();
        let mut final_rows = Vec::new();

        if !rows.is_empty() {
            columns = rows[0].columns().iter().map(|c| c.name().to_string()).collect();
            for row in rows {
                let mut row_map = HashMap::new();
                for col in row.columns() {
                    let name = col.name();
                    let value: serde_json::Value = match row.try_get_raw(name) {
                        Ok(v) if v.is_null() => serde_json::Value::Null,
                        _ => {
                            row.try_get::<String, _>(name).map(serde_json::Value::String).unwrap_or(serde_json::Value::Null)
                        }
                    };
                    row_map.insert(name.to_string(), value);
                }
                final_rows.push(row_map);
            }
        }

        Ok(vec![QueryResult {
            columns,
            rows: final_rows,
            affected_rows: 0,
            execution_time_ms: start.elapsed().as_millis(),
        }])
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let state = self.state.lock().await;
        let pool = state.pool.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let rows = sqlx::query("SHOW DATABASES").fetch_all(pool).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| DatabaseInfo { name: r.get(0), charset: None, collation: None }).collect())
    }

    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let state = self.state.lock().await;
        let pool = state.pool.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let sql = if let Some(db) = database {
            format!("SELECT TABLE_NAME, TABLE_COMMENT, TABLE_ROWS, DATA_LENGTH FROM information_schema.TABLES WHERE TABLE_SCHEMA = '{}'", db)
        } else {
            "SHOW TABLES".to_string()
        };
        
        let rows = sqlx::query(&sql).fetch_all(pool).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| TableInfo {
            name: r.get(0),
            schema: None,
            table_type: "TABLE".into(),
            engine: None,
            rows: r.try_get(2).ok(),
            size_mb: r.try_get::<i64, _>(3).ok().map(|s| s as f64 / 1024.0 / 1024.0),
            comment: r.try_get(1).ok(),
        }).collect())
    }

    async fn get_table_structure(&self, table: &str, _schema: Option<&str>, database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let state = self.state.lock().await;
        let pool = state.pool.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let schema = database.unwrap_or("");
        let sql = format!("SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT, COLUMN_KEY, EXTRA, COLUMN_COMMENT FROM information_schema.COLUMNS WHERE TABLE_NAME = '{}' AND TABLE_SCHEMA = '{}' ORDER BY ORDINAL_POSITION", table, schema);
        
        let rows = sqlx::query(&sql).fetch_all(pool).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(rows.into_iter().map(|r| {
            let key: String = r.get(4);
            ColumnInfo {
                name: r.get(0),
                data_type: r.get(1),
                nullable: r.get::<String, _>(2) == "YES",
                default_value: r.try_get(3).ok(),
                is_primary_key: key == "PRI",
                is_auto_increment: r.get::<String, _>(5).contains("auto_increment"),
                comment: r.try_get(6).ok(),
                character_maximum_length: None,
                numeric_precision: None,
                numeric_scale: None,
            }
        }).collect())
    }

    async fn get_indexes(&self, table: &str, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let state = self.state.lock().await;
        let pool = state.pool.as_ref().ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;
        let rows = sqlx::query(&format!("SHOW INDEX FROM {}", table)).fetch_all(pool).await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        
        let mut map: HashMap<String, IndexInfo> = HashMap::new();
        for r in rows {
            let name: String = r.get(2);
            let col: String = r.get(4);
            let is_unique: i64 = r.get(1);
            let entry = map.entry(name.clone()).or_insert(IndexInfo {
                name,
                columns: vec![],
                is_unique: is_unique == 0,
                is_primary: r.get::<String, _>(2) == "PRIMARY",
                index_type: r.get(10),
            });
            entry.columns.push(col);
        }
        Ok(map.into_values().collect())
    }

    async fn explain_query(&self, sql: &str, database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let explain_sql = format!("EXPLAIN {}", sql);
        self.execute_query(&explain_sql, database).await
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
