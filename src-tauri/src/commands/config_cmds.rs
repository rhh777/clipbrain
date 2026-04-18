use crate::config::manager;
use crate::config::schema::AppConfig;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// 获取当前配置
#[tauri::command]
pub fn get_config() -> AppConfig {
    manager::get()
}

/// 保存配置
#[tauri::command]
pub fn save_config(app: AppHandle, config: AppConfig) -> Result<(), String> {
    let auto_start = config.general.auto_start;
    manager::update(config)?;

    let autostart = app.autolaunch();
    if auto_start {
        autostart
            .enable()
            .map_err(|e| format!("启用开机自启动失败: {}", e))?;
    } else {
        autostart
            .disable()
            .map_err(|e| format!("关闭开机自启动失败: {}", e))?;
    }

    Ok(())
}

/// 重新加载配置
#[tauri::command]
pub fn reload_config() -> Result<AppConfig, String> {
    manager::reload()?;
    Ok(manager::get())
}

/// 首次引导标记文件路径
fn onboarding_flag_path() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_default();
    base.join(".clipbrain").join(".onboarding_done")
}

/// 检测是否为首次启动（未完成引导）
#[tauri::command]
pub fn is_first_launch() -> bool {
    !onboarding_flag_path().exists()
}

/// 标记引导已完成
#[tauri::command]
pub fn complete_onboarding() -> Result<(), String> {
    let path = onboarding_flag_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, "done").map_err(|e| format!("写入标记文件失败: {}", e))?;
    Ok(())
}
