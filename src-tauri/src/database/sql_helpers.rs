pub enum ParamStyle {
    QuestionMark,        // MySQL, SQLite: ?
    DollarNumber(usize), // PostgreSQL: $1, $2, ...
}

pub struct WhereClause {
    pub sql: String,
    pub param_values: Vec<Option<String>>,
}

/// 构建 WHERE 子句
pub fn build_where_clause(
    conditions: &std::collections::HashMap<String, serde_json::Value>,
    escape_fn: fn(&str) -> String,
    param_style: ParamStyle,
) -> WhereClause {
    let mut parts = Vec::new();
    let mut values = Vec::new();
    let mut idx = match &param_style {
        ParamStyle::DollarNumber(start) => *start,
        _ => 0,
    };

    for (col, val) in conditions {
        if val.is_null() {
            parts.push(format!("{} IS NULL", escape_fn(col)));
        } else {
            let placeholder = match &param_style {
                ParamStyle::QuestionMark => "?".to_string(),
                ParamStyle::DollarNumber(_) => {
                    idx += 1;
                    format!("${}", idx)
                }
            };
            parts.push(format!("{} = {}", escape_fn(col), placeholder));
            values.push(Some(match val {
                serde_json::Value::String(s) => s.clone(),
                _ => val.to_string(),
            }));
        }
    }

    WhereClause {
        sql: parts.join(" AND "),
        param_values: values,
    }
}
