use funera::OpenAIProvider;
use funera::{Agent, AgentEvent, AgentRuntime};
use schemars::schema_for;

use crate::data_model::extractor::ExtractedInfo;

pub struct ExtractorAgent;

impl ExtractorAgent {
    pub async fn extract(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        character_research: &str,
    ) -> anyhow::Result<ExtractedInfo> {
        let system_prompt_head = include_str!("../prompt_template/extractor_system");
        let info_schema = schema_for!(ExtractedInfo);
        let system_prompt = format!(
            "{system_prompt_head}\n\n{}",
            serde_json::to_string_pretty(&info_schema).unwrap()
        );

        let runtime = AgentRuntime::<OpenAIProvider>::builder()
            .api_key(api_key.to_string())
            .base_url(api_base.map(|s| s.to_string()))
            .model(model.to_string())
            .build()?;

        let agent = Agent::builder().system_prompt(system_prompt).build();

        eprint!("Extracting memory graph");
        let mut handle = agent
            .fire_stream(
                &format!("根据以下角色信息进行提取: \n\n{character_research}"),
                &runtime,
            )
            .await?;

        while let Some(event) = handle.recv().await {
            if matches!(event, AgentEvent::Done) {
                break;
            }
            eprint!(".");
            let _ = std::io::Write::write(&mut std::io::stderr(), b"");
        }
        let resp = handle.await?;
        eprintln!(" done");

        let extracted_info = serde_json::from_str::<ExtractedInfo>(&resp.content);

        match extracted_info {
            Ok(info) => Ok(info),
            Err(e) => {
                eprintln!("JSON parse failed, attempting automatic repair...");
                let fix_response = Self::try_fix_json(
                    api_key,
                    api_base,
                    model,
                    &resp.content,
                    character_research,
                    e,
                )
                .await?;
                serde_json::from_str::<ExtractedInfo>(&fix_response).map_err(|fatal_err| {
                    let _ = std::fs::write("raw_response_debug.json", &fix_response);
                    tracing::error!(
                        "fatal error in info deserialization after trying fix. received: \n {fix_response}"
                    );
                    fatal_err.into()
                })
            }
        }
    }

    async fn try_fix_json(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        json_str: &str,
        character_research: &str,
        de_err: serde_json::Error,
    ) -> anyhow::Result<String> {
        let fixer_system_head = include_str!("../prompt_template/extractor_fix_system");
        let info_schema = schema_for!(ExtractedInfo);
        let fixer_system = format!(
            "{fixer_system_head}\n{}",
            serde_json::to_string_pretty(&info_schema).unwrap()
        );

        let runtime = AgentRuntime::<OpenAIProvider>::builder()
            .api_key(api_key.to_string())
            .base_url(api_base.map(|s| s.to_string()))
            .model(model.to_string())
            .build()?;

        let agent = Agent::builder().system_prompt(fixer_system).build();

        eprint!("Repairing JSON");
        let mut handle = agent
            .fire_stream(&format!(
                "根据以下角色信息和json进行修复: \n\n#角色信息\n{character_research}\n\n#损坏的Json\n{json_str}\n\n#json错误原因\n{de_err}"
            ), &runtime)
            .await?;

        while let Some(event) = handle.recv().await {
            if matches!(event, AgentEvent::Done) {
                break;
            }
            eprint!(".");
            let _ = std::io::Write::write(&mut std::io::stderr(), b"");
        }
        let resp = handle.await?;
        eprintln!(" done");

        Ok(resp.content)
    }
}
