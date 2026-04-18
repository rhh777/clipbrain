use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use async_trait::async_trait;
use std::time::Duration;

/// 敏感信息脱敏操作
pub struct MaskSensitiveAction;

#[async_trait]
impl Action for MaskSensitiveAction {
    fn id(&self) -> &str {
        "mask_sensitive"
    }
    fn display_name(&self) -> &str {
        "脱敏处理"
    }
    fn display_name_en(&self) -> &str {
        "Mask Sensitive Info"
    }
    fn description(&self) -> &str {
        "对手机号、身份证等敏感信息进行脱敏"
    }
    fn description_en(&self) -> &str {
        "Mask phone numbers, ID cards, and other sensitive information"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![
            ContentType::PhoneNumber,
            ContentType::IdCard,
            ContentType::PlainText,
        ]
    }

    fn requires_model(&self) -> bool {
        false
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let result = mask_text(&input.content);
        Ok(ActionOutput {
            result,
            result_type: "text".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(10)
    }
}

fn mask_text(text: &str) -> String {
    let mut result = text.to_string();

    // 脱敏手机号: 138****1234
    let phone_re = regex::Regex::new(r"(1[3-9]\d)\d{4}(\d{4})").unwrap();
    result = phone_re.replace_all(&result, "$1****$2").to_string();

    // 脱敏身份证: 110***********1234
    let id_re = regex::Regex::new(r"(\d{3})\d{11}(\d{4})").unwrap();
    result = id_re.replace_all(&result, "$1***********$2").to_string();

    // 脱敏邮箱: u***@example.com
    let email_re = regex::Regex::new(r"([\w])[^@]*(@[\w.-]+)").unwrap();
    result = email_re.replace_all(&result, "$1***$2").to_string();

    result
}
