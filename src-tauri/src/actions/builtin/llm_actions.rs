use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use crate::model::backend::{ChatMessage, StreamSender};
use crate::model::state;
use async_trait::async_trait;
use std::time::Duration;

// ===== 翻译为中文 =====

pub struct TranslateToChineseAction;

#[async_trait]
impl Action for TranslateToChineseAction {
    fn id(&self) -> &str {
        "translate_to_chinese"
    }
    fn display_name(&self) -> &str {
        "翻译为中文"
    }
    fn display_name_en(&self) -> &str {
        "Translate to Chinese"
    }
    fn description(&self) -> &str {
        "使用 AI 将文本翻译为中文"
    }
    fn description_en(&self) -> &str {
        "Translate text to Chinese using AI"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::PlainText, ContentType::Code("".to_string())]
    }

    fn requires_model(&self) -> bool {
        true
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是一个专业翻译。请将用户提供的文本翻译为中文，保持原文格式。只输出翻译结果，不要添加解释。".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];

        let resp = state::chat_with_thinking(messages, None, Some(0.3), thinking).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "text".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(5)
    }

    async fn execute_stream(
        &self,
        input: ActionInput,
        sender: StreamSender,
    ) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是一个专业翻译。请将用户提供的文本翻译为中文，保持原文格式。只输出翻译结果，不要添加解释。".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];
        let resp =
            state::chat_stream_with_thinking(messages, None, Some(0.3), thinking, sender).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "text".to_string(),
        })
    }
}

// ===== 翻译为英文 =====

pub struct TranslateToEnglishAction;

#[async_trait]
impl Action for TranslateToEnglishAction {
    fn id(&self) -> &str {
        "translate_to_english"
    }
    fn display_name(&self) -> &str {
        "翻译为英文"
    }
    fn display_name_en(&self) -> &str {
        "Translate to English"
    }
    fn description(&self) -> &str {
        "使用 AI 将文本翻译为英文"
    }
    fn description_en(&self) -> &str {
        "Translate text to English using AI"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::PlainText, ContentType::Code("".to_string())]
    }

    fn requires_model(&self) -> bool {
        true
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a professional translator. Translate the user's text into English, preserving the original format. Output only the translation, no explanations.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];

        let resp = state::chat_with_thinking(messages, None, Some(0.3), thinking).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "text".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(5)
    }

    async fn execute_stream(
        &self,
        input: ActionInput,
        sender: StreamSender,
    ) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are a professional translator. Translate the user's text into English, preserving the original format. Output only the translation, no explanations.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];
        let resp =
            state::chat_stream_with_thinking(messages, None, Some(0.3), thinking, sender).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "text".to_string(),
        })
    }
}

// ===== 智能摘要 =====

pub struct SummarizeAction;

#[async_trait]
impl Action for SummarizeAction {
    fn id(&self) -> &str {
        "summarize"
    }
    fn display_name(&self) -> &str {
        "智能摘要"
    }
    fn display_name_en(&self) -> &str {
        "Summarize"
    }
    fn description(&self) -> &str {
        "使用 AI 生成文本摘要"
    }
    fn description_en(&self) -> &str {
        "Generate a text summary using AI"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::PlainText]
    }

    fn requires_model(&self) -> bool {
        true
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "请为用户提供的文本生成简明扼要的摘要。突出关键信息，使用要点列表格式。"
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];

        let resp = state::chat_with_thinking(messages, Some(1024), Some(0.5), thinking).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(8)
    }

    async fn execute_stream(
        &self,
        input: ActionInput,
        sender: StreamSender,
    ) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "请为用户提供的文本生成简明扼要的摘要。突出关键信息，使用要点列表格式。"
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];
        let resp =
            state::chat_stream_with_thinking(messages, Some(1024), Some(0.5), thinking, sender)
                .await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
    }
}

// ===== 代码解释 =====

pub struct CodeExplainAction;

#[async_trait]
impl Action for CodeExplainAction {
    fn id(&self) -> &str {
        "code_explain"
    }
    fn display_name(&self) -> &str {
        "代码解释"
    }
    fn display_name_en(&self) -> &str {
        "Explain Code"
    }
    fn description(&self) -> &str {
        "使用 AI 解释代码的功能和逻辑"
    }
    fn description_en(&self) -> &str {
        "Explain code functionality and logic using AI"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Code("".to_string())]
    }

    fn requires_model(&self) -> bool {
        true
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let lang_hint = match &input.content_type {
            ContentType::Code(lang) if !lang.is_empty() => format!("（语言: {}）", lang),
            _ => String::new(),
        };

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "你是一位资深程序员。请解释以下代码的功能和关键逻辑{}。\n\
                     要求：\n\
                     1. 先用一句话概述功能\n\
                     2. 逐段解释关键逻辑\n\
                     3. 指出潜在问题（如有）\n\
                     使用中文回答。",
                    lang_hint
                ),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];

        let resp = state::chat_with_thinking(messages, Some(2048), Some(0.3), thinking).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(10)
    }

    async fn execute_stream(
        &self,
        input: ActionInput,
        sender: StreamSender,
    ) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let lang_hint = match &input.content_type {
            ContentType::Code(lang) if !lang.is_empty() => format!("（语言: {}）", lang),
            _ => String::new(),
        };
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "你是一位资深程序员。请解释以下代码的功能和关键逻辑{}。\n\
                     要求：\n\
                     1. 先用一句话概述功能\n\
                     2. 逐段解释关键逻辑\n\
                     3. 指出潜在问题（如有）\n\
                     使用中文回答。",
                    lang_hint
                ),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];
        let resp =
            state::chat_stream_with_thinking(messages, Some(2048), Some(0.3), thinking, sender)
                .await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
    }
}
