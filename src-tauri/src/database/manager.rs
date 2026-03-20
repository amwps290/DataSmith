use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

use super::traits::*;
#[cfg(feature = "mysql")]
use super::mysql::MySqlDatabase;

/// 数据库会话管理器 - 支持多连接隔离与日志追踪
pub struct ConnectionManager {
    /// Key 为复合 ID: "config_id:session_id"
    connections: Arc<RwLock<HashMap<String, Box<dyn DatabaseOperations>>>>,
    /// 存储连接 ID 到数据库类型的映射
    connection_types: Arc<RwLock<HashMap<String, DatabaseType>>>,
    /// 存储原始配置，用于按需创建新会话
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

    /// 确保特定会话的连接已建立
    #[instrument(skip(self, config_id, session_id))]
    pub async fn ensure_session(
        &self,
        config_id: &str,
        session_id: &str,
    ) -> DbResult<String> {
        let composite_id = format!("{}:{}", config_id, session_id);
        
        // 1. 检查是否存在
        if self.connections.read().await.contains_key(&composite_id) {
            debug!(session = %composite_id, "会话已存在，复用连接");
            return Ok(composite_id);
        }

        info!(session = %composite_id, "正在初始化新会话连接");

        // 2. 获取原始配置
        let config = self.configs.read().await.get(config_id).cloned().ok_or_else(|| {
            error!(config_id = %config_id, "无法创建会话：未找到对应的数据库配置");
            DbError::ConfigError(format!("配置 {} 不存在", config_id))
        })?;

        // 3. 创建驱动实例并连接
        let mut db = self.create_instance(&config.db_type)?;
        db.connect(config.clone()).await?;

        // 4. 注册会话
        self.connections.write().await.insert(composite_id.clone(), db);
        self.connection_types.write().await.insert(composite_id.clone(), config.db_type.clone());

        info!(session = %composite_id, "会话连接已成功建立并注册");
        Ok(composite_id)
    }

    /// 创建初始连接（通常为元数据会话）
    #[instrument(skip(self, config))]
    pub async fn create_connection(
        &self,
        config: ConnectionConfig,
    ) -> DbResult<String> {
        let config_id = config.id.clone();
        debug!(config_id = %config_id, name = %config.name, "收到新连接注册请求");
        
        self.configs.write().await.insert(config_id.clone(), config.clone());
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
            _ => Err(DbError::UnsupportedDatabase),
        }
    }

    /// 获取数据库实例引用的包装锁
    #[instrument(skip(self))]
    pub async fn get_db_instance(&self, composite_id: &str) -> DbResult<Arc<RwLock<HashMap<String, Box<dyn DatabaseOperations>>>>> {
        let target_id = if !composite_id.contains(':') {
            format!("{}:metadata", composite_id)
        } else {
            composite_id.to_string()
        };

        if self.connections.read().await.contains_key(&target_id) {
            Ok(self.connections.clone())
        } else {
            // 自动恢复会话逻辑
            if let Some(config_id) = composite_id.split(':').next() {
                debug!(session = %target_id, "尝试自动恢复丢失的会话");
                let sid = composite_id.split(':').nth(1).unwrap_or("metadata");
                self.ensure_session(config_id, sid).await?;
                Ok(self.connections.clone())
            } else {
                Err(DbError::SessionNotFound(composite_id.to_string()))
            }
        }
    }

    pub async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let db = self.create_instance(&config.db_type)?;
        db.test_connection(config).await
    }

    #[instrument(skip(self))]
    pub async fn disconnect(&self, config_id: &str) -> DbResult<()> {
        info!(config_id = %config_id, "正在彻底关闭配置下的所有活跃会话");
        let mut connections = self.connections.write().await;
        let mut connection_types = self.connection_types.write().await;
        let mut configs = self.configs.write().await;

        let prefix = format!("{}:", config_id);
        connections.retain(|k, _| !k.starts_with(&prefix) && k != config_id);
        connection_types.retain(|k, _| !k.starts_with(&prefix) && k != config_id);
        configs.remove(config_id);
        
        Ok(())
    }

    // --- 代理方法，支持日志追踪 ---

    #[instrument(skip(self, sql))]
    pub async fn execute_query(&self, composite_id: &str, sql: &str, database: Option<&str>) -> DbResult<QueryResult> {
        let id = self.resolve_id(composite_id);
        let conns = self.connections.read().await;
        let db = conns.get(&id).ok_or_else(|| DbError::SessionNotFound(id.clone()))?;
        db.execute_query(sql, database).await
    }

    pub async fn get_databases(&self, composite_id: &str) -> DbResult<Vec<DatabaseInfo>> {
        let id = self.resolve_id(composite_id);
        self.connections.read().await.get(&id).ok_or_else(|| DbError::SessionNotFound(id))?.get_databases().await
    }

    pub async fn get_tables(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let id = self.resolve_id(composite_id);
        self.connections.read().await.get(&id).ok_or_else(|| DbError::SessionNotFound(id))?.get_tables(database).await
    }

    pub async fn get_views(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let id = self.resolve_id(composite_id);
        self.connections.read().await.get(&id).ok_or_else(|| DbError::SessionNotFound(id))?.get_views(database).await
    }

    pub async fn get_table_structure(&self, composite_id: &str, table: &str, schema: Option<&str>, database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let id = self.resolve_id(composite_id);
        self.connections.read().await.get(&id).ok_or_else(|| DbError::SessionNotFound(id))?.get_table_structure(table, schema, database).await
    }

    pub async fn get_database_type(&self, composite_id: &str) -> DbResult<DatabaseType> {
        let id = self.resolve_id(composite_id);
        self.connection_types.read().await.get(&id).cloned().ok_or_else(|| DbError::SessionNotFound(id))
    }

    pub async fn get_schemas(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<SchemaInfo>> {
        let id = self.resolve_id(composite_id);
        self.connections.read().await.get(&id).ok_or_else(|| DbError::SessionNotFound(id))?.get_schemas(database).await
    }

    pub async fn get_functions(&self, composite_id: &str, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let id = self.resolve_id(composite_id);
        self.connections.read().await.get(&id).ok_or_else(|| DbError::SessionNotFound(id))?.get_functions(database, schema).await
    }

    pub async fn get_aggregate_functions(&self, composite_id: &str, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let id = self.resolve_id(composite_id);
        self.connections.read().await.get(&id).ok_or_else(|| DbError::SessionNotFound(id))?.get_aggregate_functions(database, schema).await
    }

    fn resolve_id(&self, id: &str) -> String {
        if id.contains(':') { id.to_string() } else { format!("{}:metadata", id) }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
