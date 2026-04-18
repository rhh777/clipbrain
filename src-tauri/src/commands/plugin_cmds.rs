use crate::actions::plugin::loader;
use crate::actions::plugin::store;
use serde::Serialize;

/// 插件信息（供前端展示）
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub content_types: Vec<String>,
}

/// 列出已安装的插件
#[tauri::command]
pub fn list_plugins() -> Vec<PluginInfo> {
    let dir = loader::plugins_dir();
    let mut plugins = Vec::new();

    if !dir.exists() {
        return plugins;
    }

    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let path = entry.path();
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

        if let Ok(content) = std::fs::read_to_string(&toml_path) {
            if let Ok(config) =
                toml::from_str::<crate::actions::plugin::schema::PluginConfig>(&content)
            {
                plugins.push(PluginInfo {
                    id: config.plugin.id,
                    name: config.plugin.name,
                    description: config.plugin.description,
                    version: config.plugin.version,
                    content_types: config.trigger.content_types,
                });
            }
        }
    }

    plugins
}

/// 获取插件目录路径
#[tauri::command]
pub fn get_plugins_dir() -> String {
    loader::plugins_dir().to_string_lossy().to_string()
}

/// 重新加载所有插件（热重载入口）
#[tauri::command]
pub fn reload_plugins() -> Result<usize, String> {
    let count = crate::commands::action_cmds::reload_registry_plugins();
    Ok(count)
}

/// 拉取社区插件仓库索引
#[tauri::command]
pub async fn fetch_store_index() -> Result<store::StoreIndex, String> {
    store::fetch_store_index().await
}

/// 从仓库安装插件（内置插件直接写入，非内置走网络下载）
#[tauri::command]
pub async fn install_store_plugin(plugin_id: String) -> Result<(), String> {
    if store::is_builtin_plugin(&plugin_id) {
        return store::install_builtin_plugin(&plugin_id);
    }
    let index = store::fetch_store_index().await?;
    let entry = index
        .plugins
        .iter()
        .find(|p| p.id == plugin_id)
        .ok_or_else(|| format!("Plugin '{}' not found in store", plugin_id))?;
    store::install_plugin(entry).await
}

/// 卸载插件
#[tauri::command]
pub fn uninstall_plugin(plugin_id: String) -> Result<(), String> {
    store::uninstall_plugin(&plugin_id)
}

/// 获取已安装插件 ID 列表
#[tauri::command]
pub fn installed_plugin_ids() -> Vec<String> {
    store::installed_plugin_ids()
}
