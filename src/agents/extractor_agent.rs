use std::path::Path;

use funera::OpenAIProvider;
use funera::{Agent, AgentEvent, AgentRuntime};
use schemars::schema_for;

use crate::data_model::extractor::GraphNodeList;
use crate::graph_quality::{print_report, report_to_json, strip_illegal_edges, validate_graph};
use crate::util::{format_json_error, sanitize_json, strip_markdown_wrapping};

pub struct ExtractorAgent;

impl ExtractorAgent {
    pub async fn extract(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        character_research: &str,
        debug_dir: Option<&Path>,
    ) -> anyhow::Result<GraphNodeList> {
        let system_prompt_head = include_str!("../prompt_template/extractor_system");
        let node_schema = schema_for!(GraphNodeList);
        let system_prompt = format!(
            "{system_prompt_head}\n\n{}",
            serde_json::to_string_pretty(&node_schema).unwrap()
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

        let raw_content = sanitize_json(&strip_markdown_wrapping(&resp.content));

        // Phase 1: parse JSON
        let nodes = serde_json::from_str::<GraphNodeList>(&raw_content);

        let (parsed_nodes, did_fix) = match nodes {
            Ok(nodes) => (nodes, false),
            Err(e) => {
                let error_detail = format_json_error(&raw_content, &e);
                eprintln!("\nJSON parse failed.\n{error_detail}\nAttempting automatic repair...");
                Self::save_debug_file(debug_dir, "raw_failed_extract.json", &raw_content);

                let fix_response = Self::try_fix_json(
                    api_key, api_base, model,
                    &raw_content, character_research, &error_detail,
                ).await?;

                (serde_json::from_str::<GraphNodeList>(&fix_response).map_err(|fatal_err| {
                    let fatal_detail = format_json_error(&fix_response, &fatal_err);
                    Self::save_debug_file(debug_dir, "raw_failed_extract_fix.json", &fix_response);
                    tracing::error!("fatal error after fix.\noriginal:\n{error_detail}\n\nfix:\n{fatal_detail}");
                    fatal_err
                })?, true)
            }
        };

        // Phase 2: strip illegal edges, then validate structure
        let mut cleaned_nodes = parsed_nodes;
        let failure_snapshot = serde_json::to_string_pretty(&cleaned_nodes).unwrap();
        Self::save_debug_file(debug_dir, "raw_failed_structure.json", &failure_snapshot);
        strip_illegal_edges(&mut cleaned_nodes);
        let report = validate_graph(&cleaned_nodes);
        print_report(&report);
        Self::save_stat_file(debug_dir, "graph_stats.json", &report_to_json(&report));

        if report.is_structurally_valid {
            return Ok(cleaned_nodes);
        }

        if did_fix {
            eprintln!("\nStructural quality not met after regeneration, aborting.");
            return Err(anyhow::anyhow!(
                "Structural quality not met after regeneration:\n{}",
                report.failures.join("\n")
            ));
        }

        // Regenerate from scratch instead of trying to fix the broken JSON
        eprintln!("\nStructural quality check failed, re-extracting with failure context...");
        let regenerated = Self::re_extract(
            api_key, api_base, model,
            character_research, &report.failures.join("\n"),
        ).await?;

        let mut regenerated_nodes: GraphNodeList = serde_json::from_str(&regenerated).map_err(|e| {
            let detail = format_json_error(&regenerated, &e);
            Self::save_debug_file(debug_dir, "raw_failed_structure_fix.json", &regenerated);
            anyhow::anyhow!("Regeneration parse failed:\n{detail}")
        })?;

        strip_illegal_edges(&mut regenerated_nodes);
        let report2 = validate_graph(&regenerated_nodes);
        print_report(&report2);
        Self::save_stat_file(debug_dir, "graph_stats.json", &report_to_json(&report2));

        if report2.is_structurally_valid {
            Ok(regenerated_nodes)
        } else {
            Err(anyhow::anyhow!(
                "Structural quality not met after regeneration:\n{}",
                report2.failures.join("\n")
            ))
        }
    }

    /// 用结构失败原因作为上下文，完全重新调用 LLM 生成图谱
    async fn re_extract(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        character_research: &str,
        failure_context: &str,
    ) -> anyhow::Result<String> {
        let system_prompt_head = include_str!("../prompt_template/extractor_system");
        let node_schema = schema_for!(GraphNodeList);
        let system_prompt = format!(
            "{system_prompt_head}\n\n{}",
            serde_json::to_string_pretty(&node_schema).unwrap()
        );

        let runtime = AgentRuntime::<OpenAIProvider>::builder()
            .api_key(api_key.to_string())
            .base_url(api_base.map(|s| s.to_string()))
            .model(model.to_string())
            .build()?;

        let agent = Agent::builder().system_prompt(system_prompt).build();

        eprint!("Re-extracting with structure constraints");
        let mut handle = agent
            .fire_stream(&format!(
                "根据以下角色信息进行提取。注意修复上次生成的图谱结构问题：\n\n{failure_context}\n\n{character_research}"
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

        Ok(sanitize_json(&strip_markdown_wrapping(&resp.content)))
    }

    async fn try_fix_json(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        json_str: &str,
        character_research: &str,
        error_detail: &str,
    ) -> anyhow::Result<String> {
        let fixer_system_head = include_str!("../prompt_template/extractor_fix_system");
        let node_schema = schema_for!(GraphNodeList);
        let fixer_system = format!(
            "{fixer_system_head}\n{}",
            serde_json::to_string_pretty(&node_schema).unwrap()
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
                "根据以下角色信息和json进行修复: \n\n#角色信息\n{character_research}\n\n#损坏的Json\n{json_str}\n\n#json错误原因\n{error_detail}"
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

    fn save_debug_file(debug_dir: Option<&Path>, filename: &str, content: &str) {
        match debug_dir {
            Some(dir) => {
                let path = dir.join(filename);
                let _ = std::fs::write(&path, content);
                eprintln!("  detail saved to {}", path.display());
            }
            None => {
                let _ = std::fs::write(filename, content);
                eprintln!("  detail saved to ./{filename}");
            }
        }
    }

    fn save_stat_file(debug_dir: Option<&Path>, filename: &str, content: &str) {
        match debug_dir {
            Some(dir) => {
                let path = dir.join(filename);
                let _ = std::fs::write(&path, content);
            }
            None => {
                let _ = std::fs::write(filename, content);
            }
        }
    }
}
