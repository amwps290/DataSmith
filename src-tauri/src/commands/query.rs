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
) -> Result<Vec<QueryResult>, String> {
    let manager = &state.connection_manager;
    
    info!(sql = %sql.replace('\n', " ").trim(), "收到解释请求");
    
    let result = manager
        .explain_query(&connection_id, &sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())?;
        
    Ok(result)
}

/// 分页执行 SQL 查询 (目前仅支持取第一个结果集的分页)
#[tauri::command]
#[instrument(skip(state, sql))]
pub async fn execute_query_paged(
    connection_id: String,
    sql: String,
    database: Option<String>,
    _page: u32,
    _page_size: u32,
    state: State<'_, AppState>,
) -> Result<Vec<QueryResult>, String> {
    let manager = &state.connection_manager;
    
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
        let res_vec = manager.execute_query(&connection_id, &sql, None).await.map_err(|e| e.to_string())?;
        results.extend(res_vec);
    }
    Ok(results)
}

#[tauri::command]
pub async fn update_table_data(
    connection_id: String,
    database: String,
    table: String,
    schema: Option<String>,
    column: String,
    value: Option<String>,
    where_clause: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = &state.connection_manager;
    manager
        .update_data(&connection_id, &table, schema.as_deref(), Some(&database), &column, value, &where_clause)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn insert_table_data(
    _connection_id: String,
    _database: String,
    _table: String,
    _data: std::collections::HashMap<String, serde_json::Value>,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn delete_table_data(
    connection_id: String,
    database: String,
    table: String,
    _schema: Option<String>,
    where_clause: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = &state.connection_manager;
    let sql = format!("DELETE FROM {} WHERE {}", table, where_clause); // 简化逻辑
    manager
        .execute_query(&connection_id, &sql, Some(&database))
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
