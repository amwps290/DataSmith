use crate::database::{QueryResult, ColumnInfo, IndexInfo, DatabaseInfo, TableInfo, SchemaInfo, FunctionInfo, ExtensionInfo};
use crate::AppState;
use tauri::State;
use serde_json::Value;

#[tauri::command]
pub async fn get_databases(connection_id: String, state: State<'_, AppState>) -> Result<Vec<DatabaseInfo>, String> {
    state.connection_manager.get_databases(&connection_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tables(connection_id: String, database: Option<String>, state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    state.connection_manager.get_tables(&connection_id, database.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_views(connection_id: String, database: Option<String>, state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    state.connection_manager.get_views(&connection_id, database.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_schemas(connection_id: String, database: Option<String>, state: State<'_, AppState>) -> Result<Vec<SchemaInfo>, String> {
    state.connection_manager.get_schemas(&connection_id, database.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_schema_tables(connection_id: String, database: String, _schema: String, state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    // 简化：目前 manager 主要是获取全库表，后续可按 schema 过滤
    state.connection_manager.get_tables(&connection_id, Some(&database)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_schema_views(connection_id: String, database: String, _schema: String, state: State<'_, AppState>) -> Result<Vec<TableInfo>, String> {
    state.connection_manager.get_views(&connection_id, Some(&database)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_schema_functions(connection_id: String, database: String, schema: String, state: State<'_, AppState>) -> Result<Vec<FunctionInfo>, String> {
    state.connection_manager.get_functions(&connection_id, Some(&database), Some(&schema)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_schema_aggregate_functions(connection_id: String, database: String, schema: String, state: State<'_, AppState>) -> Result<Vec<FunctionInfo>, String> {
    state.connection_manager.get_aggregate_functions(&connection_id, Some(&database), Some(&schema)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_database_extensions(connection_id: String, database: String, state: State<'_, AppState>) -> Result<Vec<ExtensionInfo>, String> {
    state.connection_manager.get_extensions(&connection_id, Some(&database)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_table_structure(connection_id: String, table: String, database: Option<String>, schema: Option<String>, state: State<'_, AppState>) -> Result<Vec<ColumnInfo>, String> {
    state.connection_manager.get_table_structure(&connection_id, &table, schema.as_deref(), database.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_table_indexes(connection_id: String, table: String, schema: Option<String>, state: State<'_, AppState>) -> Result<Vec<IndexInfo>, String> {
    state.connection_manager.get_indexes(&connection_id, &table, schema.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_schema_indexes(_connection_id: String, _database: String, _schema: String, _state: State<'_, AppState>) -> Result<Vec<IndexInfo>, String> {
    Ok(vec![])
}

// 占位指令，后续完善
#[tauri::command] pub async fn view_table_data(_c: String, _t: String, _d: Option<String>, _s: Option<String>) -> Result<Vec<QueryResult>, String> { Err("Not implemented".into()) }
#[tauri::command] pub async fn truncate_table(_c: String, _t: String, _d: Option<String>, _s: Option<String>) -> Result<(), String> { Ok(()) }
#[tauri::command] pub async fn drop_table(_c: String, _t: String, _d: Option<String>, _s: Option<String>) -> Result<(), String> { Ok(()) }
#[tauri::command] pub async fn get_procedures(_c: String, _d: Option<String>, _s: Option<String>) -> Result<Vec<Value>, String> { Ok(vec![]) }
#[tauri::command] pub async fn get_functions(_c: String, _d: Option<String>, _s: Option<String>) -> Result<Vec<FunctionInfo>, String> { Ok(vec![]) }
#[tauri::command] pub async fn get_triggers(_c: String, _t: String, _d: Option<String>, _s: Option<String>) -> Result<Vec<Value>, String> { Ok(vec![]) }
#[tauri::command] pub async fn get_events(_c: String, _d: Option<String>, _s: Option<String>) -> Result<Vec<Value>, String> { Ok(vec![]) }
#[tauri::command] pub async fn drop_view(_c: String, _v: String, _d: Option<String>, _s: Option<String>) -> Result<(), String> { Ok(()) }
#[tauri::command] pub async fn get_view_definition(_c: String, _v: String, _d: Option<String>, _s: Option<String>) -> Result<String, String> { Ok("".into()) }
#[tauri::command] pub async fn drop_procedure(_c: String, _p: String, _d: Option<String>, _s: Option<String>) -> Result<(), String> { Ok(()) }
#[tauri::command] pub async fn drop_function(_c: String, _f: String, _d: Option<String>, _s: Option<String>) -> Result<(), String> { Ok(()) }
#[tauri::command] pub async fn drop_trigger(_c: String, _t: String, _d: Option<String>, _s: Option<String>) -> Result<(), String> { Ok(()) }
#[tauri::command] pub async fn drop_event(_c: String, _e: String, _d: Option<String>, _s: Option<String>) -> Result<(), String> { Ok(()) }
#[tauri::command] pub async fn get_table_foreign_keys(_c: String, _t: String, _s: Option<String>) -> Result<Vec<Value>, String> { Ok(vec![]) }
#[tauri::command] pub async fn get_create_table_ddl(_c: String, _t: String, _d: Option<String>, _s: Option<String>) -> Result<String, String> { Ok("".into()) }
#[tauri::command] pub async fn get_autocomplete_data(_c: String, _d: Option<String>) -> Result<Value, String> { Ok(serde_json::json!({})) }
