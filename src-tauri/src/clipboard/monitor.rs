use chrono::Local;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

use crate::actions::traits::ActionDescriptor;
use crate::classifier::rules::{classify_by_rules, ContentType};
use crate::commands::action_cmds::list_actions_for_type;
use crate::config::{manager as config_manager, privacy};
use crate::storage::clipboard_history;

/// 获取当前前台应用名称（macOS）
/// 优先返回 .app bundle 的显示名（如 "IntelliJ IDEA"），回退到进程名。
#[cfg(target_os = "macos")]
fn get_frontmost_app() -> Option<String> {
    // 先尝试获取 bundle 显示名（name of application file），更准确
    let script = r#"try
    tell application "System Events"
        set frontApp to first application process whose frontmost is true
        return name of application file of frontApp
    end tell
on error
    tell application "System Events"
        return name of first application process whose frontmost is true
    end tell
end try"#;
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn get_frontmost_app() -> Option<String> {
    // Windows: use powershell to get foreground window process name
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "(Get-Process | Where-Object { $_.MainWindowHandle -eq (Add-Type '[DllImport(\"user32.dll\")] public static extern IntPtr GetForegroundWindow();' -Name W -PassThru)::GetForegroundWindow() }).ProcessName"])
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    } else {
        None
    }
}

#[cfg(target_os = "linux")]
fn get_frontmost_app() -> Option<String> {
    // Linux: use xdotool to get active window name
    let output = std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowpid"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // Read process name from /proc
    std::fs::read_to_string(format!("/proc/{}/comm", pid))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn get_frontmost_app() -> Option<String> {
    None
}

fn get_frontmost_app_cached() -> Option<String> {
    static CACHE: OnceLock<Mutex<(Option<String>, Option<Instant>)>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new((None, None)));

    if let Ok(guard) = cache.lock() {
        if let Some(last_fetch) = guard.1 {
            if last_fetch.elapsed() < Duration::from_secs(5) {
                return guard.0.clone();
            }
        }
    }

    let fresh = get_frontmost_app();
    if let Ok(mut guard) = cache.lock() {
        *guard = (fresh.clone(), Some(Instant::now()));
    }
    fresh
}

/// 剪贴板变化事件 — 推送到前端
#[derive(Debug, Clone, Serialize)]
pub struct ClipboardChangeEvent {
    pub content: String,
    pub content_type: ContentType,
    pub preview: String,
    pub actions: Vec<ActionDescriptor>,
    pub timestamp: u64,
    pub item: Option<clipboard_history::ClipboardHistoryItem>,
}

/// 剪贴板监听器 — 轮询检测文本、图片和文件列表变化
pub struct ClipboardMonitor {
    last_content: Arc<Mutex<String>>,
    last_image_hash: Arc<Mutex<String>>,
    last_file_list_signature: Arc<Mutex<String>>,
    last_change: Arc<Mutex<Instant>>,
    debounce_ms: u64,
    poll_interval_ms: u64,
}

impl ClipboardMonitor {
    pub fn new(poll_interval_ms: u64, debounce_ms: u64) -> Self {
        Self {
            last_content: Arc::new(Mutex::new(String::new())),
            last_image_hash: Arc::new(Mutex::new(String::new())),
            last_file_list_signature: Arc::new(Mutex::new(String::new())),
            last_change: Arc::new(Mutex::new(Instant::now())),
            debounce_ms,
            poll_interval_ms,
        }
    }

    /// 读取当前剪贴板文本，如果与上次不同则返回 Some
    pub fn poll_text(&self) -> Option<String> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let text = clipboard.get_text().ok()?;

        if text.trim().is_empty() {
            return None;
        }

        let mut last = self.last_content.lock().ok()?;
        let mut last_time = self.last_change.lock().ok()?;

        if *last == text {
            return None;
        }

        // 节流：短时间内的重复变化不触发
        if last_time.elapsed() < Duration::from_millis(self.debounce_ms) {
            return None;
        }

        *last = text.clone();
        *last_time = Instant::now();
        Some(text)
    }

    /// 读取当前剪贴板图片，如果与上次不同则返回 Some((width, height, rgba_bytes, hash))
    pub fn poll_image(&self) -> Option<(u32, u32, Vec<u8>, String)> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let img = clipboard.get_image().ok()?;

        if img.width == 0 || img.height == 0 {
            return None;
        }

        // 计算哈希去重
        let mut hasher = Sha256::new();
        hasher.update(&img.bytes);
        let hash = format!("{:x}", hasher.finalize());

        let mut last_hash = self.last_image_hash.lock().ok()?;
        let mut last_time = self.last_change.lock().ok()?;

        if *last_hash == hash {
            return None;
        }

        if last_time.elapsed() < Duration::from_millis(self.debounce_ms) {
            return None;
        }

        *last_hash = hash;
        *last_time = Instant::now();
        Some((
            img.width as u32,
            img.height as u32,
            img.bytes.into_owned(),
            last_hash.clone(),
        ))
    }

    /// 读取当前剪贴板文件列表，如果与上次不同则返回 Some
    pub fn poll_file_list(&self) -> Option<Vec<String>> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let file_list = clipboard.get().file_list().ok()?;

        if file_list.is_empty() {
            return None;
        }

        let normalized = file_list
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let signature = normalized.join("\n");

        let mut last_signature = self.last_file_list_signature.lock().ok()?;
        let mut last_time = self.last_change.lock().ok()?;

        if *last_signature == signature {
            return None;
        }

        if last_time.elapsed() < Duration::from_millis(self.debounce_ms) {
            return None;
        }

        *last_signature = signature;
        *last_time = Instant::now();
        Some(normalized)
    }

    /// 当前剪贴板是否仍然携带文件列表
    pub fn has_file_list(&self) -> bool {
        arboard::Clipboard::new()
            .ok()
            .and_then(|mut clipboard| clipboard.get().file_list().ok())
            .is_some_and(|file_list| !file_list.is_empty())
    }

    /// 启动后台监听循环
    pub fn start(self, app_handle: AppHandle) {
        let interval = self.poll_interval_ms;

        std::thread::spawn(move || {
            println!(
                "[ClipBrain] 剪贴板监听已启动 (间隔 {}ms, 去重 {}ms)",
                interval, self.debounce_ms
            );

            // 先测试剪贴板是否可访问
            match arboard::Clipboard::new() {
                Ok(mut cb) => match cb.get_text() {
                    Ok(t) => println!("[ClipBrain] 剪贴板可访问，当前内容长度: {}", t.len()),
                    Err(e) => println!("[ClipBrain] 剪贴板读取失败: {}", e),
                },
                Err(e) => println!("[ClipBrain] 剪贴板初始化失败: {}", e),
            }

            loop {
                std::thread::sleep(Duration::from_millis(interval));

                // 隐私检查：排除应用
                if privacy::should_skip_clipboard() {
                    continue;
                }

                // 检测图片变化
                if let Some((width, height, rgba, image_hash)) = self.poll_image() {
                    println!("[ClipBrain] 检测到剪贴板图片: {}x{}", width, height);

                    // 保存为 PNG
                    match save_image_to_file(width, height, &rgba, &image_hash) {
                        Ok(image_path) => {
                            let source_app = get_frontmost_app_cached();
                            let content_type_str = format!("{:?}", ContentType::Image);

                            let item = match clipboard_history::insert_history(
                                None,
                                Some(&image_path),
                                Some(&image_hash),
                                &content_type_str,
                                source_app.as_deref(),
                                None,
                                false,
                            ) {
                                Ok(id) => {
                                    println!("[ClipBrain] 图片历史记录已写入, id={}", id);
                                    Some(build_history_item(
                                        id,
                                        None,
                                        Some(image_path.clone()),
                                        content_type_str.clone(),
                                        source_app.clone(),
                                        None,
                                        false,
                                    ))
                                }
                                Err(e) => {
                                    println!("[ClipBrain] 图片历史记录写入失败: {}", e);
                                    None
                                }
                            };

                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64;

                            let event = ClipboardChangeEvent {
                                content: image_path.clone(),
                                content_type: ContentType::Image,
                                preview: format!("[Image {}x{}]", width, height),
                                actions: list_actions_for_type(&ContentType::Image, "zh-CN"),
                                timestamp,
                                item,
                            };

                            let _ = app_handle.emit("clipboard-change", &event);
                        }
                        Err(e) => println!("[ClipBrain] 保存剪贴板图片失败: {}", e),
                    }
                    continue;
                }

                if let Some(paths) = self.poll_file_list() {
                    println!("[ClipBrain] 检测到剪贴板文件: {} 个", paths.len());

                    let content = paths.join("\n");
                    let content_type = ContentType::FileList;
                    let content_type_str = format!("{:?}", content_type);
                    let source_app = get_frontmost_app_cached();

                    let item = match clipboard_history::insert_history(
                        Some(&content),
                        None,
                        None,
                        &content_type_str,
                        source_app.as_deref(),
                        None,
                        false,
                    ) {
                        Ok(id) => {
                            println!("[ClipBrain] 文件历史记录已写入, id={}", id);
                            Some(build_history_item(
                                id,
                                Some(content.clone()),
                                None,
                                content_type_str.clone(),
                                source_app.clone(),
                                None,
                                false,
                            ))
                        }
                        Err(e) => {
                            println!("[ClipBrain] 文件历史记录写入失败: {}", e);
                            None
                        }
                    };

                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let event = ClipboardChangeEvent {
                        content: content.clone(),
                        content_type,
                        preview: make_file_list_preview(&paths),
                        actions: list_actions_for_type(&ContentType::FileList, "zh-CN"),
                        timestamp,
                        item,
                    };

                    match app_handle.emit("clipboard-change", &event) {
                        Ok(_) => println!("[ClipBrain] 文件事件已推送到前端"),
                        Err(e) => println!("[ClipBrain] 推送文件事件失败: {}", e),
                    }

                    let config = config_manager::get();
                    let mode = config.general.trigger_mode.as_str();
                    if mode == "auto_popup" || mode == "both" {
                        crate::show_main_window(&app_handle, crate::MainWindowShowMode::Overlay);
                    }
                    continue;
                }

                // Finder 等应用会同时放入文件列表和文件名文本；有文件列表时优先只记录文件项。
                if self.has_file_list() {
                    continue;
                }

                if let Some(text) = self.poll_text() {
                    println!("[ClipBrain] 检测到剪贴板变化: {} 字符", text.len());

                    let content_type = classify_by_rules(&text);
                    println!("[ClipBrain] 分类结果: {:?}", content_type);

                    let actions = list_actions_for_type(&content_type, "zh-CN");
                    println!("[ClipBrain] 可用操作: {} 个", actions.len());

                    let preview = make_preview(&text, 200);

                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    // 写入历史记录
                    let content_type_str = format!("{:?}", content_type);
                    let is_sensitive = matches!(
                        content_type,
                        ContentType::PhoneNumber | ContentType::IdCard | ContentType::Email
                    );
                    let char_count = text.len() as i64;
                    let source_app = get_frontmost_app_cached();
                    if let Some(ref app) = source_app {
                        println!("[ClipBrain] 来源应用: {}", app);
                    }
                    let item = match clipboard_history::insert_history(
                        Some(&text),
                        None,
                        None,
                        &content_type_str,
                        source_app.as_deref(),
                        Some(char_count),
                        is_sensitive,
                    ) {
                        Ok(id) => {
                            println!("[ClipBrain] 历史记录已写入, id={}", id);
                            Some(build_history_item(
                                id,
                                Some(text.clone()),
                                None,
                                content_type_str.clone(),
                                source_app.clone(),
                                Some(char_count),
                                is_sensitive,
                            ))
                        }
                        Err(e) => {
                            println!("[ClipBrain] 历史记录写入失败: {}", e);
                            None
                        }
                    };

                    let event = ClipboardChangeEvent {
                        content: text,
                        content_type,
                        preview,
                        actions,
                        timestamp,
                        item,
                    };

                    match app_handle.emit("clipboard-change", &event) {
                        Ok(_) => println!("[ClipBrain] 事件已推送到前端"),
                        Err(e) => println!("[ClipBrain] 推送事件失败: {}", e),
                    }

                    // 根据 trigger_mode 自动显示窗口
                    let config = config_manager::get();
                    let mode = config.general.trigger_mode.as_str();
                    if mode == "auto_popup" || mode == "both" {
                        crate::show_main_window(&app_handle, crate::MainWindowShowMode::Overlay);
                    }
                }
            }
        });
    }
}

/// 将 RGBA 图片数据保存为 PNG 文件，返回文件路径
fn save_image_to_file(
    width: u32,
    height: u32,
    rgba: &[u8],
    image_hash: &str,
) -> Result<String, String> {
    let dir = dirs::home_dir()
        .ok_or("Cannot get HOME dir")?
        .join(".clipbrain")
        .join("images");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Create images dir failed: {}", e))?;

    let filename = format!("clip_{}.png", image_hash);
    let path = dir.join(&filename);

    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or("Invalid RGBA image data")?;
    img.save(&path)
        .map_err(|e| format!("Save PNG failed: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

/// 生成内容预览（截取前 max_len 字符）
fn make_preview(text: &str, max_len: usize) -> String {
    let trimmed = text.trim();
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        let mut end = max_len;
        while !trimmed.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}…", &trimmed[..end])
    }
}

fn make_file_list_preview(paths: &[String]) -> String {
    const MAX_PREVIEW_ITEMS: usize = 3;

    let mut names = paths
        .iter()
        .take(MAX_PREVIEW_ITEMS)
        .map(|path| {
            std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .unwrap_or(path)
                .to_string()
        })
        .collect::<Vec<_>>();

    if paths.len() > MAX_PREVIEW_ITEMS {
        names.push("…".to_string());
    }

    names.join("\n")
}

fn build_history_item(
    id: i64,
    content: Option<String>,
    image_path: Option<String>,
    content_type: String,
    source_app: Option<String>,
    char_count: Option<i64>,
    is_sensitive: bool,
) -> clipboard_history::ClipboardHistoryItem {
    clipboard_history::ClipboardHistoryItem {
        id,
        content,
        image_path,
        content_type,
        source_app,
        char_count,
        created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        is_pinned: false,
        is_sensitive,
    }
}
