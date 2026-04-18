use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use async_trait::async_trait;
use std::time::Duration;

/// 表格 → Markdown 表格
pub struct TableToMarkdownAction;

#[async_trait]
impl Action for TableToMarkdownAction {
    fn id(&self) -> &str {
        "table_to_markdown"
    }
    fn display_name(&self) -> &str {
        "转换为 Markdown 表格"
    }
    fn display_name_en(&self) -> &str {
        "Convert to Markdown Table"
    }
    fn description(&self) -> &str {
        "将 TSV/CSV 数据转换为 Markdown 表格格式"
    }
    fn description_en(&self) -> &str {
        "Convert TSV/CSV data to Markdown table format"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![
            ContentType::TableData("tsv".to_string()),
            ContentType::TableData("csv".to_string()),
        ]
    }

    fn requires_model(&self) -> bool {
        false
    }
    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(10)
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let rows = parse_table(&input.content, &input.content_type)?;
        if rows.is_empty() {
            return Err("表格数据为空".to_string());
        }

        let mut md = String::new();
        // 表头
        md.push_str("| ");
        md.push_str(&rows[0].join(" | "));
        md.push_str(" |\n");
        // 分隔线
        md.push_str("|");
        for _ in &rows[0] {
            md.push_str(" --- |");
        }
        md.push('\n');
        // 数据行
        for row in &rows[1..] {
            md.push_str("| ");
            md.push_str(&row.join(" | "));
            md.push_str(" |\n");
        }

        Ok(ActionOutput {
            result: md,
            result_type: "markdown".to_string(),
        })
    }
}

/// 表格 → JSON 数组
pub struct TableToJsonAction;

#[async_trait]
impl Action for TableToJsonAction {
    fn id(&self) -> &str {
        "table_to_json"
    }
    fn display_name(&self) -> &str {
        "转换为 JSON 数组"
    }
    fn display_name_en(&self) -> &str {
        "Convert to JSON Array"
    }
    fn description(&self) -> &str {
        "将 TSV/CSV 数据转换为 JSON 对象数组"
    }
    fn description_en(&self) -> &str {
        "Convert TSV/CSV data to JSON object array"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![
            ContentType::TableData("tsv".to_string()),
            ContentType::TableData("csv".to_string()),
        ]
    }

    fn requires_model(&self) -> bool {
        false
    }
    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(10)
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let rows = parse_table(&input.content, &input.content_type)?;
        if rows.len() < 2 {
            return Err("表格数据至少需要表头和一行数据".to_string());
        }

        let headers = &rows[0];
        let mut json_arr: Vec<serde_json::Value> = Vec::new();

        for row in &rows[1..] {
            let mut obj = serde_json::Map::new();
            for (i, header) in headers.iter().enumerate() {
                let val = row.get(i).map(|s| s.as_str()).unwrap_or("");
                obj.insert(header.clone(), serde_json::Value::String(val.to_string()));
            }
            json_arr.push(serde_json::Value::Object(obj));
        }

        let result = serde_json::to_string_pretty(&json_arr)
            .map_err(|e| format!("JSON 序列化失败: {}", e))?;

        Ok(ActionOutput {
            result,
            result_type: "json".to_string(),
        })
    }
}

/// 解析表格内容为二维数组
fn parse_table(content: &str, content_type: &ContentType) -> Result<Vec<Vec<String>>, String> {
    let delimiter = match content_type {
        ContentType::TableData(fmt) if fmt == "tsv" => '\t',
        ContentType::TableData(fmt) if fmt == "csv" => ',',
        _ => {
            // 自动检测
            if content.contains('\t') {
                '\t'
            } else {
                ','
            }
        }
    };

    let rows: Vec<Vec<String>> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            line.split(delimiter)
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    if rows.is_empty() {
        return Err("无法解析表格数据".to_string());
    }

    Ok(rows)
}
