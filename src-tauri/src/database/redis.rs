use async_trait::async_trait;
use std::collections::HashMap;
use redis::Client;
use tokio::sync::Mutex;
use tracing::instrument;

use super::traits::*;

struct RedisState {
    client: Option<Client>,
    config: Option<ConnectionConfig>,
}

pub struct RedisDatabase {
    state: Mutex<RedisState>,
}

impl RedisDatabase {
    pub fn new() -> Self {
        Self { 
            state: Mutex::new(RedisState { client: None, config: None })
        }
    }

    async fn create_client(config: &ConnectionConfig) -> DbResult<Client> {
        let url = format!("redis://{}:{}", config.host, config.port);
        Client::open(url).map_err(|e| DbError::ConnectionFailed(e.to_string()))
    }

    pub async fn execute_command(&self, cmd: &str, args: Vec<String>) -> DbResult<redis::Value> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接".into()))?;
        let mut conn = client.get_multiplexed_async_connection().await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        
        let mut command = redis::cmd(cmd);
        for arg in args { command.arg(arg); }
        
        command.query_async(&mut conn).await.map_err(|e| DbError::QueryFailed(e.to_string()))
    }

    pub async fn get_server_info(&self) -> DbResult<HashMap<String, String>> {
        let val = self.execute_command("INFO", vec![]).await?;
        if let redis::Value::BulkString(bytes) = val {
            let info_str = String::from_utf8_lossy(&bytes);
            let mut map = HashMap::new();
            for line in info_str.lines() {
                if line.contains(':') {
                    let parts: Vec<&str> = line.splitn(2, ':').collect();
                    map.insert(parts[0].to_string(), parts[1].to_string());
                }
            }
            return Ok(map);
        }
        Ok(HashMap::new())
    }

    pub async fn get_key_ttl(&self, key: &str) -> DbResult<i64> {
        let val = self.execute_command("TTL", vec![key.to_string()]).await?;
        match val { redis::Value::Int(i) => Ok(i), _ => Ok(-1) }
    }

    pub async fn get_key_value(&self, key: &str) -> DbResult<redis::Value> {
        self.execute_command("GET", vec![key.to_string()]).await
    }

    pub async fn set_key_value(&self, key: &str, value: &str, ttl: Option<u64>) -> DbResult<()> {
        self.execute_command("SET", vec![key.to_string(), value.to_string()]).await?;
        if let Some(t) = ttl { self.execute_command("EXPIRE", vec![key.to_string(), t.to_string()]).await?; }
        Ok(())
    }

    pub async fn delete_key(&self, key: &str) -> DbResult<()> {
        self.execute_command("DEL", vec![key.to_string()]).await?;
        Ok(())
    }
}

#[async_trait]
impl DatabaseOperations for RedisDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let client = Self::create_client(config).await?;
        let mut conn = client.get_multiplexed_async_connection().await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        let _: String = redis::cmd("PING").query_async(&mut conn).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
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

    #[instrument(skip(self, _sql))]
    async fn execute_query(&self, _sql: &str, _database: Option<&str>) -> DbResult<QueryResult> {
        Err(DbError::Other("Redis 暂不支持直接执行 SQL".into()))
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let _info = self.get_server_info().await?;
        let mut dbs = Vec::new();
        for i in 0..16 { dbs.push(DatabaseInfo { name: format!("db{}", i), charset: None, collation: None }); }
        Ok(dbs)
    }

    async fn get_tables(&self, _database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        Ok(vec![])
    }

    async fn get_table_structure(&self, _table: &str, _schema: Option<&str>, _database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        Ok(vec![])
    }

    async fn get_indexes(&self, _table: &str, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        Ok(vec![])
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
