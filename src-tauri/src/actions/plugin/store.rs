use super::loader::plugins_dir;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 仓库索引中的插件条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorePluginEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub download_url: String,
    pub content_types: Vec<String>,
    #[serde(default)]
    pub downloads: u64,
}

/// 仓库索引
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreIndex {
    pub version: String,
    pub plugins: Vec<StorePluginEntry>,
}

/// 默认社区仓库 URL
const DEFAULT_STORE_URL: &str =
    "https://raw.githubusercontent.com/clipbrain/plugin-store/main/index.json";

// ─── 内置插件 ───────────────────────────────────────────

const BUILTIN_SQL_FORMAT_TOML: &str = r#"
[plugin]
id = "sql_format"
name = "SQL Format"
description = "Format and beautify SQL queries with proper indentation and keyword casing."
version = "1.0.0"
author = "ClipBrain"

[trigger]
content_types = ["PlainText", "Code"]

[action]
system_prompt = "You are an expert SQL formatter. Format the given SQL query with proper indentation, uppercase keywords, and consistent style. Only output the formatted SQL, nothing else."
user_prompt = "{{content}}"
output_type = "code"
max_tokens = 4096
temperature = 0.0
"#;

/// 返回内置插件条目列表
fn builtin_plugins() -> Vec<(StorePluginEntry, &'static str)> {
    vec![(
        StorePluginEntry {
            id: "sql_format".to_string(),
            name: "SQL Format".to_string(),
            description:
                "Format and beautify SQL queries with proper indentation and keyword casing."
                    .to_string(),
            version: "1.0.0".to_string(),
            author: "ClipBrain".to_string(),
            download_url: String::new(), // 内置插件无需下载
            content_types: vec!["PlainText".to_string(), "Code".to_string()],
            downloads: 0,
        },
        BUILTIN_SQL_FORMAT_TOML,
    )]
}

/// 安装内置插件（直接写入 TOML，无需网络）
pub fn install_builtin_plugin(plugin_id: &str) -> Result<(), String> {
    let builtins = builtin_plugins();
    let (entry, toml_content) = builtins
        .iter()
        .find(|(e, _)| e.id == plugin_id)
        .ok_or_else(|| format!("Builtin plugin '{}' not found", plugin_id))?;

    let content = toml_content.trim();
    // 验证 TOML
    let _config: super::schema::PluginConfig =
        toml::from_str(content).map_err(|e| format!("Invalid builtin plugin TOML: {}", e))?;

    let plugin_dir = plugins_dir().join(&entry.id);
    std::fs::create_dir_all(&plugin_dir)
        .map_err(|e| format!("Failed to create plugin dir: {}", e))?;

    let toml_path = plugin_dir.join("plugin.toml");
    std::fs::write(&toml_path, content)
        .map_err(|e| format!("Failed to write plugin file: {}", e))?;

    println!(
        "[ClipBrain] Installed builtin plugin: {} v{}",
        entry.name, entry.version
    );
    Ok(())
}

/// 判断是否为内置插件
pub fn is_builtin_plugin(plugin_id: &str) -> bool {
    builtin_plugins().iter().any(|(e, _)| e.id == plugin_id)
}

/// 获取仓库 URL（可通过配置覆盖）
fn store_url() -> String {
    // 可后续从配置读取自定义仓库 URL
    DEFAULT_STORE_URL.to_string()
}

/// 拉取远程仓库索引，并合并内置插件
pub async fn fetch_store_index() -> Result<StoreIndex, String> {
    let url = store_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    // 尝试拉取远程索引，失败则使用空列表
    let mut remote_plugins = match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<StoreIndex>()
            .await
            .map(|idx| idx.plugins)
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    // 合并内置插件（内置优先，不重复）
    let builtin_entries: Vec<StorePluginEntry> = builtin_plugins()
        .into_iter()
        .map(|(entry, _)| entry)
        .collect();

    let remote_ids: std::collections::HashSet<String> =
        remote_plugins.iter().map(|p| p.id.clone()).collect();
    for entry in builtin_entries {
        if !remote_ids.contains(&entry.id) {
            remote_plugins.insert(0, entry); // 内置插件排在前面
        }
    }

    Ok(StoreIndex {
        version: "1".to_string(),
        plugins: remote_plugins,
    })
}

/// 下载并安装插件
pub async fn install_plugin(entry: &StorePluginEntry) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(&entry.download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download plugin: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Download returned status {}", resp.status()));
    }

    let content = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read plugin content: {}", e))?;

    // 验证 TOML 是否合法
    let _config: super::schema::PluginConfig =
        toml::from_str(&content).map_err(|e| format!("Invalid plugin TOML: {}", e))?;

    // 保存到插件目录
    let plugin_dir = plugins_dir().join(&entry.id);
    std::fs::create_dir_all(&plugin_dir)
        .map_err(|e| format!("Failed to create plugin dir: {}", e))?;

    let toml_path = plugin_dir.join("plugin.toml");
    std::fs::write(&toml_path, &content)
        .map_err(|e| format!("Failed to write plugin file: {}", e))?;

    println!(
        "[ClipBrain] Installed plugin: {} v{}",
        entry.name, entry.version
    );
    Ok(())
}

/// 卸载插件
pub fn uninstall_plugin(plugin_id: &str) -> Result<(), String> {
    let dir = plugins_dir();

    // 尝试删除目录
    let plugin_dir = dir.join(plugin_id);
    if plugin_dir.is_dir() {
        std::fs::remove_dir_all(&plugin_dir)
            .map_err(|e| format!("Failed to remove plugin dir: {}", e))?;
        return Ok(());
    }

    // 或直接删除 .toml 文件
    let toml_path = dir.join(format!("{}.toml", plugin_id));
    if toml_path.exists() {
        std::fs::remove_file(&toml_path)
            .map_err(|e| format!("Failed to remove plugin file: {}", e))?;
        return Ok(());
    }

    Err(format!("Plugin '{}' not found", plugin_id))
}

/// 列出已安装插件的 ID 集合
pub fn installed_plugin_ids() -> Vec<String> {
    let dir = plugins_dir();
    let mut ids = Vec::new();

    if !dir.exists() {
        return ids;
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
            if let Ok(config) = toml::from_str::<super::schema::PluginConfig>(&content) {
                ids.push(config.plugin.id);
            }
        }
    }

    ids
}
