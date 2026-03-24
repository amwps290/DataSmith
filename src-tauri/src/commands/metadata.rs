use crate::database::{ColumnInfo, IndexInfo, ForeignKeyInfo, DatabaseInfo, TableInfo, SchemaInfo, FunctionInfo, ExtensionInfo};
use crate::AppState;
use super::error::ToCommandResult;
use tauri::State;
use serde_json::Value;

#[tauri::command]
pub async fn get_databases(connection_id: String, state: State<'_, AppState>) -> Result<Vec<DatabaseInfo>, String> {
    state.connection_manager.get_databases(&connection_id).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_tables(connection_id: String, database: Option<String>, state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    state.connection_manager.get_tables(&connection_id, database.as_deref()).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_views(connection_id: String, database: Option<String>, state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    state.connection_manager.get_views(&connection_id, database.as_deref()).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_schemas(connection_id: String, database: Option<String>, state: State<'_, AppState>) -> Result<Vec<SchemaInfo>, String> {
    state.connection_manager.get_schemas(&connection_id, database.as_deref()).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_schema_tables(connection_id: String, database: String, schema: String, state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    let tables = state.connection_manager.get_tables(&connection_id, Some(&database)).await.to_cmd_result()?;
    Ok(tables.into_iter().filter(|t| t.schema.as_deref() == Some(&schema)).collect())
}

#[tauri::command]
pub async fn get_schema_views(connection_id: String, database: String, schema: String, state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    let views = state.connection_manager.get_views(&connection_id, Some(&database)).await.to_cmd_result()?;
    Ok(views.into_iter().filter(|v| v.schema.as_deref() == Some(&schema)).collect())
}

#[tauri::command]
pub async fn get_schema_functions(connection_id: String, database: String, schema: String, state: State<'_, AppState>) -> Result<Vec<FunctionInfo>, String> {
    tracing::info!(conn = %connection_id, db = %database, sc = %schema, "正在获取 Schema 函数...");
    state.connection_manager.get_functions(&connection_id, Some(&database), Some(&schema)).await.map_err(|e| {
        tracing::error!(err = %e, "获取 Schema 函数失败");
        e.to_string()
    })
}

#[tauri::command]
pub async fn get_schema_aggregate_functions(connection_id: String, database: String, schema: String, state: State<'_, AppState>) -> Result<Vec<FunctionInfo>, String> {
    state.connection_manager.get_aggregate_functions(&connection_id, Some(&database), Some(&schema)).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_database_extensions(connection_id: String, database: String, state: State<'_, AppState>) -> Result<Vec<ExtensionInfo>, String> {
    tracing::info!(conn = %connection_id, db = %database, "正在获取数据库扩展...");
    state.connection_manager.get_extensions(&connection_id, Some(&database)).await.map_err(|e| {
        tracing::error!(err = %e, "获取数据库扩展失败");
        e.to_string()
    })
}

#[tauri::command]
pub async fn get_table_structure(connection_id: String, table: String, database: Option<String>, schema: Option<String>, state: State<'_, AppState>) -> Result<Vec<ColumnInfo>, String> {
    state.connection_manager.get_table_structure(&connection_id, &table, schema.as_deref(), database.as_deref()).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_table_indexes(connection_id: String, table: String, schema: Option<String>, state: State<'_, AppState>) -> Result<Vec<IndexInfo>, String> {
    state.connection_manager.get_indexes(&connection_id, &table, schema.as_deref()).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_table_foreign_keys(connection_id: String, table: String, schema: Option<String>, state: State<'_, AppState>) -> Result<Vec<ForeignKeyInfo>, String> {
    state.connection_manager.get_foreign_keys(&connection_id, &table, schema.as_deref()).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_schema_indexes(connection_id: String, database: String, schema: String, state: State<'_, AppState>) -> Result<Vec<IndexInfo>, String> {
    state.connection_manager.get_schema_indexes(&connection_id, Some(&database), Some(&schema)).await.to_cmd_result()
}

#[tauri::command]
pub async fn get_create_table_ddl(connection_id: String, table: String, database: Option<String>, schema: Option<String>, state: State<'_, AppState>) -> Result<String, String> {
    state.connection_manager.get_table_ddl(&connection_id, &table, schema.as_deref(), database.as_deref()).await.to_cmd_result()
}

#[tauri::command] pub async fn get_autocomplete_data(_c: String, _d: Option<String>) -> Result<Value, String> { Ok(serde_json::json!({})) }
