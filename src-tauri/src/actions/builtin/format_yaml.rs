use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use async_trait::async_trait;
use std::time::Duration;

/// YAML → JSON 转换操作
pub struct YamlToJsonAction;

#[async_trait]
impl Action for YamlToJsonAction {
    fn id(&self) -> &str {
        "yaml_to_json"
    }
    fn display_name(&self) -> &str {
        "YAML → JSON"
    }
    fn display_name_en(&self) -> &str {
        "YAML → JSON"
    }
    fn description(&self) -> &str {
        "将 YAML 转换为 JSON 格式"
    }
    fn description_en(&self) -> &str {
        "Convert YAML to JSON format"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Yaml]
    }

    fn requires_model(&self) -> bool {
        false
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let value: serde_yaml::Value =
            serde_yaml::from_str(&input.content).map_err(|e| format!("YAML 解析失败: {}", e))?;
        let json =
            serde_json::to_string_pretty(&value).map_err(|e| format!("JSON 转换失败: {}", e))?;
        Ok(ActionOutput {
            result: json,
            result_type: "code".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(10)
    }
}

/// YAML 格式化操作
pub struct YamlFormatAction;

#[async_trait]
impl Action for YamlFormatAction {
    fn id(&self) -> &str {
        "yaml_format"
    }
    fn display_name(&self) -> &str {
        "格式化 YAML"
    }
    fn display_name_en(&self) -> &str {
        "Format YAML"
    }
    fn description(&self) -> &str {
        "美化 YAML 数据"
    }
    fn description_en(&self) -> &str {
        "Pretty-print YAML data"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Yaml]
    }

    fn requires_model(&self) -> bool {
        false
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let value: serde_yaml::Value =
            serde_yaml::from_str(&input.content).map_err(|e| format!("YAML 解析失败: {}", e))?;
        let formatted =
            serde_yaml::to_string(&value).map_err(|e| format!("YAML 格式化失败: {}", e))?;
        Ok(ActionOutput {
            result: formatted,
            result_type: "code".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(10)
    }
}
