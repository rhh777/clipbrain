use serde::{Deserialize, Serialize};

/// 插件 TOML 配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub plugin: PluginMeta,
    pub trigger: PluginTrigger,
    pub action: PluginActionDef,
}

/// 插件元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    /// 插件唯一 ID（用于注册到 ActionRegistry）
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 版本
    pub version: String,
    /// 作者（可选）
    pub author: Option<String>,
}

/// 触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTrigger {
    /// 支持的内容类型列表, e.g. ["PlainText", "Code"]
    pub content_types: Vec<String>,
}

/// 操作定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginActionDef {
    /// system prompt 模板，支持 {{content}} 占位符
    pub system_prompt: String,
    /// user prompt 模板（可选），默认使用 {{content}}
    pub user_prompt: Option<String>,
    /// 输出格式: "text" | "markdown" | "json" | "code"
    #[serde(default = "default_output_type")]
    pub output_type: String,
    /// 最大 token 数
    pub max_tokens: Option<u32>,
    /// 温度
    pub temperature: Option<f32>,
}

fn default_output_type() -> String {
    "text".to_string()
}
