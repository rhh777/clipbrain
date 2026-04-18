use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::database::get_db;

/// 操作执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHistoryItem {
    pub id: i64,
    pub clipboard_id: Option<i64>,
    pub action_id: String,
    pub input_text: Option<String>,
    pub output_text: Option<String>,
    pub duration_ms: Option<i64>,
    pub model_used: Option<String>,
    pub created_at: String,
}

/// 写入操作执行记录
pub fn insert_action_history(
    clipboard_id: Option<i64>,
    action_id: &str,
    input_text: Option<&str>,
    output_text: Option<&str>,
    duration_ms: Option<i64>,
    model_used: Option<&str>,
) -> Result<i64, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    conn.execute(
        "INSERT INTO action_history (clipboard_id, action_id, input_text, output_text, duration_ms, model_used)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![clipboard_id, action_id, input_text, output_text, duration_ms, model_used],
    )
    .map_err(|e| format!("写入操作记录失败: {}", e))?;

    Ok(conn.last_insert_rowid())
}

/// 查询操作记录（分页，按时间倒序）
pub fn list_action_history(limit: i64, offset: i64) -> Result<Vec<ActionHistoryItem>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, clipboard_id, action_id, input_text, output_text, duration_ms, model_used, created_at
             FROM action_history
             ORDER BY created_at DESC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|e| format!("查询准备失败: {}", e))?;

    let rows = stmt
        .query_map(params![limit, offset], |row| {
            Ok(ActionHistoryItem {
                id: row.get(0)?,
                clipboard_id: row.get(1)?,
                action_id: row.get(2)?,
                input_text: row.get(3)?,
                output_text: row.get(4)?,
                duration_ms: row.get(5)?,
                model_used: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(items)
}

/// 统计概览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStats {
    /// 总操作次数
    pub total_count: i64,
    /// 总节省时间 (ms)
    pub total_duration_ms: i64,
    /// 最常用操作 TOP N
    pub top_actions: Vec<ActionUsageStat>,
    /// 最近 30 天每日统计
    pub daily_trend: Vec<DailyStat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionUsageStat {
    pub action_id: String,
    pub display_name: String,
    pub count: i64,
    pub total_duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyStat {
    pub date: String,
    pub count: i64,
}

/// 获取统计概览
pub fn get_action_stats() -> Result<ActionStats, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("DB lock failed: {}", e))?;

    // 总计数 & 总时间
    let (total_count, total_duration_ms): (i64, i64) = conn
        .query_row(
            "SELECT COUNT(*), COALESCE(SUM(duration_ms), 0) FROM action_history",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("Query failed: {}", e))?;

    // TOP 操作
    let mut stmt = conn
        .prepare(
            "SELECT action_id, COUNT(*) as cnt, COALESCE(SUM(duration_ms), 0) as dur
             FROM action_history
             GROUP BY action_id
             ORDER BY cnt DESC
             LIMIT 10",
        )
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let top_actions: Vec<ActionUsageStat> = stmt
        .query_map([], |row| {
            let action_id: String = row.get(0)?;
            Ok(ActionUsageStat {
                display_name: action_id.clone(),
                action_id,
                count: row.get(1)?,
                total_duration_ms: row.get(2)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    // 最近 30 天每日趋势
    let mut stmt2 = conn
        .prepare(
            "SELECT date(created_at) as d, COUNT(*) as cnt
             FROM action_history
             WHERE created_at >= datetime('now', '-30 days', 'localtime')
             GROUP BY d
             ORDER BY d ASC",
        )
        .map_err(|e| format!("Prepare failed: {}", e))?;

    let daily_trend: Vec<DailyStat> = stmt2
        .query_map([], |row| {
            Ok(DailyStat {
                date: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(ActionStats {
        total_count,
        total_duration_ms,
        top_actions,
        daily_trend,
    })
}

/// 按操作 ID 统计使用次数
pub fn action_usage_stats() -> Result<Vec<(String, i64)>, String> {
    let db = get_db();
    let conn = db.lock().map_err(|e| format!("数据库锁获取失败: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT action_id, COUNT(*) as cnt
             FROM action_history
             GROUP BY action_id
             ORDER BY cnt DESC",
        )
        .map_err(|e| format!("查询准备失败: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("查询执行失败: {}", e))?;

    let mut stats = Vec::new();
    for row in rows {
        stats.push(row.map_err(|e| format!("行解析失败: {}", e))?);
    }
    Ok(stats)
}
