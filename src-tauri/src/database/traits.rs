use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub db_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: Option<String>,
    pub ssl: bool,
    pub connection_timeout: u64,
    pub pool_size: u32,
}

/// 数据库类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
    SQLite,
    MongoDB,
    Redis,
    Elasticsearch,
}

impl std::str::FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mysql" => Ok(DatabaseType::MySQL),
            "postgresql" | "postgres" => Ok(DatabaseType::PostgreSQL),
            "sqlite" => Ok(DatabaseType::SQLite),
            "mongodb" | "mongo" => Ok(DatabaseType::MongoDB),
            "redis" => Ok(DatabaseType::Redis),
            "elasticsearch" | "es" => Ok(DatabaseType::Elasticsearch),
            _ => Err(format!("不支持的数据库类型: {}", s)),
        }
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DatabaseType::MySQL => "mysql",
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::SQLite => "sqlite",
            DatabaseType::MongoDB => "mongodb",
            DatabaseType::Redis => "redis",
            DatabaseType::Elasticsearch => "elasticsearch",
        };
        write!(f, "{}", s)
    }
}

/// 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
    pub affected_rows: u64,
    pub execution_time_ms: u128,
}

/// 数据库元数据 - 数据库信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub charset: Option<String>,
    pub collation: Option<String>,
}

/// 数据库元数据 - 表信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    pub name: String,
    pub schema: Option<String>,
    pub table_type: String,
    pub engine: Option<String>,
    pub rows: Option<u64>,
    pub size_mb: Option<f64>,
    pub comment: Option<String>,
}

/// 数据库元数据 - 列信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub comment: Option<String>,
    pub character_maximum_length: Option<i64>,
    pub numeric_precision: Option<i64>,
    pub numeric_scale: Option<i64>,
}

/// 数据库元数据 - 索引信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub is_primary: bool,
    pub index_type: String,
}

/// 数据库元数据 - Schema 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub name: String,
    pub owner: Option<String>,
    pub comment: Option<String>,
}

/// 数据库元数据 - 函数信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    pub name: String,
    pub schema: Option<String>,
    pub return_type: Option<String>,
    pub arguments: Option<String>,
    pub language: Option<String>,
    pub function_type: String, // "function" 或 "aggregate"
    pub comment: Option<String>,
}

/// 数据库元数据 - 扩展信息 (PostgreSQL 专用)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub name: String,
    pub version: String,
    pub schema: Option<String>,
    pub comment: Option<String>,
}

/// 数据库操作结果
pub type DbResult<T> = Result<T, DbError>;

/// 数据库错误
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("连接失败: {0}")]
    ConnectionFailed(String),

    #[error("查询执行失败: {0}")]
    QueryFailed(String),

    #[error("不支持的数据库类型")]
    UnsupportedDatabase,

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("认证失败: {0}")]
    AuthenticationFailed(String),

    #[error("网络超时: {0}")]
    NetworkTimeout(String),

    #[error("会话不存在: {0}")]
    SessionNotFound(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("其他错误: {0}")]
    Other(String),
}

impl Serialize for DbError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// 数据库操作 Trait
#[async_trait]
pub trait DatabaseOperations: Send + Sync {
    /// 测试连接
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool>;

    /// 连接数据库 - 改为 &self 以支持 Arc 共享
    async fn connect(&self, config: ConnectionConfig) -> DbResult<()>;

    /// 断开连接 - 改为 &self
    async fn disconnect(&self) -> DbResult<()>;

    /// 执行查询 - 支持多结果集
    async fn execute_query(&self, sql: &str, database: Option<&str>) -> DbResult<Vec<QueryResult>>;

    /// 获取数据库列表
    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>>;

    /// 获取表列表
    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>>;

    /// 获取表结构
    async fn get_table_structure(&self, table: &str, schema: Option<&str>, database: Option<&str>) -> DbResult<Vec<ColumnInfo>>;

    /// 更新数据
    async fn update_data(&self, _table: &str, _schema: Option<&str>, _column: &str, _value: Option<String>, _where_conditions: HashMap<String, serde_json::Value>) -> DbResult<()> {
        Err(DbError::Other("该数据库类型不支持此更新操作".into()))
    }

    /// 删除数据
    async fn delete_data(&self, _table: &str, _schema: Option<&str>, _where_conditions: HashMap<String, serde_json::Value>) -> DbResult<()> {
        Err(DbError::Other("该数据库类型不支持此删除操作".into()))
    }

    /// 获取索引信息
    async fn get_indexes(&self, table: &str, schema: Option<&str>) -> DbResult<Vec<IndexInfo>>;

    /// 获取 Schema 下的所有索引
    async fn get_schema_indexes(&self, _database: Option<&str>, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        Ok(Vec::new())
    }
    
    /// 获取表/视图的 DDL (CREATE 语句)
    async fn get_table_ddl(&self, _table: &str, _schema: Option<&str>) -> DbResult<String> {
        Err(DbError::Other("该数据库类型不支持 DDL 生成".into()))
    }
    
    async fn get_views(&self, _database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        Ok(Vec::new())
    }
    
    /// 切换数据库 - 改为 &self
    async fn switch_database(&self, _database: &str) -> DbResult<()> {
        Ok(())
    }
    
    /// 获取执行计划
    async fn explain_query(&self, _sql: &str, _database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        Err(DbError::Other("该数据库类型不支持执行计划分析".into()))
    }
    
    fn as_any(&self) -> &dyn std::any::Any;

    async fn get_schemas(&self, _database: Option<&str>) -> DbResult<Vec<SchemaInfo>> {
        Ok(Vec::new())
    }

    async fn get_functions(&self, _database: Option<&str>, _schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        Ok(Vec::new())
    }

    async fn get_aggregate_functions(&self, _database: Option<&str>, _schema: Option<&str>) -> DbResult<Vec<FunctionInfo>> {
        Ok(Vec::new())
    }

    async fn get_extensions(&self, _database: Option<&str>) -> DbResult<Vec<ExtensionInfo>> {
        Ok(Vec::new())
    }
}
