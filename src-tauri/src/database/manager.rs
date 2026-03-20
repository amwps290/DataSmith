use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::traits::*;
#[cfg(feature = "mysql")]
use super::mysql::MySqlDatabase;

/// 数据库会话管理器 - 支持多连接隔离
pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, Box<dyn DatabaseOperations>>>>,
    connection_types: Arc<RwLock<HashMap<String, DatabaseType>>>,
    configs: Arc<RwLock<HashMap<String, ConnectionConfig>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_types: Arc::new(RwLock::new(HashMap::new())),
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn ensure_session(
        &self,
        config_id: &str,
        session_id: &str,
    ) -> DbResult<String> {
        let composite_id = format!("{}:{}", config_id, session_id);
        
        {
            let connections = self.connections.read().await;
            if connections.contains_key(&composite_id) {
                return Ok(composite_id);
            }
        }

        println!("[MGR-DEBUG] 正在为配置 {} 创建新会话: {}", config_id, session_id);

        let config = {
            let configs = self.configs.read().await;
            configs.get(config_id).cloned().ok_or_else(|| {
                println!("[MGR-ERROR] 找不到配置 ID: {}", config_id);
                DbError::ConnectionFailed(format!("配置 {} 不存在", config_id))
            })?
        };

        let mut db: Box<dyn DatabaseOperations> = self.create_instance(&config.db_type)?;
        
        println!("[MGR-DEBUG] 正在调用驱动 connect()...");
        if let Err(e) = db.connect(config.clone()).await {
            println!("[MGR-ERROR] 驱动连接失败: {}", e);
            return Err(e);
        }

        let mut connections = self.connections.write().await;
        connections.insert(composite_id.clone(), db);
        
        let mut connection_types = self.connection_types.write().await;
        connection_types.insert(composite_id.clone(), config.db_type.clone());

        println!("[MGR-DEBUG] 会话 {} 创建并存储成功", composite_id);
        Ok(composite_id)
    }

    pub async fn create_connection(
        &self,
        config: ConnectionConfig,
    ) -> DbResult<String> {
        let config_id = config.id.clone();
        println!("[MGR-DEBUG] 收到连接请求，保存配置: ID={}, Name={}", config_id, config.name);
        
        {
            let mut configs = self.configs.write().await;
            configs.insert(config_id.clone(), config.clone());
        }

        self.ensure_session(&config_id, "metadata").await
    }

    fn create_instance(&self, db_type: &DatabaseType) -> DbResult<Box<dyn DatabaseOperations>> {
        match db_type {
            #[cfg(feature = "mysql")]
            DatabaseType::MySQL => Ok(Box::new(MySqlDatabase::new())),
            
            #[cfg(feature = "postgresql")]
            DatabaseType::PostgreSQL => Ok(Box::new(super::postgresql::PostgreSqlDatabase::new())),
            
            #[cfg(feature = "sqlite")]
            DatabaseType::SQLite => Ok(Box::new(super::sqlite::SqliteDatabase::new())),
            
            #[cfg(feature = "mongodb-support")]
            DatabaseType::MongoDB => Ok(Box::new(super::mongodb::MongoDatabase::new())),
            
            #[cfg(feature = "redis-support")]
            DatabaseType::Redis => Ok(Box::new(super::redis::RedisDatabase::new())),
            
            _ => {
                println!("[MGR-ERROR] 不支持的数据库类型: {:?}", db_type);
                Err(DbError::UnsupportedDatabase)
            },
        }
    }

    pub async fn get_db_instance(&self, composite_id: &str) -> DbResult<Arc<RwLock<HashMap<String, Box<dyn DatabaseOperations>>>>> {
        let target_id = if !composite_id.contains(':') {
            format!("{}:metadata", composite_id)
        } else {
            composite_id.to_string()
        };

        let connections = self.connections.read().await;
        if connections.contains_key(&target_id) {
            Ok(self.connections.clone())
        } else {
            if let Some(config_id) = composite_id.split(':').next() {
                println!("[MGR-DEBUG] 会话 {} 不存在，尝试自动恢复...", target_id);
                drop(connections);
                let sid = composite_id.split(':').nth(1).unwrap_or("metadata");
                match self.ensure_session(config_id, sid).await {
                    Ok(_) => Ok(self.connections.clone()),
                    Err(e) => {
                        println!("[MGR-ERROR] 自动恢复会话失败: {}", e);
                        Err(DbError::ConnectionFailed("会话无法恢复".into()))
                    }
                }
            } else {
                Err(DbError::ConnectionFailed("连接不存在".into()))
            }
        }
    }

    pub async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        println!("[MGR-DEBUG] 测试连接配置: {:?}", config.id);
        let db = self.create_instance(&config.db_type)?;
        db.test_connection(config).await
    }

    pub async fn disconnect(&self, config_id: &str) -> DbResult<()> {
        println!("[MGR-DEBUG] 正在断开配置 {} 下的所有会话", config_id);
        let mut connections = self.connections.write().await;
        let mut connection_types = self.connection_types.write().await;
        let mut configs = self.configs.write().await;

        let prefix = format!("{}:", config_id);
        connections.retain(|k, _| !k.starts_with(&prefix) && k != config_id);
        connection_types.retain(|k, _| !k.starts_with(&prefix) && k != config_id);
        configs.remove(config_id);
        
        Ok(())
    }

    pub async fn execute_query(&self, composite_id: &str, sql: &str, database: Option<&str>) -> DbResult<QueryResult> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let connections = self.connections.read().await;
        let db = connections.get(&real_id).ok_or_else(|| {
            println!("[MGR-ERROR] 执行查询失败：会话 {} 不存在", real_id);
            DbError::ConnectionFailed("会话不存在".into())
        })?;
        db.execute_query(sql, database).await
    }

    pub async fn get_databases(&self, composite_id: &str) -> DbResult<Vec<DatabaseInfo>> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let connections = self.connections.read().await;
        let db = connections.get(&real_id).ok_or(DbError::ConnectionFailed("会话不存在".into()))?;
        db.get_databases().await
    }

    pub async fn get_tables(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let connections = self.connections.read().await;
        let db = connections.get(&real_id).ok_or(DbError::ConnectionFailed("会话不存在".into()))?;
        db.get_tables(database).await
    }

    pub async fn get_views(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let connections = self.connections.read().await;
        let db = connections.get(&real_id).ok_or(DbError::ConnectionFailed("会话不存在".into()))?;
        db.get_views(database).await
    }

    pub async fn get_table_structure(&self, composite_id: &str, table: &str, schema: Option<&str>, database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let connections = self.connections.read().await;
        let db = connections.get(&real_id).ok_or(DbError::ConnectionFailed("会话不存在".into()))?;
        db.get_table_structure(table, schema, database).await
    }

    pub async fn get_database_type(&self, composite_id: &str) -> DbResult<DatabaseType> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let types = self.connection_types.read().await;
        types.get(&real_id).cloned().ok_or(DbError::ConnectionFailed("连接不存在".into()))
    }

    pub async fn get_schemas(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<SchemaInfo>> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let connections = self.connections.read().await;
        let db = connections.get(&real_id).ok_or(DbError::ConnectionFailed("会话不存在".into()))?;
        db.get_schemas(database).await
    }

    pub async fn get_functions(&self, composite_id: &str, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let connections = self.connections.read().await;
        let db = connections.get(&real_id).ok_or(DbError::ConnectionFailed("会话不存在".into()))?;
        db.get_functions(database, schema).await
    }

    pub async fn get_aggregate_functions(&self, composite_id: &str, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let real_id = if !composite_id.contains(':') { format!("{}:metadata", composite_id) } else { composite_id.to_string() };
        let connections = self.connections.read().await;
        let db = connections.get(&real_id).ok_or(DbError::ConnectionFailed("会话不存在".into()))?;
        db.get_aggregate_functions(database, schema).await
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
