use crate::actions::builtin::register_builtin_actions;
use crate::actions::plugin::loader;
use crate::actions::registry::ActionRegistry;
use crate::actions::traits::{ActionDescriptor, ActionInput, ActionOutput};
use crate::classifier::rules::{classify_by_rules, ContentType};
use crate::model::backend::{ChatMessage, StreamChunk};
use crate::model::state as model_state;
use crate::storage::action_history::insert_action_history;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;
use tauri::Emitter;

static REGISTRY: LazyLock<Mutex<ActionRegistry>> = LazyLock::new(|| {
    let mut registry = ActionRegistry::new();
    register_builtin_actions(&mut registry);
    // 加载插件操作
    for plugin_action in loader::load_all_plugins() {
        registry.register_plugin(plugin_action);
    }
    Mutex::new(registry)
});

/// 重新加载插件到全局 REGISTRY（供 plugin_cmds 调用）
pub fn reload_registry_plugins() -> usize {
    let new_plugins = loader::load_all_plugins();
    let count = new_plugins.len();
    let mut registry = REGISTRY.lock().unwrap();
    registry.reload_plugins(new_plugins);
    count
}

/// 内部函数：获取指定内容类型的可用操作列表（供 monitor 等模块调用）
pub fn list_actions_for_type(content_type: &ContentType, locale: &str) -> Vec<ActionDescriptor> {
    let registry = REGISTRY.lock().unwrap();
    registry.list_descriptors(content_type, locale)
}

/// 获取所有操作描述（不过滤类型，供统计等模块使用）
pub fn list_all_action_descriptors(locale: &str) -> Vec<ActionDescriptor> {
    let registry = REGISTRY.lock().unwrap();
    registry.list_all_descriptors(locale)
}

/// 获取指定内容类型的可用操作列表
#[tauri::command]
pub fn list_actions(content_type: ContentType, locale: Option<String>) -> Vec<ActionDescriptor> {
    let start = Instant::now();
    let actions = list_actions_for_type(&content_type, locale.as_deref().unwrap_or("zh-CN"));
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 20 {
        log::warn!(
            "[perf] list_actions slow: {} ms, content_type={:?}, locale={}",
            elapsed.as_millis(),
            content_type,
            locale.as_deref().unwrap_or("zh-CN")
        );
    }
    actions
}

/// 执行指定操作
#[tauri::command]
pub async fn execute_action(
    action_id: String,
    content: String,
    content_type: ContentType,
    thinking: Option<bool>,
) -> Result<ActionOutput, String> {
    let input_text = content.clone();
    let input = ActionInput {
        content,
        content_type,
        thinking,
    };

    // 先获取 Arc 克隆，然后释放锁，再 await
    let action = {
        let registry = REGISTRY.lock().unwrap();
        registry
            .get_action(&action_id)
            .ok_or_else(|| format!("操作 '{}' 未找到", action_id))?
    };

    let start = Instant::now();
    let output = action.execute(input).await?;
    let duration_ms = start.elapsed().as_millis() as i64;

    let model = model_state::default_backend_name();
    let _ = insert_action_history(
        None,
        &action_id,
        Some(&input_text),
        Some(&output.result),
        Some(duration_ms),
        if model.is_empty() { None } else { Some(&model) },
    );

    Ok(output)
}

/// 流式操作事件 payload
#[derive(serde::Serialize, Clone)]
pub struct ActionStreamPayload {
    pub event_type: String, // "start" | "thinking" | "delta" | "done" | "error"
    pub action_id: String,
    pub content: String,
    pub result_type: String,
}

/// 流式执行指定操作（通过事件推送增量结果）
#[tauri::command]
pub async fn execute_action_stream(
    app_handle: tauri::AppHandle,
    action_id: String,
    content: String,
    content_type: ContentType,
    thinking: Option<bool>,
) -> Result<ActionOutput, String> {
    let input = ActionInput {
        content,
        content_type,
        thinking,
    };

    let action = {
        let registry = REGISTRY.lock().unwrap();
        registry
            .get_action(&action_id)
            .ok_or_else(|| format!("操作 '{}' 未找到", action_id))?
    };

    // 发送 start 事件
    let _ = app_handle.emit(
        "action-stream",
        ActionStreamPayload {
            event_type: "start".to_string(),
            action_id: action_id.clone(),
            content: String::new(),
            result_type: String::new(),
        },
    );

    // 创建 channel 用于接收流式块
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();

    // 启动事件转发任务
    let handle = app_handle.clone();
    let aid = action_id.clone();
    let forward_task = tokio::spawn(async move {
        while let Some(chunk) = receiver.recv().await {
            match chunk {
                StreamChunk::Thinking(text) => {
                    let _ = handle.emit(
                        "action-stream",
                        ActionStreamPayload {
                            event_type: "thinking".to_string(),
                            action_id: aid.clone(),
                            content: text,
                            result_type: String::new(),
                        },
                    );
                }
                StreamChunk::Delta(text) => {
                    let _ = handle.emit(
                        "action-stream",
                        ActionStreamPayload {
                            event_type: "delta".to_string(),
                            action_id: aid.clone(),
                            content: text,
                            result_type: String::new(),
                        },
                    );
                }
            }
        }
    });

    let input_text = input.content.clone();

    // 执行流式操作
    let start = Instant::now();
    let result = action.execute_stream(input, sender).await;
    let duration_ms = start.elapsed().as_millis() as i64;

    // 等待转发任务完成
    let _ = forward_task.await;

    match result {
        Ok(output) => {
            let model = model_state::default_backend_name();
            let _ = insert_action_history(
                None,
                &action_id,
                Some(&input_text),
                Some(&output.result),
                Some(duration_ms),
                if model.is_empty() { None } else { Some(&model) },
            );
            let _ = app_handle.emit(
                "action-stream",
                ActionStreamPayload {
                    event_type: "done".to_string(),
                    action_id,
                    content: String::new(),
                    result_type: output.result_type.clone(),
                },
            );
            Ok(output)
        }
        Err(e) => {
            let _ = app_handle.emit(
                "action-stream",
                ActionStreamPayload {
                    event_type: "error".to_string(),
                    action_id,
                    content: e.clone(),
                    result_type: String::new(),
                },
            );
            Err(e)
        }
    }
}

/// 快捷操作结果
#[derive(serde::Serialize, Clone)]
pub struct QuickActionResult {
    pub success: bool,
    pub action_id: String,
    pub message: String,
}

/// 快捷操作：读取剪贴板 → 分类 → 执行指定操作 → 写回剪贴板
#[tauri::command]
pub async fn execute_quick_action(action_id: String) -> Result<QuickActionResult, String> {
    // 1. 读取剪贴板
    let text = {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("Cannot access clipboard: {}", e))?;
        clipboard
            .get_text()
            .map_err(|_| "No text in clipboard".to_string())?
    };

    if text.trim().is_empty() {
        return Err("Clipboard is empty".to_string());
    }

    // 2. 分类
    let content_type = classify_by_rules(&text);

    // 3. 查找并执行操作
    let action = {
        let registry = REGISTRY.lock().unwrap();
        registry
            .get_action(&action_id)
            .ok_or_else(|| format!("Action '{}' not found", action_id))?
    };

    let input_text = text.clone();
    let input = ActionInput {
        content: text,
        content_type,
        thinking: None,
    };

    let start = Instant::now();
    let output = action.execute(input).await?;
    let duration_ms = start.elapsed().as_millis() as i64;

    let model = model_state::default_backend_name();
    let _ = insert_action_history(
        None,
        &action_id,
        Some(&input_text),
        Some(&output.result),
        Some(duration_ms),
        if model.is_empty() { None } else { Some(&model) },
    );

    // 4. 写回剪贴板
    {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("Cannot access clipboard: {}", e))?;
        clipboard
            .set_text(&output.result)
            .map_err(|e| format!("Failed to write clipboard: {}", e))?;
    }

    Ok(QuickActionResult {
        success: true,
        action_id,
        message: output.result,
    })
}

/// 流式执行自定义操作（用户输入自定义 prompt）
#[tauri::command]
pub async fn execute_custom_stream(
    app_handle: tauri::AppHandle,
    content: String,
    prompt: String,
    thinking: Option<bool>,
) -> Result<ActionOutput, String> {
    let action_id = "custom_prompt".to_string();

    // 发送 start 事件
    let _ = app_handle.emit(
        "action-stream",
        ActionStreamPayload {
            event_type: "start".to_string(),
            action_id: action_id.clone(),
            content: String::new(),
            result_type: String::new(),
        },
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content,
        },
    ];

    // 创建 channel 用于接收流式块
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();

    // 启动事件转发任务
    let handle = app_handle.clone();
    let aid = action_id.clone();
    let forward_task = tokio::spawn(async move {
        while let Some(chunk) = receiver.recv().await {
            match chunk {
                StreamChunk::Thinking(text) => {
                    let _ = handle.emit(
                        "action-stream",
                        ActionStreamPayload {
                            event_type: "thinking".to_string(),
                            action_id: aid.clone(),
                            content: text,
                            result_type: String::new(),
                        },
                    );
                }
                StreamChunk::Delta(text) => {
                    let _ = handle.emit(
                        "action-stream",
                        ActionStreamPayload {
                            event_type: "delta".to_string(),
                            action_id: aid.clone(),
                            content: text,
                            result_type: String::new(),
                        },
                    );
                }
            }
        }
    });

    let input_text = messages
        .last()
        .map(|m| m.content.clone())
        .unwrap_or_default();

    let start = Instant::now();
    let result =
        model_state::chat_stream_with_thinking(messages, None, Some(0.5), thinking, sender).await;
    let duration_ms = start.elapsed().as_millis() as i64;

    let _ = forward_task.await;

    match result {
        Ok(resp) => {
            let output = ActionOutput {
                result: resp.content,
                result_type: "text".to_string(),
            };
            let model = model_state::default_backend_name();
            let _ = insert_action_history(
                None,
                &action_id,
                Some(&input_text),
                Some(&output.result),
                Some(duration_ms),
                if model.is_empty() { None } else { Some(&model) },
            );
            let _ = app_handle.emit(
                "action-stream",
                ActionStreamPayload {
                    event_type: "done".to_string(),
                    action_id,
                    content: String::new(),
                    result_type: output.result_type.clone(),
                },
            );
            Ok(output)
        }
        Err(e) => {
            let _ = app_handle.emit(
                "action-stream",
                ActionStreamPayload {
                    event_type: "error".to_string(),
                    action_id,
                    content: e.clone(),
                    result_type: String::new(),
                },
            );
            Err(e)
        }
    }
}
