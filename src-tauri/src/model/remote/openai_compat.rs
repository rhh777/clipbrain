use crate::model::backend::*;
use async_trait::async_trait;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 远程 API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub timeout_secs: u64,
    pub max_tokens: u32,
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: None,
            model: "gpt-4o".to_string(),
            timeout_secs: 30,
            max_tokens: 2048,
        }
    }
}

/// OpenAI 兼容 API 客户端
pub struct OpenAICompatClient {
    config: RemoteConfig,
    client: reqwest::Client,
}

impl OpenAICompatClient {
    pub fn new(config: RemoteConfig) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| format!("HTTP 客户端初始化失败: {}", e))?;
        Ok(Self { config, client })
    }
}

#[async_trait]
impl InferenceBackend for OpenAICompatClient {
    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse, String> {
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let enable_thinking = match request.thinking {
            Some(false) => Some(false),
            _ => None,
        };

        let openai_req = OpenAIRequest {
            model: self.config.model.clone(),
            messages: request
                .messages
                .iter()
                .map(|m| OpenAIMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            max_tokens: request.max_tokens.or(Some(self.config.max_tokens)),
            temperature: request.temperature,
            stream: None,
            enable_thinking,
        };

        let mut req_builder = self.client.post(&url).json(&openai_req);

        if let Some(ref key) = self.config.api_key {
            if !key.is_empty() {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
            }
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API 错误 ({}): {}", status, body));
        }

        let openai_resp: OpenAIResponse = resp
            .json()
            .await
            .map_err(|e| format!("响应解析失败: {}", e))?;

        let choice = openai_resp.choices.first().ok_or("API 返回空结果")?;

        Ok(ChatResponse {
            content: choice.message.content.clone(),
            model: openai_resp.model,
            usage: openai_resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    async fn vision_completion(&self, request: VisionRequest) -> Result<ChatResponse, String> {
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                let content_parts: Vec<serde_json::Value> = m
                    .content
                    .iter()
                    .map(|part| match part {
                        VisionContentPart::Text { text } => serde_json::json!({
                            "type": "text",
                            "text": text
                        }),
                        VisionContentPart::ImageUrl { image_url } => {
                            let mut img = serde_json::json!({
                                "url": image_url.url
                            });
                            if let Some(ref detail) = image_url.detail {
                                img["detail"] = serde_json::json!(detail);
                            }
                            serde_json::json!({
                                "type": "image_url",
                                "image_url": img
                            })
                        }
                    })
                    .collect();

                serde_json::json!({
                    "role": m.role,
                    "content": content_parts
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(self.config.max_tokens),
            "temperature": request.temperature.unwrap_or(0.3),
        });

        let mut last_err = None;
        let mut resp = None;
        for _attempt in 0..2 {
            let mut req_builder = self.client.post(&url).json(&body);

            if let Some(ref key) = self.config.api_key {
                if !key.is_empty() {
                    req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
                }
            }

            match req_builder.send().await {
                Ok(ok) => {
                    resp = Some(ok);
                    break;
                }
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }
        let resp = resp.ok_or_else(|| {
            format!(
                "Vision request failed: {}",
                last_err
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "unknown transport error".to_string())
            )
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Vision API error ({}): {}", status, body));
        }

        let openai_resp: OpenAIResponse = resp
            .json()
            .await
            .map_err(|e| format!("Vision response parse failed: {}", e))?;

        let choice = openai_resp
            .choices
            .first()
            .ok_or("Vision API returned empty result")?;

        Ok(ChatResponse {
            content: choice.message.content.clone(),
            model: openai_resp.model,
            usage: openai_resp.usage.map(|u| TokenUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }

    async fn chat_completion_stream(
        &self,
        request: ChatRequest,
        sender: StreamSender,
    ) -> Result<ChatResponse, String> {
        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let enable_thinking = match request.thinking {
            Some(false) => Some(false),
            _ => None,
        };

        let openai_req = OpenAIRequest {
            model: self.config.model.clone(),
            messages: request
                .messages
                .iter()
                .map(|m| OpenAIMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            max_tokens: request.max_tokens.or(Some(self.config.max_tokens)),
            temperature: request.temperature,
            stream: Some(true),
            enable_thinking,
        };

        let mut req_builder = self
            .client
            .post(&url)
            .json(&openai_req)
            .timeout(Duration::from_secs(300));

        if let Some(ref key) = self.config.api_key {
            if !key.is_empty() {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
            }
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| format!("流式请求失败: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("API 错误 ({}): {}", status, body));
        }

        let mut full_content = String::new();
        let mut full_thinking = String::new();
        let mut model_name = self.config.model.clone();
        let mut buf = String::new();
        let mut finished = false;

        let mut stream = resp.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("流读取错误: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            buf.push_str(&text);

            while let Some(line_end) = buf.find('\n') {
                let line = buf[..line_end].trim().to_string();
                buf = buf[line_end + 1..].to_string();

                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }

                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(chunk_data) = serde_json::from_str::<StreamResponseChunk>(json_str) {
                        if let Some(ref m) = chunk_data.model {
                            model_name = m.clone();
                        }
                        for choice in &chunk_data.choices {
                            if let Some(ref thinking) = choice.delta.reasoning_content {
                                if !thinking.is_empty() {
                                    full_thinking.push_str(thinking);
                                    let _ = sender.send(StreamChunk::Thinking(thinking.clone()));
                                }
                            }
                            if let Some(ref content) = choice.delta.content {
                                if !content.is_empty() {
                                    full_content.push_str(content);
                                    let _ = sender.send(StreamChunk::Delta(content.clone()));
                                }
                            }
                        }
                        if !chunk_data.choices.is_empty()
                            && chunk_data
                                .choices
                                .iter()
                                .all(|choice| choice.finish_reason.is_some())
                        {
                            finished = true;
                        }
                    }
                }
            }
            if finished {
                break;
            }
        }

        Ok(ChatResponse {
            content: full_content,
            model: model_name,
            usage: None,
        })
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Remote(self.config.name.clone())
    }

    async fn health_check(&self) -> Result<bool, String> {
        let url = format!("{}/models", self.config.base_url.trim_end_matches('/'));
        let mut req_builder = self.client.get(&url);

        if let Some(ref key) = self.config.api_key {
            if !key.is_empty() {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
            }
        }

        match req_builder.send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => Err(format!("连接失败: {}", e)),
        }
    }
}

// --- OpenAI API 数据结构 ---

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    /// 控制深度思考（DeepSeek 等支持 reasoning 的模型），false = 关闭思考
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_thinking: Option<bool>,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIChoiceMessage,
}

#[derive(Deserialize)]
struct OpenAIChoiceMessage {
    content: String,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// --- SSE 流式响应数据结构 ---

#[derive(Deserialize)]
struct StreamResponseChunk {
    #[allow(dead_code)]
    model: Option<String>,
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
}
