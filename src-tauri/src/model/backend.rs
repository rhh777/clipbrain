use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// 流式输出块
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// 模型思考内容（reasoning_content）
    Thinking(String),
    /// 正文内容增量
    Delta(String),
}

/// 流式发送器类型别名
pub type StreamSender = mpsc::UnboundedSender<StreamChunk>;

/// 聊天请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    /// 是否启用深度思考（None = 由模型决定, Some(false) = 关闭思考）
    pub thinking: Option<bool>,
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant"
    pub content: String,
}

/// 聊天响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
}

/// Token 使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 视觉请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionRequest {
    pub messages: Vec<VisionMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// 视觉消息（支持图片 + 文本混合内容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionMessage {
    pub role: String,
    pub content: Vec<VisionContentPart>,
}

/// 视觉内容部分
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VisionContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrlDetail },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrlDetail {
    pub url: String, // base64 data URL or http URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>, // "low" | "high" | "auto"
}

/// 后端类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendType {
    Rules,
    Remote(String), // backend name
    Local(String),  // "llama_cpp" | "mlx"
}

/// 推理后端 trait — 本地和远程统一接口
#[async_trait]
pub trait InferenceBackend: Send + Sync {
    /// 文本补全
    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse, String>;

    /// 视觉推理（图片 + 文本）
    async fn vision_completion(&self, _request: VisionRequest) -> Result<ChatResponse, String> {
        Err("Vision not supported by this backend".to_string())
    }

    /// 后端类型标识
    fn backend_type(&self) -> BackendType;

    /// 流式文本补全（默认回退到非流式）
    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
        sender: StreamSender,
    ) -> Result<ChatResponse, String> {
        let resp = self.chat_completion(request).await?;
        let _ = sender.send(StreamChunk::Delta(resp.content.clone()));
        Ok(resp)
    }

    /// 是否就绪
    async fn health_check(&self) -> Result<bool, String>;
}
