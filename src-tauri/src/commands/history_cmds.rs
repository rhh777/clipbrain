use crate::storage::clipboard_history::{self, ClipboardHistoryItem};
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;

/// 查询历史记录（分页）
#[tauri::command]
pub fn list_history(
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ClipboardHistoryItem>, String> {
    let start = Instant::now();
    let result = clipboard_history::list_history(limit.unwrap_or(50), offset.unwrap_or(0));
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 40 {
        log::warn!(
            "[perf] list_history slow: {} ms, limit={}, offset={}",
            elapsed.as_millis(),
            limit.unwrap_or(50),
            offset.unwrap_or(0)
        );
    }
    result
}

/// 搜索历史记录
#[tauri::command]
pub fn search_history(
    keyword: String,
    content_type: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<ClipboardHistoryItem>, String> {
    let start = Instant::now();
    let result =
        clipboard_history::search_history(&keyword, content_type.as_deref(), limit.unwrap_or(50));
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 40 {
        log::warn!(
            "[perf] search_history slow: {} ms, keyword_len={}, content_type={:?}, limit={}",
            elapsed.as_millis(),
            keyword.len(),
            content_type,
            limit.unwrap_or(50)
        );
    }
    result
}

/// 删除历史记录
#[tauri::command]
pub fn delete_history(id: i64) -> Result<(), String> {
    clipboard_history::delete_history(id)
}

/// 切换收藏状态
#[tauri::command]
pub fn toggle_pin(id: i64) -> Result<bool, String> {
    clipboard_history::toggle_pin(id)
}

/// 清空未收藏的历史记录
#[tauri::command]
pub fn clear_history() -> Result<u64, String> {
    clipboard_history::clear_unpinned()
}

/// 清空未收藏的历史记录，保留最近 retain_days 天的数据
#[tauri::command]
pub fn clear_history_with_retention(retain_days: u32) -> Result<u64, String> {
    clipboard_history::clear_unpinned_with_retention(retain_days)
}

/// 获取历史记录总数
#[tauri::command]
pub fn history_count() -> Result<i64, String> {
    clipboard_history::count_history()
}

/// 统计未收藏且字节数 >= min_bytes 的文本记录数量和总字节数
#[tauri::command]
pub fn count_history_over_size(min_bytes: i64) -> Result<(i64, i64), String> {
    if min_bytes < 0 {
        return Err("min_bytes 必须 >= 0".to_string());
    }
    clipboard_history::count_unpinned_over_size(min_bytes)
}

/// 删除未收藏且字节数 >= min_bytes 的文本记录
#[tauri::command]
pub fn clear_history_over_size(min_bytes: i64) -> Result<u64, String> {
    if min_bytes <= 0 {
        return Err("min_bytes 必须 > 0".to_string());
    }
    clipboard_history::clear_unpinned_over_size(min_bytes)
}

/// 高级搜索：关键词 + 内容类型 + 标签 + 日期范围组合筛选
#[tauri::command]
pub fn search_history_advanced(
    keyword: Option<String>,
    content_type: Option<String>,
    tag: Option<String>,
    pinned_only: Option<bool>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ClipboardHistoryItem>, String> {
    let start = Instant::now();
    let result = clipboard_history::search_history_advanced(
        keyword.as_deref(),
        content_type.as_deref(),
        tag.as_deref(),
        pinned_only,
        date_from.as_deref(),
        date_to.as_deref(),
        limit.unwrap_or(50),
        offset.unwrap_or(0),
    );
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 40 {
        log::warn!(
            "[perf] search_history_advanced slow: {} ms, keyword_len={}, content_type={:?}, tag={:?}, pinned_only={:?}, has_date_from={}, has_date_to={}, limit={}, offset={}",
            elapsed.as_millis(),
            keyword.as_ref().map(|s| s.len()).unwrap_or(0),
            content_type,
            tag,
            pinned_only,
            date_from.is_some(),
            date_to.is_some(),
            limit.unwrap_or(50),
            offset.unwrap_or(0)
        );
    }
    result
}

/// 获取应用图标（macOS），返回 base64 data URL
#[tauri::command]
pub fn get_app_icon(app_name: String) -> Result<String, String> {
    let start = Instant::now();
    let cache_dir = get_icon_cache_dir()?;

    // 安全文件名
    let safe_name: String = app_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let png_path = cache_dir.join(format!("{}.png", safe_name));

    // 已缓存则直接读取并返回 base64
    if png_path.exists() {
        let result = png_to_data_url(&png_path);
        let elapsed = start.elapsed();
        if elapsed.as_millis() > 40 {
            log::warn!(
                "[perf] get_app_icon cache read slow: {} ms, app={}",
                elapsed.as_millis(),
                app_name
            );
        }
        return result;
    }

    #[cfg(target_os = "macos")]
    {
        // 通过常见路径或 mdfind 查找 app
        let app_path = find_app_path(&app_name)?;
        let icns_path = find_icns_path(&app_path)?;

        // 使用 sips 转换为 32x32 PNG
        let output = std::process::Command::new("sips")
            .args([
                "-s",
                "format",
                "png",
                "-z",
                "32",
                "32",
                &icns_path,
                "--out",
                png_path.to_str().unwrap_or_default(),
            ])
            .output()
            .map_err(|e| format!("sips 执行失败: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "sips 转换失败: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let result = png_to_data_url(&png_path);
        log::warn!(
            "[perf] get_app_icon cold path: {} ms, app={}",
            start.elapsed().as_millis(),
            app_name
        );
        result
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err("仅支持 macOS".to_string())
    }
}

#[derive(serde::Serialize)]
pub struct FilePreviewPayload {
    pub path: String,
    pub file_name: String,
    pub extension: Option<String>,
    pub kind: String,
    pub data_url: Option<String>,
    pub text: Option<String>,
    pub truncated: bool,
    pub is_dir: bool,
}

/// 获取文件预览：文本类返回内容片段，图片类返回缩略图，其余返回系统图标
#[tauri::command]
pub fn get_file_preview(path: String) -> Result<FilePreviewPayload, String> {
    let start = Instant::now();
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("文件不存在: {}", path));
    }

    let is_dir = path_buf.is_dir();
    let file_name = path_buf
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&path)
        .to_string();
    let extension = path_buf
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());

    if !is_dir {
        if let Some(mime) = image_mime_for_extension(extension.as_deref()) {
            let result = Ok(FilePreviewPayload {
                path,
                file_name,
                extension,
                kind: "image".to_string(),
                data_url: Some(file_to_data_url(&path_buf, mime)?),
                text: None,
                truncated: false,
                is_dir,
            });
            let elapsed = start.elapsed();
            if elapsed.as_millis() > 80 {
                log::warn!(
                    "[perf] get_file_preview image slow: {} ms, path={}",
                    elapsed.as_millis(),
                    path_buf.display()
                );
            }
            return result;
        }

        if is_text_previewable(extension.as_deref()) {
            let (text, truncated) = read_text_preview(&path_buf, 16 * 1024)?;
            let result = Ok(FilePreviewPayload {
                path,
                file_name,
                extension,
                kind: "text".to_string(),
                data_url: None,
                text: Some(text),
                truncated,
                is_dir,
            });
            let elapsed = start.elapsed();
            if elapsed.as_millis() > 40 {
                log::warn!(
                    "[perf] get_file_preview text slow: {} ms, path={}",
                    elapsed.as_millis(),
                    path_buf.display()
                );
            }
            return result;
        }
    }

    let result = Ok(FilePreviewPayload {
        path,
        file_name,
        extension,
        kind: "icon".to_string(),
        data_url: None,
        text: None,
        truncated: false,
        is_dir,
    });
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 20 {
        log::warn!(
            "[perf] get_file_preview fallback slow: {} ms, path={}",
            elapsed.as_millis(),
            path_buf.display()
        );
    }
    result
}

fn png_to_data_url(path: &PathBuf) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("读取图标失败: {}", e))?;
    let encoded = base64_encode(&data);
    Ok(format!("data:image/png;base64,{}", encoded))
}

fn file_to_data_url(path: &PathBuf, mime: &str) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("读取文件失败: {}", e))?;
    let encoded = base64_encode(&data);
    Ok(format!("data:{};base64,{}", mime, encoded))
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

fn get_icon_cache_dir() -> Result<PathBuf, String> {
    let cache = dirs::cache_dir().ok_or("无法获取缓存目录")?;
    let dir = cache.join("clipbrain").join("app_icons");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建缓存目录失败: {}", e))?;
    Ok(dir)
}

fn read_text_preview(path: &PathBuf, max_bytes: usize) -> Result<(String, bool), String> {
    let mut file = File::open(path).map_err(|e| format!("打开文件失败: {}", e))?;
    let metadata = file
        .metadata()
        .map_err(|e| format!("读取文件信息失败: {}", e))?;

    let mut buffer = vec![0u8; max_bytes];
    let read_len = file
        .read(&mut buffer)
        .map_err(|e| format!("读取文件失败: {}", e))?;
    buffer.truncate(read_len);

    let text = String::from_utf8_lossy(&buffer).replace('\0', " ");
    Ok((text, metadata.len() > max_bytes as u64))
}

fn is_text_previewable(extension: Option<&str>) -> bool {
    matches!(
        extension,
        Some(
            "txt"
                | "md"
                | "markdown"
                | "json"
                | "yaml"
                | "yml"
                | "toml"
                | "xml"
                | "csv"
                | "tsv"
                | "log"
                | "ini"
                | "conf"
                | "cfg"
                | "sh"
                | "zsh"
                | "bash"
                | "py"
                | "js"
                | "jsx"
                | "ts"
                | "tsx"
                | "c"
                | "h"
                | "cpp"
                | "hpp"
                | "java"
                | "go"
                | "rs"
                | "swift"
                | "css"
                | "scss"
                | "html"
                | "sql"
        )
    )
}

fn image_mime_for_extension(extension: Option<&str>) -> Option<&'static str> {
    match extension {
        Some("png") => Some("image/png"),
        Some("jpg") | Some("jpeg") => Some("image/jpeg"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        Some("bmp") => Some("image/bmp"),
        Some("svg") => Some("image/svg+xml"),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn find_app_path(app_name: &str) -> Result<String, String> {
    // 优先在 /Applications 和 /System/Applications 查找
    for base in &[
        "/Applications",
        "/System/Applications",
        "/System/Applications/Utilities",
    ] {
        let path = format!("{}/{}.app", base, app_name);
        if std::path::Path::new(&path).exists() {
            return Ok(path);
        }
    }

    // 回退到 mdfind
    let output = std::process::Command::new("mdfind")
        .arg(format!(
            "kMDItemKind == 'Application' && kMDItemDisplayName == '{}'",
            app_name
        ))
        .output()
        .map_err(|e| format!("mdfind 执行失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .next()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("未找到应用: {}", app_name))
}

#[cfg(target_os = "macos")]
fn find_icns_path(app_path: &str) -> Result<String, String> {
    let plist_path = format!("{}/Contents/Info.plist", app_path);

    // 使用 defaults read 获取图标文件名
    let output = std::process::Command::new("defaults")
        .args(["read", &plist_path, "CFBundleIconFile"])
        .output()
        .map_err(|e| format!("defaults read 失败: {}", e))?;

    if !output.status.success() {
        // 回退：查找 Resources 目录下的 .icns 文件
        let resources = format!("{}/Contents/Resources", app_path);
        if let Ok(entries) = std::fs::read_dir(&resources) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "icns" {
                        return entry
                            .path()
                            .to_str()
                            .map(|s| s.to_string())
                            .ok_or_else(|| "路径转换失败".to_string());
                    }
                }
            }
        }
        return Err("未找到应用图标".to_string());
    }

    let icon_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let icon_file = if icon_name.ends_with(".icns") {
        icon_name
    } else {
        format!("{}.icns", icon_name)
    };
    let full_path = format!("{}/Contents/Resources/{}", app_path, icon_file);

    if std::path::Path::new(&full_path).exists() {
        Ok(full_path)
    } else {
        Err(format!("图标文件不存在: {}", full_path))
    }
}
