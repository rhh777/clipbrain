use super::schema::AppConfig;
use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// 全局配置状态
static CONFIG: LazyLock<Mutex<AppConfig>> = LazyLock::new(|| {
    let config = load_from_file().unwrap_or_default();
    Mutex::new(config)
});

/// 获取配置文件路径: ~/Library/Application Support/com.clipbrain.app/config.toml
fn config_path() -> PathBuf {
    let base = dirs_next().unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("com.clipbrain.app");
    let _ = fs::create_dir_all(&dir);
    dir.join("config.toml")
}

/// macOS: ~/Library/Application Support
fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::config_dir()
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::config_dir()
    }
}

/// 从文件加载配置
fn load_from_file() -> Option<AppConfig> {
    let path = config_path();
    if !path.exists() {
        println!("[ClipBrain] 配置文件不存在，使用默认配置");
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    match toml::from_str::<AppConfig>(&content) {
        Ok(config) => {
            println!("[ClipBrain] 配置已从 {} 加载", path.display());
            Some(config)
        }
        Err(e) => {
            println!("[ClipBrain] 配置解析失败: {}，使用默认配置", e);
            None
        }
    }
}

/// 保存配置到文件
fn save_to_file(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    let content = toml::to_string_pretty(config).map_err(|e| format!("配置序列化失败: {}", e))?;
    fs::write(&path, content).map_err(|e| format!("配置写入失败: {}", e))?;
    println!("[ClipBrain] 配置已保存到 {}", path.display());
    Ok(())
}

/// 获取当前配置的克隆
pub fn get() -> AppConfig {
    CONFIG.lock().unwrap().clone()
}

/// 更新配置并持久化
pub fn update(new_config: AppConfig) -> Result<(), String> {
    save_to_file(&new_config)?;
    let mut config = CONFIG.lock().map_err(|e| e.to_string())?;
    *config = new_config;
    Ok(())
}

/// 更新单个字段（通过闭包）
pub fn update_with<F>(f: F) -> Result<(), String>
where
    F: FnOnce(&mut AppConfig),
{
    let mut config = CONFIG.lock().map_err(|e| e.to_string())?;
    f(&mut config);
    save_to_file(&config)?;
    Ok(())
}

/// 重新从文件加载
pub fn reload() -> Result<(), String> {
    let new_config = load_from_file().unwrap_or_default();
    let mut config = CONFIG.lock().map_err(|e| e.to_string())?;
    *config = new_config;
    Ok(())
}
