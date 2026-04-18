use crate::actions::traits::{Action, ActionInput, ActionOutput};
use crate::classifier::rules::ContentType;
use async_trait::async_trait;
use std::time::Duration;

/// URL 转 Markdown 链接操作
pub struct UrlToMarkdownAction;

#[async_trait]
impl Action for UrlToMarkdownAction {
    fn id(&self) -> &str {
        "url_to_markdown"
    }
    fn display_name(&self) -> &str {
        "URL → Markdown"
    }
    fn display_name_en(&self) -> &str {
        "URL → Markdown"
    }
    fn description(&self) -> &str {
        "抓取网页标题，生成 Markdown 链接"
    }
    fn description_en(&self) -> &str {
        "Fetch page title and generate a Markdown link"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Url]
    }

    fn requires_model(&self) -> bool {
        false
    }

    async fn execute(&self, input: ActionInput) -> Result<ActionOutput, String> {
        let url = input.content.trim();

        // 请求网页获取标题
        let title = fetch_title(url).await.unwrap_or_else(|_| url.to_string());

        let markdown = format!("[{}]({})", title, url);
        Ok(ActionOutput {
            result: markdown,
            result_type: "markdown".to_string(),
        })
    }

    fn estimated_duration(&self) -> Duration {
        Duration::from_secs(2)
    }
}

async fn fetch_title(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let body = resp.text().await.map_err(|e| e.to_string())?;

    // 简单提取 <title> 标签
    if let Some(start) = body.find("<title>").or_else(|| body.find("<TITLE>")) {
        let start = start + 7;
        if let Some(end) = body[start..]
            .find("</title>")
            .or_else(|| body[start..].find("</TITLE>"))
        {
            let title = body[start..start + end].trim().to_string();
            if !title.is_empty() {
                return Ok(title);
            }
        }
    }

    Ok(url.to_string())
}
