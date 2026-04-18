use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::database::get_db;

/// 自定义 Prompt 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPrompt {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub prompt: String,
    pub content_types: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建或更新自定义 Prompt
pub fn upsert_prompt(
    id: &str,
    name: &str,
    description: Option<&str>,
    prompt: &str,
    content_types: Option<&str>,
    sort_order: i32,
) -> Result<(), String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    conn.execute(
        "INSERT INTO custom_prompts (id, name, description, prompt, content_types, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            description = excluded.description,
            prompt = excluded.prompt,
            content_types = excluded.content_types,
            sort_order = excluded.sort_order,
            updated_at = datetime('now', 'localtime')",
        params![id, name, description, prompt, content_types, sort_order],
    )
    .map_err(|e| format!("保存 Prompt 失败: {}", e))?;

    Ok(())
}

/// 查询所有自定义 Prompt
pub fn list_prompts() -> Result<Vec<CustomPrompt>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, prompt, content_types, sort_order, created_at, updated_at
             FROM custom_prompts
             ORDER BY sort_order ASC, created_at DESC",
        )
        .map_err(|e| format!("查询准备失败: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(CustomPrompt {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                prompt: row.get(3)?,
                content_types: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(items)
}

/// 根据 ID 获取 Prompt
pub fn get_prompt(id: &str) -> Result<Option<CustomPrompt>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let result = conn.query_row(
        "SELECT id, name, description, prompt, content_types, sort_order, created_at, updated_at
         FROM custom_prompts WHERE id = ?1",
        params![id],
        |row| {
            Ok(CustomPrompt {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                prompt: row.get(3)?,
                content_types: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    );

    match result {
        Ok(prompt) => Ok(Some(prompt)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("查询 Prompt 失败: {}", e)),
    }
}

/// 删除 Prompt
pub fn delete_prompt(id: &str) -> Result<(), String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    conn.execute("DELETE FROM custom_prompts WHERE id = ?1", params![id])
        .map_err(|e| format!("删除 Prompt 失败: {}", e))?;
    Ok(())
}

/// 查询适用于指定内容类型的自定义 Prompt
pub fn find_prompts_for_type(content_type: &str) -> Result<Vec<CustomPrompt>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, prompt, content_types, sort_order, created_at, updated_at
             FROM custom_prompts
             WHERE content_types IS NULL OR content_types LIKE '%' || ?1 || '%'
             ORDER BY sort_order ASC",
        )
        .map_err(|e| format!("查询准备失败: {}", e))?;

    let rows = stmt
        .query_map(params![content_type], |row| {
            Ok(CustomPrompt {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                prompt: row.get(3)?,
                content_types: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(items)
}
