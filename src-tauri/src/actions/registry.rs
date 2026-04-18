use super::traits::{Action, ActionDescriptor};
use crate::classifier::rules::ContentType;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionMatchKind {
    Specific,
    General,
}

/// 宽松匹配内容类型（Code 变体忽略语言参数）
fn content_type_match_kind(
    supported: &ContentType,
    actual: &ContentType,
) -> Option<ActionMatchKind> {
    match (supported, actual) {
        // 数学表达式本质上仍是文本，应保留通用文本操作。
        (ContentType::PlainText, ContentType::MathExpression) => Some(ActionMatchKind::General),
        (ContentType::Code(_), ContentType::Code(_)) => Some(ActionMatchKind::Specific),
        (ContentType::TableData(_), ContentType::TableData(_)) => Some(ActionMatchKind::Specific),
        _ if supported == actual => Some(ActionMatchKind::Specific),
        _ => None,
    }
}

fn descriptor_for_action(
    action: &dyn Action,
    content_type: &ContentType,
    locale: &str,
) -> Option<ActionDescriptor> {
    let match_kind = action
        .supported_types()
        .iter()
        .filter_map(|supported| content_type_match_kind(supported, content_type))
        .max_by_key(|kind| match kind {
            ActionMatchKind::General => 0,
            ActionMatchKind::Specific => 1,
        })?;

    let mut descriptor = action.to_descriptor(locale);
    descriptor.action_scope = if match_kind == ActionMatchKind::Specific {
        "specific".to_string()
    } else {
        "general".to_string()
    };
    Some(descriptor)
}

fn descriptor_sort_key(descriptor: &ActionDescriptor) -> (u8, bool, String) {
    (
        if descriptor.action_scope == "specific" {
            0
        } else {
            1
        },
        descriptor.requires_model,
        descriptor.display_name.to_lowercase(),
    )
}

/// 操作注册表
pub struct ActionRegistry {
    actions: Vec<Arc<dyn Action>>,
    /// 跟踪哪些 action ID 来自插件（用于热重载时区分内置与插件）
    plugin_action_ids: HashSet<String>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            plugin_action_ids: HashSet::new(),
        }
    }

    /// 注册一个操作
    pub fn register(&mut self, action: Arc<dyn Action>) {
        self.actions.push(action);
    }

    /// 注册一个插件操作（同时记录其 ID）
    pub fn register_plugin(&mut self, action: Arc<dyn Action>) {
        self.plugin_action_ids.insert(action.id().to_string());
        self.actions.push(action);
    }

    /// 获取所有操作的描述信息，根据 locale 返回对应语言
    pub fn list_descriptors(
        &self,
        content_type: &ContentType,
        locale: &str,
    ) -> Vec<ActionDescriptor> {
        let mut descriptors: Vec<_> = self
            .actions
            .iter()
            .filter_map(|a| descriptor_for_action(a.as_ref(), content_type, locale))
            .collect();
        descriptors.sort_by_key(descriptor_sort_key);
        descriptors
    }

    /// 获取所有操作的描述信息（不过滤类型）
    pub fn list_all_descriptors(&self, locale: &str) -> Vec<ActionDescriptor> {
        self.actions
            .iter()
            .map(|a| a.to_descriptor(locale))
            .collect()
    }

    /// 根据 ID 查找操作，返回 Arc 克隆
    pub fn get_action(&self, id: &str) -> Option<Arc<dyn Action>> {
        self.actions.iter().find(|a| a.id() == id).cloned()
    }

    /// 热重载插件：移除所有旧插件操作，注册新插件操作
    pub fn reload_plugins(&mut self, new_plugins: Vec<Arc<dyn Action>>) {
        // 移除所有旧插件操作
        self.actions
            .retain(|a| !self.plugin_action_ids.contains(a.id()));
        self.plugin_action_ids.clear();
        // 注册新插件
        for plugin in new_plugins {
            self.plugin_action_ids.insert(plugin.id().to_string());
            self.actions.push(plugin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{content_type_match_kind, descriptor_for_action, ActionRegistry};
    use crate::actions::builtin;
    use crate::classifier::rules::ContentType;

    #[test]
    fn plain_text_actions_apply_to_math_expression() {
        assert_eq!(
            content_type_match_kind(&ContentType::PlainText, &ContentType::MathExpression),
            Some(super::ActionMatchKind::General)
        );
    }

    #[test]
    fn plain_text_actions_do_not_apply_to_json() {
        assert_eq!(
            content_type_match_kind(&ContentType::PlainText, &ContentType::Json),
            None
        );
    }

    #[test]
    fn math_expression_contains_general_and_specific_actions() {
        let mut registry = ActionRegistry::new();
        builtin::register_builtin_actions(&mut registry);
        let descriptors = registry.list_descriptors(&ContentType::MathExpression, "zh-CN");

        assert!(descriptors
            .iter()
            .any(|d| d.id == "math_calculate" && d.action_scope == "specific"));
        assert!(descriptors
            .iter()
            .any(|d| d.id == "summarize" && d.action_scope == "general"));
    }

    #[test]
    fn exact_match_multi_type_action_is_marked_specific() {
        let action = builtin::llm_actions::TranslateToChineseAction;
        let descriptor =
            descriptor_for_action(&action, &ContentType::Code("rust".to_string()), "zh-CN")
                .expect("translate action should match code");
        assert_eq!(descriptor.action_scope, "specific");
    }

    #[test]
    fn inherited_match_action_is_marked_general() {
        let action = builtin::llm_actions::SummarizeAction;
        let descriptor = descriptor_for_action(&action, &ContentType::MathExpression, "zh-CN")
            .expect("summarize action should match math expression through plain text fallback");
        assert_eq!(descriptor.action_scope, "general");
    }

    #[test]
    fn specific_actions_are_sorted_before_general_actions() {
        let mut registry = ActionRegistry::new();
        builtin::register_builtin_actions(&mut registry);
        let descriptors = registry.list_descriptors(&ContentType::MathExpression, "zh-CN");

        let first_general = descriptors.iter().position(|d| d.action_scope == "general");
        let last_specific = descriptors
            .iter()
            .rposition(|d| d.action_scope == "specific");
        assert!(matches!((last_specific, first_general), (Some(s), Some(g)) if s < g));
    }
}
