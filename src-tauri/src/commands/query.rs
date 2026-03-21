use crate::database::QueryResult;
use crate::utils::sql_formatter::SqlFormatter;
use crate::AppState;
use tauri::State;
use tracing::{info, instrument};

/// 格式化 SQL
#[tauri::command]
#[instrument(skip(state, sql))]
pub async fn beautify_sql(
    connection_id: String,
    sql: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = &state.connection_manager;
    
    let db_type = manager
        .get_database_type(&connection_id)
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(SqlFormatter::beautify(&sql, &db_type))
}

/// 执行 SQL 查询
#[tauri::command]
#[instrument(skip(state, sql))]
pub async fn execute_query(
    connection_id: String,
    sql: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = &state.connection_manager;
    
    info!(sql = %sql.replace('\n', " ").trim(), "收到执行请求");
    
    let result = manager
        .execute_query(&connection_id, &sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(result)
}

/// 获取 SQL 执行计划
#[tauri::command]
#[instrument(skip(state, sql))]
pub async fn explain_query(
    connection_id: String,
    sql: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = &state.connection_manager;
    
    info!(sql = %sql.replace('\n', " ").trim(), "收到解释请求");
    
    let result = manager
        .explain_query(&connection_id, &sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(result)
}

/// 分页执行 SQL 查询
#[tauri::command]
#[instrument(skip(state, sql))]
pub async fn execute_query_paged(
    connection_id: String,
    sql: String,
    database: Option<String>,
    page: u32,
    page_size: u32,
    state: State<'_, AppState>,
) -> Result<QueryResult, String> {
    let manager = &state.connection_manager;
    
    // 目前简单实现：直接转发给 execute_query，后续可扩展真正的数据库分页
    manager
        .execute_query(&connection_id, &sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_query_batch(
    connection_id: String,
    sqls: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<QueryResult>, String> {
    let manager = &state.connection_manager;
    let mut results = Vec::new();
    for sql in sqls {
        let res = manager.execute_query(&connection_id, &sql, None).await.map_err(|e| e.to_string())?;
        results.push(res);
    }
    Ok(results)
}

#[tauri::command]
pub async fn update_table_data(
    connection_id: String,
    database: String,
    table: String,
    primary_key: String,
    pk_value: serde_json::Value,
    data: HashMap<String, serde_json::Value>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 逻辑实现...
    Ok(())
}

#[tauri::command]
pub async fn insert_table_data(
    connection_id: String,
    database: String,
    table: String,
    data: HashMap<String, serde_json::Value>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn delete_table_data(
    connection_id: String,
    database: String,
    table: String,
    primary_key: String,
    pk_value: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    Ok(())
}

use std::collections::HashMap;
