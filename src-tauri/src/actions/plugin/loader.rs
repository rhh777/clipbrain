use std::path::PathBuf;
use std::sync::Arc;

use super::plugin_action::PluginAction;
use super::schema::PluginConfig;
use crate::actions::traits::Action;

/// 插件目录: ~/.clipbrain/plugins/
pub fn plugins_dir() -> PathBuf {
    let base = dirs::home_dir().expect("无法获取 HOME 目录");
    let dir = base.join(".clipbrain").join("plugins");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// 扫描插件目录，解析所有 plugin.toml 并返回 PluginAction 列表
pub fn load_all_plugins() -> Vec<Arc<dyn Action>> {
    let dir = plugins_dir();
    let mut actions: Vec<Arc<dyn Action>> = Vec::new();

    if !dir.exists() {
        log::info!("插件目录不存在: {}", dir.display());
        return actions;
    }

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            log::warn!("读取插件目录失败: {}", e);
            return actions;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // 每个子目录代表一个插件，或者直接是 .toml 文件
        let toml_path = if path.is_dir() {
            path.join("plugin.toml")
        } else if path.extension().map(|e| e == "toml").unwrap_or(false) {
            path.clone()
        } else {
            continue;
        };

        if !toml_path.exists() {
            continue;
        }

        match load_plugin(&toml_path) {
            Ok(action) => {
                log::info!(
                    "已加载插件: {} ({})",
                    action.plugin_name(),
                    toml_path.display()
                );
                actions.push(Arc::new(action));
            }
            Err(e) => {
                log::warn!("加载插件失败 {}: {}", toml_path.display(), e);
            }
        }
    }

    log::info!("共加载 {} 个插件", actions.len());
    actions
}

/// 从 TOML 文件加载单个插件
fn load_plugin(path: &std::path::Path) -> Result<PluginAction, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("读取插件文件失败: {}", e))?;

    let config: PluginConfig =
        toml::from_str(&content).map_err(|e| format!("解析插件 TOML 失败: {}", e))?;

    // 验证必要字段
    if config.plugin.id.is_empty() {
        return Err("插件 ID 不能为空".to_string());
    }
    if config.action.system_prompt.is_empty() {
        return Err("system_prompt 不能为空".to_string());
    }

    Ok(PluginAction::new(config, path.to_path_buf()))
}
