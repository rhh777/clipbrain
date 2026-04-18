use crate::classifier::rules::ContentType;
use crate::model::backend::{StreamChunk, StreamSender};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 操作输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionInput {
    pub content: String,
    pub content_type: ContentType,
    /// 是否启用深度思考（None = 由模型决定, Some(false) = 关闭思考加速响应）
    #[serde(default)]
    pub thinking: Option<bool>,
}

/// 操作输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionOutput {
    pub result: String,
    pub result_type: String, // "text", "code", "markdown", etc.
}

/// 操作描述 — 用于前端展示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDescriptor {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub action_scope: String,
    pub requires_model: bool,
    pub estimated_duration_ms: u64,
}

/// Action trait — 所有操作的统一抽象
#[async_trait]
pub trait Action: Send + Sync {
    /// 操作唯一标识
    fn id(&self) -> &str;

    /// 显示名称
    fn display_name(&self) -> &str;

    /// 显示名称（英文），默认 fallback 到 display_name()
    fn display_name_en(&self) -> &str {
        self.display_name()
    }

    /// 描述
    fn description(&self) -> &str;

    /// 描述（英文），默认 fallback 到 description()
    fn description_en(&self) -> &str {
        self.description()
    }

    /// 此操作支持哪些内容类型
    fn supported_types(&self) -> Vec<ContentType>;

    /// 是否需要 LLM
    fn requires_model(&self) -> bool;

    /// 执行操作
    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String>;

    /// 预估耗时
    fn estimated_duration(&self) -> Duration;

    /// 流式执行操作（默认回退到非流式执行）
    async fn execute_stream(
        &self,
        input: ActionInput,
        sender: StreamSender,
    ) -> Result<ActionOutput, String> {
        let output = self.execute(input).await?;
        let _ = sender.send(StreamChunk::Delta(output.result.clone()));
        Ok(output)
    }

    /// 转为描述对象（供前端使用），根据 locale 选择语言
    fn to_descriptor(&self, locale: &str) -> ActionDescriptor {
        let (name, desc) = if locale.starts_with("en") {
            (
                self.display_name_en().to_string(),
                self.description_en().to_string(),
            )
        } else {
            (
                self.display_name().to_string(),
                self.description().to_string(),
            )
        };
        ActionDescriptor {
            id: self.id().to_string(),
            display_name: name,
            description: desc,
            action_scope: "specific".to_string(),
            requires_model: self.requires_model(),
            estimated_duration_ms: self.estimated_duration().as_millis() as u64,
        }
    }
}
