use async_trait::async_trait;
use mongodb::{Client, options::ClientOptions};
use tokio::sync::Mutex;
use tracing::instrument;

use super::traits::*;

struct MongoState {
    client: Option<Client>,
    config: Option<ConnectionConfig>,
}

pub struct MongoDatabase {
    state: Mutex<MongoState>,
}

impl MongoDatabase {
    pub fn new() -> Self {
        Self { 
            state: Mutex::new(MongoState { client: None, config: None })
        }
    }

    async fn create_client(config: &ConnectionConfig) -> DbResult<Client> {
        let url = format!("mongodb://{}:{}", config.host, config.port);
        let mut client_options = ClientOptions::parse(&url).await.map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        client_options.app_name = Some("DataSmith".to_string());
        Client::with_options(client_options).map_err(|e| DbError::ConnectionFailed(e.to_string()))
    }
}

#[async_trait]
impl DatabaseOperations for MongoDatabase {
    async fn test_connection(&self, config: &ConnectionConfig) -> DbResult<bool> {
        let client = Self::create_client(config).await?;
        client.list_database_names().await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
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
        Err(DbError::Other("MongoDB 暂不支持直接执行 SQL".into()))
    }

    async fn get_databases(&self) -> DbResult<Vec<DatabaseInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接".into()))?;
        let names = client.list_database_names().await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(names.into_iter().map(|n| DatabaseInfo { name: n, charset: None, collation: None }).collect())
    }

    async fn get_tables(&self, database: Option<&str>) -> DbResult<Vec<TableInfo>> {
        let state = self.state.lock().await;
        let client = state.client.as_ref().ok_or(DbError::ConnectionFailed("未连接".into()))?;
        let db_name = database.ok_or(DbError::Other("未指定数据库".into()))?;
        let names = client.database(db_name).list_collection_names().await.map_err(|e| DbError::QueryFailed(e.to_string()))?;
        Ok(names.into_iter().map(|n| TableInfo { name: n, schema: None, table_type: "COLLECTION".into(), engine: None, rows: None, size_mb: None, comment: None }).collect())
    }

    async fn get_table_structure(&self, _table: &str, _schema: Option<&str>, _database: Option<&str>) -> DbResult<Vec<ColumnInfo>> {
        Ok(vec![])
    }

    async fn get_indexes(&self, _table: &str, _schema: Option<&str>) -> DbResult<Vec<IndexInfo>> {
        Ok(vec![])
    }

    fn as_any(&self) -> &dyn std::any::Any { self }
}
