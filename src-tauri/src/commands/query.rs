use std::collections::HashMap;
use crate::database::QueryResult;
use crate::utils::sql_formatter::SqlFormatter;
use crate::AppState;
use tauri::State;
use tracing::{info, instrument};

pub trait ToCommandResult<T> {
    fn to_cmd_result(self) -> Result<T, String>;
}

impl<T, E: std::fmt::Display> ToCommandResult<T> for Result<T, E> {
    fn to_cmd_result(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

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
        .to_cmd_result()?;
        
    Ok(SqlFormatter::beautify(&sql, &db_type))
}

/// 执行 SQL 查询 - 支持多结果集
#[tauri::command]
#[instrument(skip(state, sql))]
pub async fn execute_query(
    connection_id: String,
    sql: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<QueryResult>, String> {
    let manager = &state.connection_manager;
    
    info!(sql = %sql.replace('\n', " ").trim(), "收到执行请求");
    
    manager
        .execute_query(&connection_id, &sql, database.as_deref())
        .await
        .to_cmd_result()
}

/// 获取 SQL 执行计划
#[tauri::command]
#[instrument(skip(state, sql))]
pub async fn explain_query(
    connection_id: String,
    sql: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<QueryResult>, String> {
    let manager = &state.connection_manager;
    
    info!(sql = %sql.replace('\n', " ").trim(), "收到解释请求");
    
    manager
        .explain_query(&connection_id, &sql, database.as_deref())
        .await
        .to_cmd_result()
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
        let res_vec = manager.execute_query(&connection_id, &sql, None).await.to_cmd_result()?;
        results.extend(res_vec);
    }
    Ok(results)
}

#[tauri::command]
pub async fn alter_table_structure(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    changes: Vec<crate::database::TableChange>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = &state.connection_manager;
    manager
        .alter_table(&connection_id, &table, schema.as_deref(), Some(&database), changes)
        .await
        .to_cmd_result()
}

#[tauri::command]
pub async fn update_table_data(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    column: String,
    value: Option<String>,
    where_conditions: HashMap<String, serde_json::Value>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = &state.connection_manager;
    manager
        .update_data(&connection_id, &table, schema.as_deref(), Some(&database), &column, value, where_conditions)
        .await
        .to_cmd_result()
}

#[tauri::command]
pub async fn delete_table_data(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    where_conditions: HashMap<String, serde_json::Value>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = &state.connection_manager;
    manager
        .delete_data(&connection_id, &table, schema.as_deref(), Some(&database), where_conditions)
        .await
        .to_cmd_result()
}

#[tauri::command]
pub async fn insert_table_data(
    _connection_id: String,
    _database: String,
    _table: String,
    _data: HashMap<String, serde_json::Value>,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    Ok(())
}
