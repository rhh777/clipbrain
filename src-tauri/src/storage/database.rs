use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};

use super::migrations;

/// 全局数据库连接
static DB: LazyLock<Arc<Mutex<Connection>>> = LazyLock::new(|| {
    let conn = init_database().expect("数据库初始化失败");
    Arc::new(Mutex::new(conn))
});

/// 获取数据库连接的 Arc 引用
pub fn get_db() -> Arc<Mutex<Connection>> {
    DB.clone()
}

/// 获取数据库文件路径：~/.clipbrain/clipbrain.db
fn db_path() -> PathBuf {
    let base = dirs::home_dir().expect("无法获取 HOME 目录");
    let dir = base.join(".clipbrain");
    std::fs::create_dir_all(&dir).expect("无法创建 .clipbrain 目录");
    dir.join("clipbrain.db")
}

/// 初始化数据库：创建文件、执行迁移
fn init_database() -> Result<Connection, rusqlite::Error> {
    let path = db_path();
    log::info!("数据库路径: {}", path.display());

    let conn = Connection::open(&path)?;

    // 启用 WAL 模式提升并发性能
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    // 启用外键约束
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    // 执行数据库迁移
    migrations::run_migrations(&conn)?;

    log::info!("数据库初始化完成");
    Ok(conn)
}
