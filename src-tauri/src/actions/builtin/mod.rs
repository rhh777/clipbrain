pub mod format_json;
pub mod format_yaml;
pub mod llm_actions;
pub mod llm_actions_v2;
pub mod mask_sensitive;
pub mod math_calc;
pub mod table_convert;
pub mod url_preview;
pub mod vision_actions;

use crate::actions::registry::ActionRegistry;
use std::sync::Arc;

/// 注册所有内置操作
pub fn register_builtin_actions(registry: &mut ActionRegistry) {
    // JSON
    registry.register(Arc::new(format_json::JsonFormatAction));
    registry.register(Arc::new(format_json::JsonToYamlAction));
    // YAML
    registry.register(Arc::new(format_yaml::YamlFormatAction));
    registry.register(Arc::new(format_yaml::YamlToJsonAction));
    // 文本/敏感信息
    registry.register(Arc::new(mask_sensitive::MaskSensitiveAction));
    // 数学
    registry.register(Arc::new(math_calc::MathCalculateAction));
    // URL
    registry.register(Arc::new(url_preview::UrlToMarkdownAction));
    // 表格转换
    registry.register(Arc::new(table_convert::TableToMarkdownAction));
    registry.register(Arc::new(table_convert::TableToJsonAction));
    // LLM 操作（需要远程后端）
    registry.register(Arc::new(llm_actions::TranslateToChineseAction));
    registry.register(Arc::new(llm_actions::TranslateToEnglishAction));
    registry.register(Arc::new(llm_actions::SummarizeAction));
    registry.register(Arc::new(llm_actions::CodeExplainAction));
    // Phase 2 新增 LLM 操作
    registry.register(Arc::new(llm_actions_v2::PolishTextAction));
    registry.register(Arc::new(llm_actions_v2::FixGrammarAction));
    registry.register(Arc::new(llm_actions_v2::ExtractContactsAction));
    registry.register(Arc::new(llm_actions_v2::GenerateRegexAction));
    // 视觉操作（图片）
    registry.register(Arc::new(vision_actions::ImageOcrAction));
    registry.register(Arc::new(vision_actions::ImageDescribeAction));
    registry.register(Arc::new(vision_actions::ImageTableExtractAction));
}
