use serde::{Deserialize, Serialize};

/// 应用配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub hotkey: HotkeyConfig,
    pub popup: PopupConfig,
    pub model: ModelConfig,
    pub privacy: PrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub trigger_mode: String,    // "hotkey" | "auto_popup" | "both"
    pub capability_mode: String, // "rules_only" | "remote_api" | "local_model"
    pub locale: String,
    pub auto_start: bool,
    #[serde(default = "default_history_limit")]
    pub history_limit: u32,
    #[serde(default = "default_true")]
    pub show_detail_panel_by_default: bool,
    #[serde(default)]
    pub show_search_toolbar_buttons: bool,
    #[serde(default)]
    pub clear_inputs_on_panel_open: bool,
    #[serde(default = "default_true")]
    pub show_item_meta: bool,
}

fn default_history_limit() -> u32 {
    500
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupConfig {
    /// 弹窗位置: "cursor" | "top_right" | "bottom_right"
    pub position: String,
    /// 自动消失时间(ms), 0 表示不自动消失
    pub auto_dismiss_ms: u32,
    /// 弹窗最大宽度
    pub max_width: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub open_panel: String,
    pub quick_translate: String,
    pub quick_summarize: String,
    /// 用户自定义快捷操作绑定
    #[serde(default)]
    pub quick_actions: Vec<QuickActionBinding>,
}

/// 快捷操作绑定：快捷键 → 操作 ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickActionBinding {
    /// 显示名称
    pub label: String,
    /// 操作 ID（如 "translate_to_chinese", "json_format" 等）
    pub action_id: String,
    /// 快捷键字符串（如 "CommandOrControl+Shift+T"）
    pub shortcut: String,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub default_backend: String,
    pub remote_backends: Vec<RemoteBackendConfig>,
    #[serde(default)]
    pub local: LocalModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalModelConfig {
    /// 本地推理引擎: "llama_cpp" | "mlx" | "none"
    #[serde(default = "default_local_engine")]
    pub engine: String,
    /// llama-server / mlx-lm server 监听地址
    #[serde(default = "default_local_url")]
    pub server_url: String,
    /// 默认本地模型名称
    #[serde(default)]
    pub default_model: String,
    /// 模型文件存放目录
    #[serde(default = "default_model_dir")]
    pub model_dir: String,
    /// 远程失败时是否回退到本地
    #[serde(default)]
    pub fallback_to_local: bool,
}

fn default_local_engine() -> String {
    "none".to_string()
}
fn default_local_url() -> String {
    "http://127.0.0.1:8080".to_string()
}
fn default_model_dir() -> String {
    dirs::home_dir()
        .map(|h| {
            h.join(".clipbrain")
                .join("models")
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|| "~/.clipbrain/models".to_string())
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            engine: default_local_engine(),
            server_url: default_local_url(),
            default_model: String::new(),
            model_dir: default_model_dir(),
            fallback_to_local: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteBackendConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub timeout: u64,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub excluded_apps: Vec<String>,
    pub log_sensitive: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                trigger_mode: "hotkey".to_string(),
                capability_mode: "rules_only".to_string(),
                locale: "zh-CN".to_string(),
                auto_start: false,
                history_limit: 500,
                show_detail_panel_by_default: true,
                show_search_toolbar_buttons: false,
                clear_inputs_on_panel_open: false,
                show_item_meta: true,
            },
            popup: PopupConfig {
                position: "top_right".to_string(),
                auto_dismiss_ms: 3000,
                max_width: 400,
            },
            hotkey: HotkeyConfig {
                open_panel: "Alt+CommandOrControl+C".to_string(),
                quick_translate: "CommandOrControl+Shift+T".to_string(),
                quick_summarize: "CommandOrControl+Shift+S".to_string(),
                quick_actions: vec![],
            },
            model: ModelConfig {
                default_backend: "rules".to_string(),
                remote_backends: vec![],
                local: LocalModelConfig::default(),
            },
            privacy: PrivacyConfig {
                excluded_apps: vec![
                    "1Password".to_string(),
                    "Bitwarden".to_string(),
                    "KeePassXC".to_string(),
                ],
                log_sensitive: false,
            },
        }
    }
}
