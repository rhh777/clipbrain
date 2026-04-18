use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;

use super::schema::PluginConfig;
use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use crate::model::backend::ChatMessage;
use crate::model::state;

/// 由 TOML + Prompt 模板定义的插件操作
pub struct PluginAction {
    config: PluginConfig,
    /// 插件文件路径（用于热重载判断）
    #[allow(dead_code)]
    source_path: PathBuf,
}

impl PluginAction {
    pub fn new(config: PluginConfig, source_path: PathBuf) -> Self {
        Self {
            config,
            source_path,
        }
    }

    pub fn plugin_name(&self) -> &str {
        &self.config.plugin.name
    }

    /// 将 content_type 字符串转为 ContentType 枚举
    fn parse_content_type(s: &str) -> Option<ContentType> {
        match s {
            "Json" => Some(ContentType::Json),
            "Yaml" => Some(ContentType::Yaml),
            "Url" => Some(ContentType::Url),
            "Email" => Some(ContentType::Email),
            "PhoneNumber" => Some(ContentType::PhoneNumber),
            "IdCard" => Some(ContentType::IdCard),
            "MathExpression" => Some(ContentType::MathExpression),
            "FileList" => Some(ContentType::FileList),
            "PlainText" => Some(ContentType::PlainText),
            s if s.starts_with("Code") => Some(ContentType::Code("".to_string())),
            s if s.starts_with("TableData") => Some(ContentType::TableData("".to_string())),
            _ => None,
        }
    }

    /// 渲染模板，替换 {{content}} 占位符
    fn render_template(template: &str, content: &str) -> String {
        template.replace("{{content}}", content)
    }
}

#[async_trait]
impl Action for PluginAction {
    fn id(&self) -> &str {
        &self.config.plugin.id
    }

    fn display_name(&self) -> &str {
        &self.config.plugin.name
    }

    fn description(&self) -> &str {
        &self.config.plugin.description
    }

    fn supported_types(&self) -> Vec<ContentType> {
        self.config
            .trigger
            .content_types
            .iter()
            .filter_map(|s| Self::parse_content_type(s))
            .collect()
    }

    fn requires_model(&self) -> bool {
        true
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(8)
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let system_prompt =
            Self::render_template(&self.config.action.system_prompt, &input.content);

        let user_content = if let Some(ref tpl) = self.config.action.user_prompt {
            Self::render_template(tpl, &input.content)
        } else {
            input.content
        };

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ];

        let resp = state::chat(
            messages,
            self.config.action.max_tokens,
            self.config.action.temperature,
        )
        .await?;

        Ok(ActionOutput {
            result: resp.content,
            result_type: self.config.action.output_type.clone(),
        })
    }
}
