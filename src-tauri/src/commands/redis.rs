use std::sync::Arc;
use crate::database::redis::RedisDatabase;
use crate::database::DatabaseOperations;
use crate::AppState;
use super::error::ToCommandResult;
use tauri::State;
use std::collections::HashMap;

/// 获取 Redis 数据库引用的辅助函数
async fn get_redis_ref(connection_id: &str, state: &AppState) -> Result<Arc<dyn DatabaseOperations>, String> {
    state.connection_manager.get_db_ref(connection_id).await.to_cmd_result()
}

/// 执行 Redis 命令
#[tauri::command]
pub async fn execute_redis_command(
    connection_id: String,
    command: String,
    args: Vec<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = get_redis_ref(&connection_id, &state).await?;
    let redis_db = db.as_any().downcast_ref::<RedisDatabase>().ok_or("不是 Redis 连接")?;

    match redis_db.execute_command(&command, args).await {
        Ok(val) => Ok(redis_value_to_json(val)),
        Err(e) => Err(e.to_string()),
    }
}

/// 获取 Redis 信息
#[tauri::command]
pub async fn get_redis_info(
    connection_id: String,
    state: State<'_, AppState>,
) -> Result<HashMap<String, String>, String> {
    let db = get_redis_ref(&connection_id, &state).await?;
    let redis_db = db.as_any().downcast_ref::<RedisDatabase>().ok_or("不是 Redis 连接")?;

    let info_str = redis_db.get_server_info().await.to_cmd_result()?;

    let mut info_map = HashMap::new();
    for line in info_str.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() == 2 {
            info_map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }

    Ok(info_map)
}

/// 获取 Redis Key 的值和 TTL
#[tauri::command]
pub async fn get_redis_key_value(
    connection_id: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = get_redis_ref(&connection_id, &state).await?;
    let redis_db = db.as_any().downcast_ref::<RedisDatabase>().ok_or("不是 Redis 连接")?;

    let ttl = redis_db.get_key_ttl(&key).await.to_cmd_result()?;
    let value = redis_db.get_key_value(&key).await.to_cmd_result()?;

    Ok(serde_json::json!({
        "value": value,
        "ttl": ttl,
    }))
}

/// 设置 Redis Key 的值
#[tauri::command]
pub async fn set_redis_key_value(
    connection_id: String,
    key: String,
    value: String,
    ttl: Option<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = get_redis_ref(&connection_id, &state).await?;
    let redis_db = db.as_any().downcast_ref::<RedisDatabase>().ok_or("不是 Redis 连接")?;

    redis_db.set_key_value(&key, &value, ttl).await.to_cmd_result()
}

/// 删除 Redis Key
#[tauri::command]
pub async fn delete_redis_key(
    connection_id: String,
    key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = get_redis_ref(&connection_id, &state).await?;
    let redis_db = db.as_any().downcast_ref::<RedisDatabase>().ok_or("不是 Redis 连接")?;

    redis_db.delete_key(&key).await.to_cmd_result()
}

/// 将 Redis 的 Value 转换为 JSON Value (适配 1.0.2 版本变体名)
fn redis_value_to_json(value: redis::Value) -> serde_json::Value {
    match value {
        redis::Value::Nil => serde_json::Value::Null,
        redis::Value::Int(i) => serde_json::Value::Number(i.into()),
        redis::Value::BulkString(d) => {
            if let Ok(s) = String::from_utf8(d) {
                serde_json::Value::String(s)
            } else {
                serde_json::Value::String("Binary Data".to_string())
            }
        },
        redis::Value::Array(v) => {
            let list: Vec<serde_json::Value> = v.into_iter().map(redis_value_to_json).collect();
            serde_json::Value::Array(list)
        },
        redis::Value::SimpleString(s) => serde_json::Value::String(s),
        redis::Value::Okay => serde_json::Value::String("OK".to_string()),
        _ => serde_json::Value::String(format!("{:?}", value)),
    }
}
