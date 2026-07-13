use async_trait::async_trait;
use funera::core::re_act::tool::{Tool, ToolCallError};
use serde_json::{Value as JsonValue, json};

#[derive(Debug)]
pub struct WebFetchTool {
    client: reqwest::Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        WebFetchTool {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "fetch the content from the given url."
    }

    fn schema(&self) -> JsonValue {
        json!({
            "type": "function",
            "function": {
                "name": "web_fetch",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "the content url, e.g. https://zh.moegirl.org.cn/%E8%8B%A5%E5%8F%B6%E7%9D%AD#.E6.97.A9.E5.B9.B4"
                        }
                    },
                    "required": ["url"]
                }
            }
        })
    }

    async fn execute(&self, args: JsonValue) -> Result<String, ToolCallError> {
        let url = args["url"]
            .as_str()
            .ok_or_else(|| ToolCallError::ParameterMismatch(json!({"missing": "url"})))?;

        tracing::debug!("fetching {url}");

        let request = self.client.get(url).build().map_err(|e| {
            ToolCallError::ToolExecutionError(anyhow::anyhow!("failed to build request: {e}"))
        })?;

        let response = self.client.execute(request).await.map_err(|e| {
            ToolCallError::ToolExecutionError(anyhow::anyhow!("fetch failed: {e}"))
        })?;

        let text = response.text().await.map_err(|e| {
            ToolCallError::ToolExecutionError(anyhow::anyhow!("failed to read response body: {e}"))
        })?;

        Ok(text)
    }
}
