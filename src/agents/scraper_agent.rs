use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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

        let turn = Arc::new(AtomicUsize::new(0));

        let agent = Agent::builder()
            .system_prompt(system_prompt)
            .on_turn_start({
                let turn = turn.clone();
                move || {
                    let t = turn.fetch_add(1, Ordering::Relaxed) + 1;
                    eprint!("\r\x1b[Kscraping... turn {t}/{max_iterations}");
                }
            })
            .on_tool_call({
                let turn = turn.clone();
                move |name, args| {
                    if name == "web_fetch" {
                        if let Some(url) = args.get("url").and_then(|u| u.as_str()) {
                            let t = turn.load(Ordering::Relaxed);
                            eprint!("\r\x1b[Kscraping... turn {t}/{max_iterations} fetching {url}");
                        }
                    }
                }
            })
            .build();

        let resp = agent
            .fire(
                &format!("从这个url开始进行角色研究 {url}"),
                &runtime,
            )
            .await?;

        eprintln!();
        Ok(resp.content)
    }
}
