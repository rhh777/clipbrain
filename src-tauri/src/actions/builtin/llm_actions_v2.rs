use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use crate::model::backend::{ChatMessage, StreamSender};
use crate::model::state;
use async_trait::async_trait;
use std::time::Duration;

// ===== 文本润色 =====

pub struct PolishTextAction;

#[async_trait]
impl Action for PolishTextAction {
    fn id(&self) -> &str {
        "polish_text"
    }
    fn display_name(&self) -> &str {
        "文本润色"
    }
    fn display_name_en(&self) -> &str {
        "Polish Text"
    }
    fn description(&self) -> &str {
        "使用 AI 润色文本，使其更流畅、专业"
    }
    fn description_en(&self) -> &str {
        "Polish text to make it more fluent and professional"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::PlainText]
    }

    fn requires_model(&self) -> bool {
        true
    }
    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(6)
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是一位专业的文字编辑。请润色以下文本，使其更加流畅、专业、易读。\n\
                         要求：\n\
                         1. 保持原文含义不变\n\
                         2. 修正语法和用词错误\n\
                         3. 提升表达的流畅度和专业性\n\
                         4. 只输出润色后的文本，不要添加解释"
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];

        let resp = state::chat_with_thinking(messages, None, Some(0.5), thinking).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "text".to_string(),
        })
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
                content: "你是一位专业的文字编辑。请润色以下文本，使其更加流畅、专业、易读。\n\
                         要求：\n\
                         1. 保持原文含义不变\n\
                         2. 修正语法和用词错误\n\
                         3. 提升表达的流畅度和专业性\n\
                         4. 只输出润色后的文本，不要添加解释"
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];
        let resp =
            state::chat_stream_with_thinking(messages, None, Some(0.5), thinking, sender).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "text".to_string(),
        })
    }
}

// ===== 英文语法纠错 =====

pub struct FixGrammarAction;

#[async_trait]
impl Action for FixGrammarAction {
    fn id(&self) -> &str {
        "fix_grammar"
    }
    fn display_name(&self) -> &str {
        "英文语法纠错"
    }
    fn display_name_en(&self) -> &str {
        "Fix English Grammar"
    }
    fn description(&self) -> &str {
        "使用 AI 修正英文语法错误，并标注修改说明"
    }
    fn description_en(&self) -> &str {
        "Fix English grammar errors and annotate changes"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::PlainText]
    }

    fn requires_model(&self) -> bool {
        true
    }
    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(6)
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are an English grammar expert. Fix all grammar errors in the text.\n\
                         Output format:\n\
                         **Corrected:**\n\
                         [corrected text]\n\n\
                         **Changes:**\n\
                         - [change 1]\n\
                         - [change 2]\n\
                         If there are no errors, say 'No grammar issues found.'"
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];

        let resp = state::chat_with_thinking(messages, Some(2048), Some(0.2), thinking).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
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
                content: "You are an English grammar expert. Fix all grammar errors in the text.\n\
                         Output format:\n\
                         **Corrected:**\n\
                         [corrected text]\n\n\
                         **Changes:**\n\
                         - [change 1]\n\
                         - [change 2]\n\
                         If there are no errors, say 'No grammar issues found.'"
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];
        let resp =
            state::chat_stream_with_thinking(messages, Some(2048), Some(0.2), thinking, sender)
                .await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
    }
}

// ===== 联系人提取 =====

pub struct ExtractContactsAction;

#[async_trait]
impl Action for ExtractContactsAction {
    fn id(&self) -> &str {
        "extract_contacts"
    }
    fn display_name(&self) -> &str {
        "提取联系人"
    }
    fn display_name_en(&self) -> &str {
        "Extract Contacts"
    }
    fn description(&self) -> &str {
        "使用 AI 从文本中提取姓名、电话、邮箱等联系信息"
    }
    fn description_en(&self) -> &str {
        "Extract names, phones, emails, and other contact info from text"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![
            ContentType::PlainText,
            ContentType::Email,
            ContentType::PhoneNumber,
        ]
    }

    fn requires_model(&self) -> bool {
        true
    }
    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(5)
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是信息提取专家。请从文本中提取所有联系人信息。\n\
                         输出 JSON 数组格式：\n\
                         ```json\n\
                         [\n\
                           {\n\
                             \"name\": \"姓名（如有）\",\n\
                             \"phone\": \"电话（如有）\",\n\
                             \"email\": \"邮箱（如有）\",\n\
                             \"company\": \"公司（如有）\",\n\
                             \"title\": \"职位（如有）\"\n\
                           }\n\
                         ]\n\
                         ```\n\
                         如果没有找到联系人，返回空数组 []。只输出 JSON，不要解释。"
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];

        let resp = state::chat_with_thinking(messages, Some(2048), Some(0.1), thinking).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "json".to_string(),
        })
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
                content: "你是信息提取专家。请从文本中提取所有联系人信息。\n\
                         输出 JSON 数组格式：\n\
                         ```json\n\
                         [\n\
                           {\n\
                             \"name\": \"姓名（如有）\",\n\
                             \"phone\": \"电话（如有）\",\n\
                             \"email\": \"邮箱（如有）\",\n\
                             \"company\": \"公司（如有）\",\n\
                             \"title\": \"职位（如有）\"\n\
                           }\n\
                         ]\n\
                         ```\n\
                         如果没有找到联系人，返回空数组 []。只输出 JSON，不要解释。"
                    .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];
        let resp =
            state::chat_stream_with_thinking(messages, Some(2048), Some(0.1), thinking, sender)
                .await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "json".to_string(),
        })
    }
}

// ===== 正则生成 =====

pub struct GenerateRegexAction;

#[async_trait]
impl Action for GenerateRegexAction {
    fn id(&self) -> &str {
        "generate_regex"
    }
    fn display_name(&self) -> &str {
        "生成正则表达式"
    }
    fn display_name_en(&self) -> &str {
        "Generate Regex"
    }
    fn description(&self) -> &str {
        "使用 AI 根据示例文本生成匹配的正则表达式"
    }
    fn description_en(&self) -> &str {
        "Generate a matching regex from sample text using AI"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::PlainText, ContentType::Code("".to_string())]
    }

    fn requires_model(&self) -> bool {
        true
    }
    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(5)
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let thinking = input.thinking;
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content:
                    "你是正则表达式专家。根据用户提供的文本示例，生成能匹配这些文本的正则表达式。\n\
                         输出格式：\n\
                         **正则表达式：**\n\
                         `regex_pattern`\n\n\
                         **说明：**\n\
                         - 简要解释各部分含义\n\n\
                         **测试：**\n\
                         - 列出匹配结果"
                        .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];

        let resp = state::chat_with_thinking(messages, Some(1024), Some(0.2), thinking).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
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
                content:
                    "你是正则表达式专家。根据用户提供的文本示例，生成能匹配这些文本的正则表达式。\n\
                         输出格式：\n\
                         **正则表达式：**\n\
                         `regex_pattern`\n\n\
                         **说明：**\n\
                         - 简要解释各部分含义\n\n\
                         **测试：**\n\
                         - 列出匹配结果"
                        .to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input.content,
            },
        ];
        let resp =
            state::chat_stream_with_thinking(messages, Some(1024), Some(0.2), thinking, sender)
                .await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
    }
}
