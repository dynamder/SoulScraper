use funera::{Agent, AgentRuntime};
use funera::OpenAIProvider;

use crate::tools::WebFetchTool;

pub struct ScraperAgent;

impl ScraperAgent {
    pub async fn scrape(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        url: &str,
        max_iterations: usize,
    ) -> anyhow::Result<String> {
        let system_prompt = include_str!("../prompt_template/scraper_system");

        let runtime = AgentRuntime::<OpenAIProvider>::builder()
            .api_key(api_key.to_string())
            .base_url(api_base.map(|s| s.to_string()))
            .model(model.to_string())
            .max_iterations(max_iterations)
            .with_tool_instance(Box::new(WebFetchTool::new()))
            .build()?;

        let agent = Agent::builder()
            .system_prompt(system_prompt)
            .build();

        let resp = agent
            .fire(
                &format!("从这个url开始进行角色研究 {url}"),
                &runtime,
            )
            .await?;

        Ok(resp.content)
    }
}
