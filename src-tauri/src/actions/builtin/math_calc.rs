use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use async_trait::async_trait;
use std::time::Duration;

/// 数学表达式计算操作
pub struct MathCalculateAction;

#[async_trait]
impl Action for MathCalculateAction {
    fn id(&self) -> &str {
        "math_calculate"
    }
    fn display_name(&self) -> &str {
        "计算表达式"
    }
    fn display_name_en(&self) -> &str {
        "Calculate Expression"
    }
    fn description(&self) -> &str {
        "计算数学表达式的结果"
    }
    fn description_en(&self) -> &str {
        "Evaluate a math expression"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::MathExpression]
    }

    fn requires_model(&self) -> bool {
        false
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let expr = input.content.trim();
        let result: f64 = meval::eval_str(expr).map_err(|e| format!("计算失败: {}", e))?;

        let result_str = if result.fract() == 0.0 && result.abs() < i64::MAX as f64 {
            format!("{} = {}", expr, result as i64)
        } else {
            format!("{} = {}", expr, result)
        };

        Ok(ActionOutput {
            result: result_str,
            result_type: "text".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_millis(5)
    }
}
