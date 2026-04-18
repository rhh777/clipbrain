use super::backend::{
    ChatMessage, ChatRequest, ChatResponse, StreamSender, VisionMessage, VisionRequest,
};
use super::remote::openai_compat::{OpenAICompatClient, RemoteConfig};
use super::router::InferenceRouter;
use std::sync::{Arc, LazyLock, Mutex};

/// 全局推理路由器
static INFERENCE_ROUTER: LazyLock<Mutex<InferenceRouter>> =
    LazyLock::new(|| Mutex::new(InferenceRouter::new("rules".to_string())));

/// 配置远程后端（添加或更新）
pub fn configure_remote_backend(config: RemoteConfig) -> Result<(), String> {
    let client = OpenAICompatClient::new(config.clone())?;
    let name = config.name.clone();

    let mut router = INFERENCE_ROUTER.lock().map_err(|e| e.to_string())?;
    router.register_backend(name.clone(), Arc::new(client));

    // 如果还没有设置远程后端为默认，自动切换
    if router.default_backend_name() == "rules" {
        router.set_default_backend(name);
    }

    Ok(())
}

/// 移除远程后端
pub fn remove_remote_backend(name: &str) -> Result<(), String> {
    let mut router = INFERENCE_ROUTER.lock().map_err(|e| e.to_string())?;
    router.remove_backend(name);
    // 如果移除的是默认后端，回退到 rules
    if router.default_backend_name() == name {
        router.set_default_backend("rules".to_string());
    }
    Ok(())
}

/// 列出所有已注册后端
pub fn list_backends() -> Vec<String> {
    INFERENCE_ROUTER
        .lock()
        .map(|r| r.list_backends())
        .unwrap_or_default()
}

/// 使用默认后端执行聊天推理
pub async fn chat(
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<ChatResponse, String> {
    chat_with_thinking(messages, max_tokens, temperature, None).await
}

/// 使用默认后端执行聊天推理（支持思考模式控制）
pub async fn chat_with_thinking(
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    thinking: Option<bool>,
) -> Result<ChatResponse, String> {
    let backend = {
        let router = INFERENCE_ROUTER.lock().map_err(|e| e.to_string())?;
        router
            .get_default_backend()
            .ok_or_else(|| "未配置推理后端".to_string())?
    };

    let request = ChatRequest {
        messages,
        max_tokens,
        temperature,
        thinking,
    };
    backend.chat_completion(request).await
}

/// 测试指定后端的连接
pub async fn test_connection(backend_name: &str) -> Result<bool, String> {
    let backend = {
        let router = INFERENCE_ROUTER.lock().map_err(|e| e.to_string())?;
        router
            .get_backend(backend_name)
            .ok_or_else(|| format!("后端 '{}' 未找到", backend_name))?
    };

    backend.health_check().await
}

/// 使用默认后端执行视觉推理
pub async fn vision_chat(
    messages: Vec<VisionMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
) -> Result<ChatResponse, String> {
    let backend = {
        let router = INFERENCE_ROUTER.lock().map_err(|e| e.to_string())?;
        router
            .get_default_backend()
            .ok_or_else(|| "未配置推理后端".to_string())?
    };

    let request = VisionRequest {
        messages,
        max_tokens,
        temperature,
    };
    backend.vision_completion(request).await
}

/// 使用默认后端执行流式聊天推理
pub async fn chat_stream(
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    sender: StreamSender,
) -> Result<ChatResponse, String> {
    chat_stream_with_thinking(messages, max_tokens, temperature, None, sender).await
}

/// 使用默认后端执行流式聊天推理（支持思考模式控制）
pub async fn chat_stream_with_thinking(
    messages: Vec<ChatMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    thinking: Option<bool>,
    sender: StreamSender,
) -> Result<ChatResponse, String> {
    let backend = {
        let router = INFERENCE_ROUTER.lock().map_err(|e| e.to_string())?;
        router
            .get_default_backend()
            .ok_or_else(|| "未配置推理后端".to_string())?
    };

    let request = ChatRequest {
        messages,
        max_tokens,
        temperature,
        thinking,
    };
    backend.chat_completion_stream(request, sender).await
}

/// 设置默认后端
pub fn set_default_backend(name: &str) -> Result<(), String> {
    let mut router = INFERENCE_ROUTER.lock().map_err(|e| e.to_string())?;
    router.set_default_backend(name.to_string());
    Ok(())
}

/// 获取当前默认后端名称
pub fn default_backend_name() -> String {
    INFERENCE_ROUTER
        .lock()
        .map(|r| r.default_backend_name().to_string())
        .unwrap_or_default()
}

/// 检查是否有可用的远程后端
pub fn has_remote_backend() -> bool {
    INFERENCE_ROUTER
        .lock()
        .map(|r| r.has_remote_backend())
        .unwrap_or(false)
}
