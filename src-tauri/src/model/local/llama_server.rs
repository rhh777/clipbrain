use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::model::backend::{
    BackendType, ChatMessage, ChatRequest, ChatResponse, InferenceBackend, TokenUsage,
};

/// llama.cpp server 后端 — 通过 HTTP API 与 llama-server 进程通信
/// llama-server 提供 OpenAI 兼容的 /v1/chat/completions 接口
pub struct LlamaServerBackend {
    /// llama-server 监听地址，如 http://127.0.0.1:8080
    base_url: String,
    /// 模型名称（仅用于标识）
    model_name: String,
    client: reqwest::Client,
}

impl LlamaServerBackend {
    pub fn new(base_url: &str, model_name: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model_name: model_name.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[derive(Serialize)]
struct LlamaRequest {
    model: String,
    messages: Vec<LlamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Serialize)]
struct LlamaMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct LlamaResponse {
    choices: Vec<LlamaChoice>,
    model: Option<String>,
    usage: Option<LlamaUsage>,
}

#[derive(Deserialize)]
struct LlamaChoice {
    message: LlamaMessageResp,
}

#[derive(Deserialize)]
struct LlamaMessageResp {
    content: String,
}

#[derive(Deserialize)]
struct LlamaUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[async_trait]
impl InferenceBackend for LlamaServerBackend {
    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let body = LlamaRequest {
            model: self.model_name.clone(),
            messages: request
                .messages
                .into_iter()
                .map(|m| LlamaMessage {
                    role: m.role,
                    content: m.content,
                })
                .collect(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: false,
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("llama-server 请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("llama-server 返回错误 {}: {}", status, text));
        }

        let data: LlamaResponse = resp
            .json()
            .await
            .map_err(|e| format!("llama-server 响应解析失败: {}", e))?;

        let content = data
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = data.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens.unwrap_or(0),
            completion_tokens: u.completion_tokens.unwrap_or(0),
            total_tokens: u.total_tokens.unwrap_or(0),
        });

        Ok(ChatResponse {
            content,
            model: data.model.unwrap_or_else(|| self.model_name.clone()),
            usage,
        })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Local("llama_cpp".to_string())
    }

    async fn health_check(&self) -> Result<bool, String> {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}
