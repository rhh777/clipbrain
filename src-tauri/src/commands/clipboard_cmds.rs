use crate::classifier::rules::{classify_by_rules, ContentType};
use std::borrow::Cow;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// 获取当前剪贴板内容并分类
#[tauri::command]
pub fn get_clipboard_content() -> Result<ClipboardResult, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {}", e))?;

    if let Ok(paths) = clipboard.get().file_list() {
        if !paths.is_empty() {
            return Ok(ClipboardResult {
                content: stringify_file_list(&paths),
                content_type: ContentType::FileList,
            });
        }
    }

    let text = clipboard
        .get_text()
        .map_err(|_| "剪贴板中无文本内容".to_string())?;

    if text.trim().is_empty() {
        return Err("剪贴板为空".to_string());
    }

    let content_type = classify_by_rules(&text);

    Ok(ClipboardResult {
        content: text,
        content_type,
    })
}

/// 将文本写入剪贴板
#[tauri::command]
pub fn write_to_clipboard(text: String) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {}", e))?;

    clipboard
        .set_text(text)
        .map_err(|e| format!("写入剪贴板失败: {}", e))?;

    Ok(())
}

/// 将图片文件写入剪贴板
#[tauri::command]
pub fn write_image_to_clipboard(path: String) -> Result<(), String> {
    let img = image::ImageReader::open(&path)
        .map_err(|e| format!("无法打开图片: {}", e))?
        .with_guessed_format()
        .map_err(|e| format!("无法识别图片格式: {}", e))?
        .decode()
        .map_err(|e| format!("无法解码图片: {}", e))?
        .into_rgba8();

    let (width, height) = img.dimensions();
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {}", e))?;

    clipboard
        .set_image(arboard::ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(img.into_raw()),
        })
        .map_err(|e| format!("写入图片到剪贴板失败: {}", e))?;

    Ok(())
}

/// 将文件列表写入剪贴板
#[tauri::command]
pub fn write_files_to_clipboard(paths: Vec<String>) -> Result<(), String> {
    let file_paths: Vec<PathBuf> = paths
        .into_iter()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect();

    if file_paths.is_empty() {
        return Err("没有可写入剪贴板的文件".to_string());
    }

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {}", e))?;

    clipboard
        .set()
        .file_list(&file_paths)
        .map_err(|e| format!("写入文件到剪贴板失败: {}", e))?;

    Ok(())
}

/// 触发系统粘贴当前剪贴板内容
#[tauri::command]
pub async fn paste_clipboard() -> Result<(), String> {
    tokio::time::sleep(Duration::from_millis(120)).await;
    paste_clipboard_impl()
}

/// 恢复到唤起面板前的应用并触发系统粘贴
#[tauri::command]
pub async fn restore_previous_app_and_paste() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        ensure_accessibility_permission()?;
        let reactivated = crate::reactivate_previous_overlay_app()?;
        tokio::time::sleep(Duration::from_millis(if reactivated { 120 } else { 60 })).await;
        return post_paste_shortcut();
    }

    #[cfg(not(target_os = "macos"))]
    {
        paste_clipboard().await
    }
}

#[cfg(target_os = "macos")]
fn paste_clipboard_impl() -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEYCODE_V: u16 = 9;

    ensure_accessibility_permission()?;
    post_paste_shortcut_with_keycode(KEYCODE_V)
}

#[cfg(target_os = "macos")]
fn ensure_accessibility_permission() -> Result<(), String> {
    use std::sync::atomic::{AtomicBool, Ordering};

    use macos_accessibility_client::accessibility::{
        application_is_trusted, application_is_trusted_with_prompt,
    };

    static ACCESSIBILITY_PROMPTED: AtomicBool = AtomicBool::new(false);

    if application_is_trusted() {
        return Ok(());
    }

    if !ACCESSIBILITY_PROMPTED.swap(true, Ordering::SeqCst)
        && application_is_trusted_with_prompt()
    {
        return Ok(());
    }

    Err("需要授予 ClipBrain“辅助功能”权限后才能自动粘贴，请在系统设置 > 隐私与安全性 > 辅助功能中开启后重试".to_string())
}

#[cfg(target_os = "macos")]
fn post_paste_shortcut() -> Result<(), String> {
    const KEYCODE_V: u16 = 9;
    post_paste_shortcut_with_keycode(KEYCODE_V)
}

#[cfg(target_os = "macos")]
fn post_paste_shortcut_with_keycode(keycode: u16) -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "创建系统输入事件失败".to_string())?;

    let key_down = CGEvent::new_keyboard_event(source.clone(), keycode, true)
        .map_err(|_| "创建粘贴按键事件失败".to_string())?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(CGEventTapLocation::HID);

    std::thread::sleep(Duration::from_millis(12));

    let key_up = CGEvent::new_keyboard_event(source, keycode, false)
        .map_err(|_| "创建粘贴按键事件失败".to_string())?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.post(CGEventTapLocation::HID);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn paste_clipboard_impl() -> Result<(), String> {
    Err("当前平台暂不支持自动粘贴".to_string())
}

#[derive(serde::Serialize)]
pub struct ClipboardResult {
    pub content: String,
    pub content_type: ContentType,
}

/// 读取图片文件并返回 base64 data URL
#[tauri::command]
pub fn read_image_base64(path: String) -> Result<String, String> {
    let start = Instant::now();
    let data = std::fs::read(&path).map_err(|e| format!("Failed to read image: {}", e))?;
    let b64 = base64_encode(&data);
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 80 {
        log::warn!(
            "[perf] read_image_base64 slow: {} ms, bytes={}, path={}",
            elapsed.as_millis(),
            data.len(),
            path
        );
    }
    Ok(format!("data:image/png;base64,{}", b64))
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

fn stringify_file_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n")
}
