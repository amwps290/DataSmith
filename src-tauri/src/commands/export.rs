use super::error::ToCommandResult;
use crate::database::QueryResult;
use crate::utils::sql_sanitize::escape_mysql_id;
use crate::AppState;
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use tauri::State;

/// 导出为 CSV
#[tauri::command]
pub async fn export_to_csv(
    data: QueryResult,
    file_path: String,
) -> Result<bool, String> {
    let mut file = File::create(&file_path).to_cmd_result()?;

    // 写入表头
    let header = data.columns.join(",");
    writeln!(file, "{}", header).to_cmd_result()?;

    // 写入数据行
    for row in &data.rows {
        let values: Vec<String> = data
            .columns
            .iter()
            .map(|col| {
                let val = row.get(col).unwrap_or(&Value::Null);
                match val {
                    Value::String(s) => format!("\"{}\"", s.replace("\"", "\"\"")),
                    Value::Null => String::new(),
                    _ => val.to_string(),
                }
            })
            .collect();
        writeln!(file, "{}", values.join(",")).to_cmd_result()?;
    }

    Ok(true)
}

/// 导出为 JSON
#[tauri::command]
pub async fn export_to_json(
    data: QueryResult,
    file_path: String,
) -> Result<bool, String> {
    let json = serde_json::to_string_pretty(&data.rows).to_cmd_result()?;

    let mut file = File::create(&file_path).to_cmd_result()?;
    file.write_all(json.as_bytes()).to_cmd_result()?;

    Ok(true)
}

/// 导出为 SQL INSERT 语句
#[tauri::command]
pub async fn export_to_sql(
    data: QueryResult,
    table_name: String,
    file_path: String,
) -> Result<bool, String> {
    let mut file = File::create(&file_path).to_cmd_result()?;

    let columns = data.columns.iter().map(|c| escape_mysql_id(c)).collect::<Vec<_>>().join(", ");

    for row in &data.rows {
        let values: Vec<String> = data
            .columns
            .iter()
            .map(|col| {
                let val = row.get(col).unwrap_or(&Value::Null);
                match val {
                    Value::String(s) => format!("'{}'", s.replace("'", "''")),
                    Value::Null => "NULL".to_string(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                    _ => format!("'{}'", val.to_string()),
                }
            })
            .collect();

        let insert_stmt = format!(
            "INSERT INTO {} ({}) VALUES ({});\n",
            escape_mysql_id(&table_name),
            columns,
            values.join(", ")
        );

        file.write_all(insert_stmt.as_bytes()).to_cmd_result()?;
    }

    Ok(true)
}

/// 导出表结构为 DDL
#[tauri::command]
pub async fn export_table_ddl(
    connection_id: String,
    database: String,
    table: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = &state.connection_manager;

    let ddl = manager
        .get_table_ddl(&connection_id, &table, None, Some(&database))
        .await
        .map_err(|e| format!("获取表结构失败: {}", e))?;

    Ok(ddl)
}
