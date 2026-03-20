use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    pub key: String,
    pub title: String,
    #[serde(rename = "type")]
    pub tab_type: String,
    pub connection_id: Option<String>,
    pub database: Option<String>,
    pub schema: Option<String>,
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub open_tabs: Vec<TabState>,
    pub active_tab_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub name: String,
    pub path: String,
    pub last_modified: u64,
    pub size: u64,
}

/// 获取应用数据目录下的脚本根目录
fn get_scripts_root_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let scripts_dir = app_dir.join("scripts");
    if !scripts_dir.exists() {
        fs::create_dir_all(&scripts_dir).map_err(|e| e.to_string())?;
    }
    Ok(scripts_dir)
}

/// 获取特定数据库的脚本目录
#[tauri::command]
pub async fn get_db_scripts_dir(
    connection_id: String,
    database: String,
    app: AppHandle,
) -> Result<String, String> {
    let root = get_scripts_root_dir(&app)?;
    let db_dir = root.join(&connection_id).join(&database);
    if !db_dir.exists() {
        fs::create_dir_all(&db_dir).map_err(|e| e.to_string())?;
    }
    Ok(db_dir.to_string_lossy().to_string())
}

/// 列出特定数据库下的脚本文件
#[tauri::command]
pub async fn list_db_scripts(
    connection_id: String,
    database: String,
    app: AppHandle,
) -> Result<Vec<ScriptInfo>, String> {
    let root = get_scripts_root_dir(&app)?;
    let db_dir = root.join(&connection_id).join(&database);
    
    if !db_dir.exists() {
        return Ok(Vec::new());
    }

    let mut scripts = Vec::new();
    let entries = fs::read_dir(db_dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sql") {
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            let last_modified = metadata.modified().map_err(|e| e.to_string())?
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            
            scripts.push(ScriptInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string(),
                last_modified,
                size: metadata.len(),
            });
        }
    }

    // 按修改时间降序排列
    scripts.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    
    Ok(scripts)
}

/// 保存会话状态
#[tauri::command]
pub async fn save_session(
    state: SessionState,
    app: AppHandle,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let session_file = app_dir.join("session.json");
    
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    fs::write(session_file, json).map_err(|e| e.to_string())?;
    
    Ok(())
}

/// 加载会话状态
#[tauri::command]
pub async fn load_session(
    app: AppHandle,
) -> Result<Option<SessionState>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let session_file = app_dir.join("session.json");
    
    if !session_file.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(session_file).map_err(|e| e.to_string())?;
    let state: SessionState = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    
    Ok(Some(state))
}
