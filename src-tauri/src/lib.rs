pub mod commands;
pub mod database;
pub mod models;
pub mod utils;

use database::ConnectionManager;
use std::sync::Arc;

/// 应用状态 - 移除全局 Mutex，因为 ConnectionManager 内部已实现细粒度锁
pub struct AppState {
    pub connection_manager: Arc<ConnectionManager>,
}
