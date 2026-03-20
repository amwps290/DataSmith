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

/// 标准化目录名（转小写并清理特殊字符）
fn normalize_dir_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// 获取特定数据库的脚本目录
#[tauri::command]
pub async fn get_db_scripts_dir(
    connection_id: String,
    database: String,
    app: AppHandle,
) -> Result<String, String> {
    let root = get_scripts_root_dir(&app)?;
    let conn_dir = normalize_dir_name(&connection_id);
    let db_dir_name = normalize_dir_name(&database);
    
    let db_dir = root.join(&conn_dir).join(&db_dir_name);
    let path_str = db_dir.to_string_lossy().to_string();
    
    if !db_dir.exists() {
        fs::create_dir_all(&db_dir).map_err(|e| format!("无法创建脚本目录: {}", e))?;
    }
    
    println!("[Workspace] 脚本存放目录: {}", path_str);
    Ok(path_str)
}

/// 列出特定数据库下的脚本文件
#[tauri::command]
pub async fn list_db_scripts(
    connection_id: String,
    database: String,
    app: AppHandle,
) -> Result<Vec<ScriptInfo>, String> {
    let root = get_scripts_root_dir(&app)?;
    let conn_dir = normalize_dir_name(&connection_id);
    let db_dir_name = normalize_dir_name(&database);
    
    let db_dir = root.join(&conn_dir).join(&db_dir_name);
    let path_str = db_dir.to_string_lossy().to_string();
    
    println!("[Workspace] 正在扫描目录: {}", path_str);
    
    if !db_dir.exists() {
        println!("[Workspace] 目录不存在，返回空列表");
        return Ok(Vec::new());
    }

    let mut scripts = Vec::new();
    let entries = fs::read_dir(&db_dir).map_err(|e| format!("无法读取目录: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sql") {
            let metadata = entry.metadata().map_err(|e| e.to_string())?;
            let last_modified = metadata.modified().map_err(|e| e.to_string())?
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            
            scripts.push(ScriptInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.to_string_lossy().to_string().replace("\\", "/"),
                last_modified,
                size: metadata.len(),
            });
        }
    }

    println!("[Workspace] 扫描完成, 找到 {} 个有效 SQL 脚本", scripts.len());
    scripts.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    
    Ok(scripts)
}

/// 在特定数据库目录下创建一个新脚本
#[tauri::command]
pub async fn create_db_script(
    connection_id: String,
    database: String,
    content: Option<String>,
    app: AppHandle,
) -> Result<ScriptInfo, String> {
    let root = get_scripts_root_dir(&app)?;
    let conn_dir = normalize_dir_name(&connection_id);
    let db_dir_name = normalize_dir_name(&database);
    
    let db_dir = root.join(&conn_dir).join(&db_dir_name);
    
    if !db_dir.exists() {
        fs::create_dir_all(&db_dir).map_err(|e| format!("无法创建目录: {}", e))?;
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();
    let file_name = format!("script-{}.sql", timestamp);
    let path = db_dir.join(&file_name);
    
    let initial_content = content.unwrap_or_else(|| "-- 在此输入 SQL 查询\n".to_string());
    fs::write(&path, initial_content).map_err(|e| format!("创建脚本失败: {}", e))?;
    
    println!("[Workspace] 物理文件已创建: {}", path.to_string_lossy());
    
    let metadata = path.metadata().map_err(|e| e.to_string())?;
    
    Ok(ScriptInfo {
        name: file_name,
        path: path.to_string_lossy().to_string().replace("\\", "/"),
        last_modified: timestamp as u64 / 1000,
        size: metadata.len(),
    })
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
