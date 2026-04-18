use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use async_trait::async_trait;
use std::time::Duration;

/// JSON 格式化操作
pub struct JsonFormatAction;

#[async_trait]
impl Action for JsonFormatAction {
    fn id(&self) -> &str {
        "json_format"
    }
    fn display_name(&self) -> &str {
        "格式化 JSON"
    }
    fn display_name_en(&self) -> &str {
        "Format JSON"
    }
    fn description(&self) -> &str {
        "美化或压缩 JSON 数据"
    }
    fn description_en(&self) -> &str {
        "Pretty-print or compact JSON data"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Json]
    }

    fn requires_model(&self) -> bool {
        false
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let value: serde_json::Value =
            serde_json::from_str(&input.content).map_err(|e| format!("JSON 解析失败: {}", e))?;
        let formatted =
            serde_json::to_string_pretty(&value).map_err(|e| format!("JSON 格式化失败: {}", e))?;
        Ok(ActionOutput {
            result: formatted,
            result_type: "code".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(10)
    }
}

/// JSON → YAML 转换操作
pub struct JsonToYamlAction;

#[async_trait]
impl Action for JsonToYamlAction {
    fn id(&self) -> &str {
        "json_to_yaml"
    }
    fn display_name(&self) -> &str {
        "JSON → YAML"
    }
    fn display_name_en(&self) -> &str {
        "JSON → YAML"
    }
    fn description(&self) -> &str {
        "将 JSON 转换为 YAML 格式"
    }
    fn description_en(&self) -> &str {
        "Convert JSON to YAML format"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Json]
    }

    fn requires_model(&self) -> bool {
        false
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let value: serde_json::Value =
            serde_json::from_str(&input.content).map_err(|e| format!("JSON 解析失败: {}", e))?;
        let yaml = serde_yaml::to_string(&value).map_err(|e| format!("YAML 转换失败: {}", e))?;
        Ok(ActionOutput {
            result: yaml,
            result_type: "code".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(10)
    }
}
