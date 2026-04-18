use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::database::get_db;

/// 标签条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardTag {
    pub id: i64,
    pub clipboard_id: i64,
    pub tag_name: String,
    pub created_at: String,
}

/// 为剪贴板条目添加标签
pub fn add_tag(clipboard_id: i64, tag_name: &str) -> Result<(), String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    conn.execute(
        "INSERT OR IGNORE INTO clipboard_tags (clipboard_id, tag_name) VALUES (?1, ?2)",
        params![clipboard_id, tag_name],
    )
    .map_err(|e| format!("添加标签失败: {}", e))?;

    Ok(())
}

/// 删除剪贴板条目的指定标签
pub fn remove_tag(clipboard_id: i64, tag_name: &str) -> Result<(), String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    conn.execute(
        "DELETE FROM clipboard_tags WHERE clipboard_id = ?1 AND tag_name = ?2",
        params![clipboard_id, tag_name],
    )
    .map_err(|e| format!("删除标签失败: {}", e))?;

    Ok(())
}

/// 获取剪贴板条目的所有标签名
pub fn get_tags_for_item(clipboard_id: i64) -> Result<Vec<String>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT tag_name FROM clipboard_tags WHERE clipboard_id = ?1 ORDER BY created_at")
        .map_err(|e| format!("查询准备失败: {}", e))?;

    let rows = stmt
        .query_map(params![clipboard_id], |row| row.get::<_, String>(0))
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(tags)
}

/// 获取所有已使用的标签名（去重，按使用次数降序）
pub fn list_all_tags() -> Result<Vec<String>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT tag_name, COUNT(*) as cnt FROM clipboard_tags
             GROUP BY tag_name ORDER BY cnt DESC",
        )
        .map_err(|e| format!("查询准备失败: {}", e))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(tags)
}

/// 根据标签名搜索剪贴板条目 ID 列表
pub fn find_clipboard_ids_by_tag(tag_name: &str) -> Result<Vec<i64>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut stmt = conn
        .prepare("SELECT clipboard_id FROM clipboard_tags WHERE tag_name = ?1")
        .map_err(|e| format!("查询准备失败: {}", e))?;

    let rows = stmt
        .query_map(params![tag_name], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(ids)
}
