use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use crate::model::backend::{ImageUrlDetail, VisionContentPart, VisionMessage};
use crate::model::state;
use async_trait::async_trait;
use std::time::Duration;

/// 读取图片文件并返回 base64 data URL
fn image_path_to_data_url(path: &str) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read image: {}", e))?;

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
    Ok(format!("data:image/png;base64,{}", result))
}

/// 构建视觉消息：system prompt + 图片 + 用户文本
fn build_vision_messages(
    system_prompt: &str,
    image_data_url: &str,
    user_text: &str,
) -> Vec<VisionMessage> {
    vec![
        VisionMessage {
            role: "system".to_string(),
            content: vec![VisionContentPart::Text {
                text: system_prompt.to_string(),
            }],
        },
        VisionMessage {
            role: "user".to_string(),
            content: vec![
                VisionContentPart::ImageUrl {
                    image_url: ImageUrlDetail {
                        url: image_data_url.to_string(),
                        detail: Some("auto".to_string()),
                    },
                },
                VisionContentPart::Text {
                    text: user_text.to_string(),
                },
            ],
        },
    ]
}

// ===== 图片 OCR =====

pub struct ImageOcrAction;

#[async_trait]
impl Action for ImageOcrAction {
    fn id(&self) -> &str {
        "image_ocr"
    }
    fn display_name(&self) -> &str {
        "图片文字提取 (OCR)"
    }
    fn display_name_en(&self) -> &str {
        "Image OCR"
    }
    fn description(&self) -> &str {
        "识别图片中的文字内容"
    }
    fn description_en(&self) -> &str {
        "Extract text from image using vision model"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Image]
    }

    fn requires_model(&self) -> bool {
        true
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let image_url = image_path_to_data_url(&input.content)?;
        let messages = build_vision_messages(
            "你是一个精确的 OCR 引擎。请提取图片中的所有文字内容，保持原始排版格式。只输出提取到的文字，不要添加任何解释。",
            &image_url,
            "请提取这张图片中的所有文字。",
        );
        let resp = state::vision_chat(messages, None, Some(0.1)).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "text".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(10)
    }
}

// ===== 图片描述 =====

pub struct ImageDescribeAction;

#[async_trait]
impl Action for ImageDescribeAction {
    fn id(&self) -> &str {
        "image_describe"
    }
    fn display_name(&self) -> &str {
        "图片描述"
    }
    fn display_name_en(&self) -> &str {
        "Describe Image"
    }
    fn description(&self) -> &str {
        "使用 AI 描述图片内容"
    }
    fn description_en(&self) -> &str {
        "Describe image content using vision model"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Image]
    }

    fn requires_model(&self) -> bool {
        true
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let image_url = image_path_to_data_url(&input.content)?;
        let messages = build_vision_messages(
            "你是一个图片描述助手。请详细描述图片中的内容，包括主要对象、场景、颜色、布局等。",
            &image_url,
            "请详细描述这张图片的内容。",
        );
        let resp = state::vision_chat(messages, None, Some(0.5)).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "text".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(10)
    }
}

// ===== 截图表格提取 =====

pub struct ImageTableExtractAction;

#[async_trait]
impl Action for ImageTableExtractAction {
    fn id(&self) -> &str {
        "image_table_extract"
    }
    fn display_name(&self) -> &str {
        "截图表格提取"
    }
    fn display_name_en(&self) -> &str {
        "Extract Table from Image"
    }
    fn description(&self) -> &str {
        "从截图中提取表格数据为 Markdown 格式"
    }
    fn description_en(&self) -> &str {
        "Extract table data from screenshot as Markdown"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Image]
    }

    fn requires_model(&self) -> bool {
        true
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let image_url = image_path_to_data_url(&input.content)?;
        let messages = build_vision_messages(
            "你是一个表格提取专家。请从图片中识别表格数据，并输出为 Markdown 表格格式。如果图片中没有表格，请说明。只输出 Markdown 表格，不要添加解释。",
            &image_url,
            "请提取这张图片中的表格数据。",
        );
        let resp = state::vision_chat(messages, None, Some(0.2)).await?;
        Ok(ActionOutput {
            result: resp.content,
            result_type: "markdown".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(15)
    }
}
