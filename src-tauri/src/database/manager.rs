use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

use super::traits::*;
#[cfg(feature = "mysql")]
use super::mysql::MySqlDatabase;

/// 数据库会话管理器 - 支持多连接隔离与极致并发
pub struct ConnectionManager {
    /// 使用 Arc 包裹驱动实例，允许在释放 Map 锁后继续持有驱动
    connections: Arc<RwLock<HashMap<String, Arc<dyn DatabaseOperations>>>>,
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

    /// 核心：解析 ID 并获取驱动实例，获取后立即释放锁
    async fn get_db_ref(&self, composite_id: &str) -> DbResult<Arc<dyn DatabaseOperations>> {
        let real_id = if composite_id.contains(':') {
            composite_id.to_string()
        } else {
            format!("{}:metadata", composite_id)
        };

        // 1. 尝试直接获取已存在的连接
        {
            let conns = self.connections.read().await;
            if let Some(db) = conns.get(&real_id) {
                return Ok(db.clone());
            }
        }

        // 2. 如果不存在，触发创建流程 (ensure_session 内部会处理锁)
        debug!(session = %real_id, "会话不存在，触发自动创建流程");
        let config_id = real_id.split(':').next().unwrap_or(composite_id);
        let session_id = real_id.split(':').nth(1).unwrap_or("metadata");
        
        self.ensure_session(config_id, session_id).await?;
        
        // 3. 再次获取（此时肯定存在了）
        let conns = self.connections.read().await;
        conns.get(&real_id).cloned().ok_or_else(|| DbError::SessionNotFound(real_id))
    }

    #[instrument(skip(self, config_id, session_id))]
    pub async fn ensure_session(
        &self,
        config_id: &str,
        session_id: &str,
    ) -> DbResult<String> {
        let composite_id = format!("{}:{}", config_id, session_id);
        
        // 双重检查锁定模式
        if self.connections.read().await.contains_key(&composite_id) {
            return Ok(composite_id);
        }

        let config = self.configs.read().await.get(config_id).cloned().ok_or_else(|| {
            error!(config_id = %config_id, "未找到配置，请确保已保存连接");
            DbError::ConfigError(format!("配置 {} 不存在", config_id))
        })?;

        let mut db = self.create_instance(&config.db_type)?;
        
        debug!(session = %composite_id, "正在发起驱动层连接...");
        db.connect(config.clone()).await?;

        let mut conns = self.connections.write().await;
        conns.insert(composite_id.clone(), Arc::from(db));
        
        self.connection_types.write().await.insert(composite_id.clone(), config.db_type.clone());

        info!(session = %composite_id, "物理连接已建立并存入管理器");
        Ok(composite_id)
    }

    pub async fn create_connection(
        &self,
        config: ConnectionConfig,
    ) -> DbResult<String> {
        let config_id = config.id.clone();
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

    pub async fn get_db_instance(&self, composite_id: &str) -> DbResult<Arc<RwLock<HashMap<String, Arc<dyn DatabaseOperations>>>>> {
        self.get_db_ref(composite_id).await?;
        Ok(self.connections.clone())
    }

    pub async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let db = self.create_instance(&config.db_type)?;
        db.test_connection(config).await
    }

    pub async fn disconnect(&self, config_id: &str) -> DbResult<()> {
        let mut conns = self.connections.write().await;
        let prefix = format!("{}:", config_id);
        conns.retain(|k, _| !k.starts_with(&prefix) && k != config_id);
        
        self.connection_types.write().await.retain(|k, _| !k.starts_with(&prefix) && k != config_id);
        self.configs.write().await.remove(config_id);
        Ok(())
    }

    // --- 代理方法：转发给具体驱动 ---

    pub async fn execute_query(&self, composite_id: &str, sql: &str, database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let db = self.get_db_ref(composite_id).await?;
        
        if let Some(db_name) = database {
            let mut conns = self.connections.write().await;
            let real_id = if composite_id.contains(':') { composite_id.to_string() } else { format!("{}:metadata", composite_id) };
            if let Some(db_instance) = conns.get_mut(&real_id) {
                db_instance.switch_database(db_name).await?;
            }
        }

        db.execute_query(sql, database).await
    }

    pub async fn get_databases(&self, composite_id: &str) -> DbResult<Vec<DatabaseInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_databases().await
    }

    pub async fn get_tables(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_tables(database).await
    }

    pub async fn get_views(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_views(database).await
    }

    pub async fn get_table_structure(&self, composite_id: &str, table: &str, schema: Option<&str>, database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_table_structure(table, schema, database).await
    }

    pub async fn get_database_type(&self, composite_id: &str) -> DbResult<DatabaseType> {
        let real_id = if composite_id.contains(':') { composite_id.to_string() } else { format!("{}:metadata", composite_id) };
        let config_id = real_id.split(':').next().unwrap_or(composite_id);
        
        if let Some(t) = self.connection_types.read().await.get(&real_id) {
            return Ok(t.clone());
        }
        
        self.configs.read().await.get(config_id).map(|c| c.db_type.clone()).ok_or(DbError::SessionNotFound(real_id))
    }

    pub async fn get_schemas(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<SchemaInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_schemas(database).await
    }

    pub async fn get_functions(&self, composite_id: &str, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_functions(database, schema).await
    }

    pub async fn get_indexes(&self, composite_id: &str, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_indexes(table, schema).await
    }

    pub async fn get_aggregate_functions(&self, composite_id: &str, database: Option<&str>, schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_aggregate_functions(database, schema).await
    }

    pub async fn get_extensions(&self, composite_id: &str, database: Option<&str>) -> DbResult<Vec<ExtensionInfo>> {
        let db = self.get_db_ref(composite_id).await?;
        db.get_extensions(database).await
    }

    pub async fn explain_query(&self, composite_id: &str, sql: &str, database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let db = self.get_db_ref(composite_id).await?;
        db.explain_query(sql, database).await
    }
}

impl Default for ConnectionManager {
    fn default() -> Self { Self::new() }
}
