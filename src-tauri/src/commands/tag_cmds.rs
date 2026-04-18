use crate::storage::clipboard_tags;
use std::time::Instant;

/// 为剪贴板条目添加标签
#[tauri::command]
pub fn add_tag(clipboard_id: i64, tag_name: String) -> Result<(), String> {
    clipboard_tags::add_tag(clipboard_id, &tag_name)
}

/// 删除剪贴板条目的指定标签
#[tauri::command]
pub fn remove_tag(clipboard_id: i64, tag_name: String) -> Result<(), String> {
    clipboard_tags::remove_tag(clipboard_id, &tag_name)
}

/// 获取剪贴板条目的所有标签
#[tauri::command]
pub fn get_tags(clipboard_id: i64) -> Result<Vec<String>, String> {
    let start = Instant::now();
    let result = clipboard_tags::get_tags_for_item(clipboard_id);
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 20 {
        log::warn!(
            "[perf] get_tags slow: {} ms, clipboard_id={}",
            elapsed.as_millis(),
            clipboard_id
        );
    }
    result
}

/// 获取所有已使用的标签（去重）
#[tauri::command]
pub fn list_all_tags() -> Result<Vec<String>, String> {
    let start = Instant::now();
    let result = clipboard_tags::list_all_tags();
    let elapsed = start.elapsed();
    if elapsed.as_millis() > 20 {
        log::warn!("[perf] list_all_tags slow: {} ms", elapsed.as_millis());
    }
    result
}

/// 根据标签搜索历史记录
#[tauri::command]
pub fn search_by_tag(tag_name: String) -> Result<Vec<i64>, String> {
    clipboard_tags::find_clipboard_ids_by_tag(&tag_name)
}
