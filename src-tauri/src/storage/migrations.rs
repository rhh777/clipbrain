use std::collections::HashMap;

use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

/// 数据库版本号
const CURRENT_VERSION: u32 = 3;

/// 执行数据库迁移
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    // 创建版本跟踪表
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER NOT NULL DEFAULT 0
        );",
    )?;

    let version: u32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    log::info!("当前数据库版本: {}, 目标版本: {}", version, CURRENT_VERSION);

    if version < 1 {
        migrate_v1(conn)?;
    }
    if version < 2 {
        migrate_v2(conn)?;
    }
    if version < 3 {
        migrate_v3(conn)?;
    }

    Ok(())
}

/// V1: 创建核心表 — clipboard_history, action_history, custom_prompts
fn migrate_v1(conn: &Connection) -> Result<(), rusqlite::Error> {
    log::info!("执行迁移 V1: 创建核心表");

    conn.execute_batch(
        "
        -- 剪贴板历史记录
        CREATE TABLE IF NOT EXISTS clipboard_history (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            content     TEXT,
            image_path  TEXT,
            content_type TEXT NOT NULL,
            source_app  TEXT,
            char_count  INTEGER,
            created_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            is_pinned   INTEGER NOT NULL DEFAULT 0,
            is_sensitive INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_history_created ON clipboard_history(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_history_type ON clipboard_history(content_type);
        CREATE INDEX IF NOT EXISTS idx_history_pinned ON clipboard_history(is_pinned);

        -- 操作执行记录
        CREATE TABLE IF NOT EXISTS action_history (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            clipboard_id INTEGER REFERENCES clipboard_history(id) ON DELETE SET NULL,
            action_id    TEXT NOT NULL,
            input_text   TEXT,
            output_text  TEXT,
            duration_ms  INTEGER,
            model_used   TEXT,
            created_at   TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        CREATE INDEX IF NOT EXISTS idx_action_history_action ON action_history(action_id);
        CREATE INDEX IF NOT EXISTS idx_action_history_created ON action_history(created_at DESC);

        -- 用户自定义 prompt 模板
        CREATE TABLE IF NOT EXISTS custom_prompts (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL,
            description  TEXT,
            prompt       TEXT NOT NULL,
            content_types TEXT,
            sort_order   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            updated_at   TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
        );

        -- 记录版本
        DELETE FROM schema_version;
        INSERT INTO schema_version (version) VALUES (1);
        ",
    )?;

    log::info!("迁移 V1 完成");
    Ok(())
}

/// V2: 创建 clipboard_tags 表（用户自定义标签系统）
fn migrate_v2(conn: &Connection) -> Result<(), rusqlite::Error> {
    log::info!("执行迁移 V2: 创建 clipboard_tags 表");

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS clipboard_tags (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            clipboard_id INTEGER NOT NULL REFERENCES clipboard_history(id) ON DELETE CASCADE,
            tag_name     TEXT NOT NULL,
            created_at   TEXT NOT NULL DEFAULT (datetime('now', 'localtime')),
            UNIQUE(clipboard_id, tag_name)
        );

        CREATE INDEX IF NOT EXISTS idx_tags_clipboard ON clipboard_tags(clipboard_id);
        CREATE INDEX IF NOT EXISTS idx_tags_name ON clipboard_tags(tag_name);

        -- 更新版本
        DELETE FROM schema_version;
        INSERT INTO schema_version (version) VALUES (2);
        ",
    )?;

    log::info!("迁移 V2 完成");
    Ok(())
}

/// V3: 为图片历史增加 image_hash，并清理重复图片记录
fn migrate_v3(conn: &Connection) -> Result<(), rusqlite::Error> {
    log::info!("执行迁移 V3: 图片历史永久去重");

    if !column_exists(conn, "clipboard_history", "image_hash")? {
        conn.execute_batch("ALTER TABLE clipboard_history ADD COLUMN image_hash TEXT;")?;
    }

    backfill_and_deduplicate_image_history(conn).map_err(|e| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            e,
        )))
    })?;

    conn.execute_batch(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS idx_history_image_hash_unique
        ON clipboard_history(image_hash)
        WHERE image_hash IS NOT NULL;

        DELETE FROM schema_version;
        INSERT INTO schema_version (version) VALUES (3);
        ",
    )?;

    log::info!("迁移 V3 完成");
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for row in rows {
        if row? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

#[derive(Clone)]
struct ImageRow {
    id: i64,
    image_path: String,
    created_at: String,
    is_pinned: bool,
}

fn backfill_and_deduplicate_image_history(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, image_path, created_at, is_pinned
             FROM clipboard_history
             WHERE image_path IS NOT NULL",
        )
        .map_err(|e| format!("查询图片历史失败: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ImageRow {
                id: row.get(0)?,
                image_path: row.get(1)?,
                created_at: row.get(2)?,
                is_pinned: row.get::<_, i32>(3)? != 0,
            })
        })
        .map_err(|e| format!("读取图片历史失败: {}", e))?;

    let rows = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("解析图片历史失败: {}", e))?;

    let mut winners: HashMap<String, ImageRow> = HashMap::new();
    let mut losers: Vec<i64> = Vec::new();

    for row in rows {
        let hash = match hash_image_rgba(&row.image_path) {
            Ok(hash) => hash,
            Err(err) => {
                log::warn!("跳过无法回填哈希的图片历史 {}: {}", row.id, err);
                continue;
            }
        };
        if let Some(existing) = winners.get(&hash).cloned() {
            if prefer_image_row(&row, &existing) {
                losers.push(existing.id);
                winners.insert(hash, row);
            } else {
                losers.push(row.id);
            }
        } else {
            winners.insert(hash, row);
        }
    }

    for (hash, row) in winners {
        conn.execute(
            "UPDATE clipboard_history SET image_hash = ?1 WHERE id = ?2",
            params![hash, row.id],
        )
        .map_err(|e| format!("写入图片哈希失败: {}", e))?;
    }

    for id in losers {
        conn.execute("DELETE FROM clipboard_history WHERE id = ?1", params![id])
            .map_err(|e| format!("删除重复图片历史失败: {}", e))?;
    }

    Ok(())
}

fn prefer_image_row(candidate: &ImageRow, existing: &ImageRow) -> bool {
    if candidate.is_pinned != existing.is_pinned {
        return candidate.is_pinned;
    }
    candidate.created_at > existing.created_at
}

fn hash_image_rgba(path: &str) -> Result<String, String> {
    let img = image::ImageReader::open(path)
        .map_err(|e| format!("打开图片失败 ({}): {}", path, e))?
        .with_guessed_format()
        .map_err(|e| format!("识别图片格式失败 ({}): {}", path, e))?
        .decode()
        .map_err(|e| format!("解码图片失败 ({}): {}", path, e))?
        .into_rgba8();

    let mut hasher = Sha256::new();
    hasher.update(img.as_raw());
    Ok(format!("{:x}", hasher.finalize()))
}
