use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::database::get_db;

/// 剪贴板历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardHistoryItem {
    pub id: i64,
    pub content: Option<String>,
    pub image_path: Option<String>,
    pub content_type: String,
    pub source_app: Option<String>,
    pub char_count: Option<i64>,
    pub created_at: String,
    pub is_pinned: bool,
    pub is_sensitive: bool,
}

/// 插入新的剪贴板历史记录，返回新记录的 ID
pub fn insert_history(
    content: Option<&str>,
    image_path: Option<&str>,
    image_hash: Option<&str>,
    content_type: &str,
    source_app: Option<&str>,
    char_count: Option<i64>,
    is_sensitive: bool,
) -> Result<i64, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    if let Some(hash) = image_hash {
        let existing_id = conn
            .query_row(
                "SELECT id FROM clipboard_history WHERE image_hash = ?1 LIMIT 1",
                params![hash],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| format!("查询图片历史失败: {}", e))?;

        if let Some(id) = existing_id {
            conn.execute(
                "UPDATE clipboard_history
                 SET image_path = ?1,
                     content_type = ?2,
                     source_app = ?3,
                     char_count = ?4,
                     is_sensitive = ?5,
                     created_at = datetime('now', 'localtime')
                 WHERE id = ?6",
                params![
                    image_path,
                    content_type,
                    source_app,
                    char_count,
                    is_sensitive as i32,
                    id
                ],
            )
            .map_err(|e| format!("更新图片历史失败: {}", e))?;

            return Ok(id);
        }
    }

    conn.execute(
        "INSERT INTO clipboard_history (content, image_path, image_hash, content_type, source_app, char_count, is_sensitive)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![content, image_path, image_hash, content_type, source_app, char_count, is_sensitive as i32],
    )
    .map_err(|e| format!("写入历史记录失败: {}", e))?;

    Ok(conn.last_insert_rowid())
}

/// 查询历史记录（分页，按时间倒序）
pub fn list_history(limit: i64, offset: i64) -> Result<Vec<ClipboardHistoryItem>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut stmt = conn
        .prepare(
            "WITH ranked AS (
                 SELECT id, content, image_path, content_type, source_app, char_count, created_at, is_pinned, is_sensitive,
                        ROW_NUMBER() OVER (
                            PARTITION BY content_type, COALESCE(image_path, ''), COALESCE(content, '')
                            ORDER BY created_at DESC, id DESC
                        ) AS rn
                 FROM clipboard_history
             )
             SELECT id, content, image_path, content_type, source_app, char_count, created_at, is_pinned, is_sensitive
             FROM ranked
             WHERE rn = 1
             ORDER BY created_at DESC, id DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| format!("查询准备失败: {}", e))?;

    let rows = stmt
        .query_map(params![limit, offset], map_history_row)
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(items)
}

/// 搜索历史记录（关键词 + 可选类型过滤）
pub fn search_history(
    keyword: &str,
    content_type_filter: Option<&str>,
    limit: i64,
) -> Result<Vec<ClipboardHistoryItem>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let like_pattern = format!("%{}%", keyword);

    let (query, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(ct) =
        content_type_filter
    {
        (
                "WITH ranked AS (
                     SELECT id, content, image_path, content_type, source_app, char_count, created_at, is_pinned, is_sensitive,
                            ROW_NUMBER() OVER (
                                PARTITION BY content_type, COALESCE(image_path, ''), COALESCE(content, '')
                                ORDER BY created_at DESC, id DESC
                            ) AS rn
                     FROM clipboard_history
                     WHERE content LIKE ?1 AND content_type = ?2
                 )
                 SELECT id, content, image_path, content_type, source_app, char_count, created_at, is_pinned, is_sensitive
                 FROM ranked
                 WHERE rn = 1
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?3"
                    .to_string(),
                vec![
                    Box::new(like_pattern),
                    Box::new(ct.to_string()),
                    Box::new(limit),
                ],
            )
    } else {
        (
                "WITH ranked AS (
                     SELECT id, content, image_path, content_type, source_app, char_count, created_at, is_pinned, is_sensitive,
                            ROW_NUMBER() OVER (
                                PARTITION BY content_type, COALESCE(image_path, ''), COALESCE(content, '')
                                ORDER BY created_at DESC, id DESC
                            ) AS rn
                     FROM clipboard_history
                     WHERE content LIKE ?1
                 )
                 SELECT id, content, image_path, content_type, source_app, char_count, created_at, is_pinned, is_sensitive
                 FROM ranked
                 WHERE rn = 1
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?2"
                    .to_string(),
                vec![Box::new(like_pattern), Box::new(limit)],
            )
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("查询准备失败: {}", e))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();

    let rows = stmt
        .query_map(params_refs.as_slice(), map_history_row)
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(items)
}

/// 删除历史记录
pub fn delete_history(id: i64) -> Result<(), String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    conn.execute("DELETE FROM clipboard_history WHERE id = ?1", params![id])
        .map_err(|e| format!("删除失败: {}", e))?;
    Ok(())
}

/// 切换收藏状态
pub fn toggle_pin(id: i64) -> Result<bool, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let current: i32 = conn
        .query_row(
            "SELECT is_pinned FROM clipboard_history WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| format!("查询收藏状态失败: {}", e))?;

    let new_state = if current == 0 { 1 } else { 0 };
    conn.execute(
        "UPDATE clipboard_history SET is_pinned = ?1 WHERE id = ?2",
        params![new_state, id],
    )
    .map_err(|e| format!("更新收藏状态失败: {}", e))?;

    Ok(new_state != 0)
}

/// 清空所有未收藏的历史记录
pub fn clear_unpinned() -> Result<u64, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let count = conn
        .execute("DELETE FROM clipboard_history WHERE is_pinned = 0", [])
        .map_err(|e| format!("清空失败: {}", e))?;
    Ok(count as u64)
}

/// 清空未收藏的历史记录，但保留最近 retain_days 天内的数据。
/// retain_days == 0 表示清空全部未收藏记录。
pub fn clear_unpinned_with_retention(retain_days: u32) -> Result<u64, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let count = if retain_days == 0 {
        conn.execute("DELETE FROM clipboard_history WHERE is_pinned = 0", [])
    } else {
        conn.execute(
            "DELETE FROM clipboard_history WHERE is_pinned = 0 AND created_at < datetime('now', 'localtime', ?1)",
            params![format!("-{} days", retain_days)],
        )
    }
    .map_err(|e| format!("清空失败: {}", e))?;

    Ok(count as u64)
}

/// 统计未收藏且 content 字节数 >= min_bytes 的文本记录数量和总字节数。
/// 仅统计 content 非空的记录（图片等无文本内容的记录不计入）。
pub fn count_unpinned_over_size(min_bytes: i64) -> Result<(i64, i64), String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(LENGTH(CAST(content AS BLOB))), 0)
         FROM clipboard_history
         WHERE is_pinned = 0 AND content IS NOT NULL
               AND LENGTH(CAST(content AS BLOB)) >= ?1",
        params![min_bytes],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    )
    .map_err(|e| format!("查询失败: {}", e))
}

/// 删除未收藏且 content 字节数 >= min_bytes 的文本记录，返回删除数量。
pub fn clear_unpinned_over_size(min_bytes: i64) -> Result<u64, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let count = conn
        .execute(
            "DELETE FROM clipboard_history
             WHERE is_pinned = 0 AND content IS NOT NULL
                   AND LENGTH(CAST(content AS BLOB)) >= ?1",
            params![min_bytes],
        )
        .map_err(|e| format!("按大小清空失败: {}", e))?;
    Ok(count as u64)
}

/// 搜索历史记录（关键词 + 可选类型 + 可选标签 + 可选日期范围组合筛选）
pub fn search_history_advanced(
    keyword: Option<&str>,
    content_type_filter: Option<&str>,
    tag_filter: Option<&str>,
    pinned_only: Option<bool>,
    date_from: Option<&str>,
    date_to: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClipboardHistoryItem>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut conditions = Vec::new();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut param_idx = 1u32;

    // 标签过滤需要 JOIN
    let join_clause = if tag_filter.is_some() {
        "INNER JOIN clipboard_tags ct ON ch.id = ct.clipboard_id"
    } else {
        ""
    };

    if let Some(kw) = keyword {
        if !kw.is_empty() {
            conditions.push(format!("ch.content LIKE ?{}", param_idx));
            param_values.push(Box::new(format!("%{}%", kw)));
            param_idx += 1;
        }
    }

    if let Some(ct) = content_type_filter {
        if ct.starts_with("Code") {
            conditions.push(format!("ch.content_type LIKE ?{}", param_idx));
            param_values.push(Box::new("Code%".to_string()));
        } else {
            conditions.push(format!("ch.content_type = ?{}", param_idx));
            param_values.push(Box::new(ct.to_string()));
        }
        param_idx += 1;
    }

    if let Some(tag) = tag_filter {
        conditions.push(format!("ct.tag_name = ?{}", param_idx));
        param_values.push(Box::new(tag.to_string()));
        param_idx += 1;
    }

    if pinned_only == Some(true) {
        conditions.push("ch.is_pinned = 1".to_string());
    }

    if let Some(df) = date_from {
        if !df.is_empty() {
            conditions.push(format!("ch.created_at >= ?{}", param_idx));
            param_values.push(Box::new(df.to_string()));
            param_idx += 1;
        }
    }

    if let Some(dt) = date_to {
        if !dt.is_empty() {
            conditions.push(format!("ch.created_at <= ?{}", param_idx));
            param_values.push(Box::new(dt.to_string()));
            param_idx += 1;
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let query = format!(
        "WITH ranked AS (
             SELECT ch.id, ch.content, ch.image_path, ch.content_type, ch.source_app, ch.char_count, ch.created_at, ch.is_pinned, ch.is_sensitive,
                    ROW_NUMBER() OVER (
                        PARTITION BY ch.content_type, COALESCE(ch.image_path, ''), COALESCE(ch.content, '')
                        ORDER BY ch.created_at DESC, ch.id DESC
                    ) AS rn
             FROM clipboard_history ch
             {}
             {}
         )
         SELECT id, content, image_path, content_type, source_app, char_count, created_at, is_pinned, is_sensitive
         FROM ranked
         WHERE rn = 1
         ORDER BY created_at DESC, id DESC
         LIMIT ?{} OFFSET ?{}",
        join_clause, where_clause, param_idx, param_idx + 1
    );

    param_values.push(Box::new(limit));
    param_values.push(Box::new(offset));

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("查询准备失败: {}", e))?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        param_values.iter().map(|p| p.as_ref()).collect();

    let rows = stmt
        .query_map(params_refs.as_slice(), map_history_row)
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(items)
}

/// 获取历史记录总数
pub fn count_history() -> Result<i64, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    conn.query_row("SELECT COUNT(*) FROM clipboard_history", [], |row| {
        row.get(0)
    })
    .map_err(|e| format!("计数查询失败: {}", e))
}

fn map_history_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardHistoryItem> {
    Ok(ClipboardHistoryItem {
        id: row.get(0)?,
        content: row.get(1)?,
        image_path: row.get(2)?,
        content_type: row.get(3)?,
        source_app: row.get(4)?,
        char_count: row.get(5)?,
        created_at: row.get(6)?,
        is_pinned: row.get::<_, i32>(7)? != 0,
        is_sensitive: row.get::<_, i32>(8)? != 0,
    })
}
