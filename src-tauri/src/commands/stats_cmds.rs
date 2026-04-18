use super::action_cmds::list_all_action_descriptors;
use crate::storage::action_history::{get_action_stats, ActionStats};

/// 获取操作统计概览
#[tauri::command]
pub fn get_stats(locale: Option<String>) -> Result<ActionStats, String> {
    let locale = locale.as_deref().unwrap_or("zh-CN");
    let mut stats = get_action_stats()?;

    // 获取所有操作的 display_name 映射
    let all_descriptors = list_all_action_descriptors(locale);
    let name_map: std::collections::HashMap<&str, &str> = all_descriptors
        .iter()
        .map(|d| (d.id.as_str(), d.display_name.as_str()))
        .collect();

    // 特殊 action_id 的 display_name 映射（不在 registry 中注册的操作）
    let special_names: std::collections::HashMap<&str, &str> = if locale.starts_with("en") {
        [("custom_prompt", "Custom Action")].into()
    } else {
        [("custom_prompt", "自定义操作")].into()
    };

    for action in &mut stats.top_actions {
        if let Some(name) = name_map.get(action.action_id.as_str()) {
            action.display_name = name.to_string();
        } else if let Some(name) = special_names.get(action.action_id.as_str()) {
            action.display_name = name.to_string();
        }
    }

    Ok(stats)
}
