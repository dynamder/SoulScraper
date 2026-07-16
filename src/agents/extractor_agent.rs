use std::path::Path;

use funera::OpenAIProvider;
use funera::{Agent, AgentEvent, AgentRuntime};
use schemars::schema_for;

use crate::data_model::extractor::{
    apply_edges, EdgeList, GraphNode, GraphNodeList,
};
use crate::data_model::soul_mem::sit::SituationType;
use crate::data_model::soul_mem::MemoryType;
use crate::graph_quality::{
    connect_missing_proc_none, ensure_proc_none_node, normalize_proc_edges, print_report,
    report_to_json, strip_illegal_edges, validate_graph,
};
use crate::util::{format_json_error, sanitize_json, strip_markdown_wrapping};

/// 单次 LLM 调用的结果
struct LlmResult {
    content: String,
}

pub struct ExtractorAgent;

impl ExtractorAgent {
    pub async fn extract(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        character_research: &str,
        debug_dir: Option<&Path>,
    ) -> anyhow::Result<GraphNodeList> {
        // ── Phase 1: nodes ──
        let mut nodes = loop {
            let raw = Self::call_llm(
                api_key, api_base, model,
                "extractor_node_system",
                &format!("根据以下角色信息提取记忆节点（仅节点，不生成边）:\n\n{character_research}"),
            ).await?;

            let cleaned = sanitize_json(&strip_markdown_wrapping(&raw.content));
            let result = serde_json::from_str::<GraphNodeList>(&cleaned);

            match result {
                Ok(nodes) => {
                    if nodes.is_empty() {
                        eprintln!("\nNode list empty, retrying with empty-list context...");
                        Self::save_debug_file(debug_dir, "raw_failed_nodes_empty.json", &cleaned);
                        continue;
                    }
                    if !nodes.iter().any(|n| n.id.contains("self")) {
                        eprintln!("\nsem_self node missing, retrying with context...");
                        Self::save_debug_file(debug_dir, "raw_failed_nodes_no_self.json", &cleaned);
                        continue;
                    }
                    if nodes.len() < 10 {
                        eprintln!("Warning: only {} nodes extracted", nodes.len());
                    }
                    break nodes;
                }
                Err(e) => {
                    let detail = format_json_error(&cleaned, &e);
                    eprintln!("\nNode parse failed.\n{detail}\nAttempting automatic repair...");
                    Self::save_debug_file(debug_dir, "raw_failed_nodes.json", &cleaned);

                    let fix = Self::call_llm(
                        api_key, api_base, model,
                        "extractor_node_system",
                        &format!(
                            "上一步生成的 JSON 解析失败，请参照完整的角色信息修复。\n\n# 角色信息\n{character_research}\n\n# 失败原因\n{detail}\n\n# 损坏的 JSON\n{cleaned}"
                        ),
                    ).await?;

                    let fix_cleaned = sanitize_json(&strip_markdown_wrapping(&fix.content));
                    match serde_json::from_str::<GraphNodeList>(&fix_cleaned) {
                        Ok(nodes) => {
                            if !nodes.iter().any(|n| n.id.contains("self")) {
                                eprintln!("Warning: sem_self missing after fix, continuing");
                            }
                            break nodes;
                        }
                        Err(fatal_e) => {
                            Self::save_debug_file(debug_dir, "raw_failed_nodes_fix.json", &fix_cleaned);
                            return Err(anyhow::anyhow!(
                                "Node fix failed:\n{}", format_json_error(&fix_cleaned, &fatal_e)
                            ));
                        }
                    }
                }
            }
        };

        // ── Phase 2: edges ──
        let node_summary = Self::build_node_summary(&nodes);

        loop {
            let raw = Self::call_llm(
                api_key, api_base, model,
                "extractor_edge_system",
                &format!("根据以下节点列表生成合法的连接边:\n\n{node_summary}"),
            ).await?;

            let cleaned = sanitize_json(&strip_markdown_wrapping(&raw.content));
            let result = serde_json::from_str::<EdgeList>(&cleaned);

            let edges = match result {
                Ok(edges) => edges,
                Err(e) => {
                    let detail = format_json_error(&cleaned, &e);
                    eprintln!("\nEdge parse failed.\n{detail}\nAttempting automatic repair...");
                    Self::save_debug_file(debug_dir, "raw_failed_edges.json", &cleaned);

                    let fix = Self::call_llm(
                        api_key, api_base, model,
                        "extractor_edge_system",
                        &format!(
                            "上一步生成的边列表解析失败，请参照以下节点列表修复:\n\n{node_summary}\n\n# 失败原因\n{detail}\n\n# 损坏的 JSON\n{cleaned}"
                        ),
                    ).await?;

                    let fix_cleaned = sanitize_json(&strip_markdown_wrapping(&fix.content));
                    match serde_json::from_str::<EdgeList>(&fix_cleaned) {
                        Ok(edges) => edges,
                        Err(fatal_e) => {
                            Self::save_debug_file(debug_dir, "raw_failed_edges_fix.json", &fix_cleaned);
                            return Err(anyhow::anyhow!(
                                "Edge fix failed:\n{}", format_json_error(&fix_cleaned, &fatal_e)
                            ));
                        }
                    }
                }
            };

            apply_edges(&mut nodes, &edges);
            strip_illegal_edges(&mut nodes);
            ensure_proc_none_node(&mut nodes);
            connect_missing_proc_none(&mut nodes);
            normalize_proc_edges(&mut nodes);

            let report = validate_graph(&nodes);
            print_report(&report);
            Self::save_stat_file(debug_dir, "graph_stats.json", &report_to_json(&report));

            if report.is_structurally_valid {
                return Ok(nodes);
            }

            // Structural failure — regenerate edges
            let failure_text = report.failures.join("\n");
            let illegal_detail: Vec<String> = report.illegal_edges.iter()
                .map(|e| format!("❌ {} ({}) --{}--> {} ({}) | {}", e.from_id, e.from_type, e.link_type, e.to_id, e.to_type, e.reason))
                .collect();
            let edge_context = if illegal_detail.is_empty() {
                String::new()
            } else {
                format!("\n# 已知非法边（不可出现）\n{}\n", illegal_detail.join("\n"))
            };

            eprintln!("\nStructural quality check failed, regenerating edges...\n{failure_text}");
            Self::save_debug_file(debug_dir, "raw_failed_edges_structure.json", &serde_json::to_string_pretty(&edges).unwrap());

            // Rebuild summary and retry
            let node_summary = Self::build_node_summary(&nodes);
            let raw2 = Self::call_llm(
                api_key, api_base, model,
                "extractor_edge_system",
                &format!(
                    "重新生成边。之前的边结构验证失败，请重新构建。\n\n{node_summary}{edge_context}\n\n# 结构验证失败原因\n{failure_text}"
                ),
            ).await?;

            let cleaned2 = sanitize_json(&strip_markdown_wrapping(&raw2.content));
            match serde_json::from_str::<EdgeList>(&cleaned2) {
                Ok(edges2) => {
                    apply_edges(&mut nodes, &edges2);
                    strip_illegal_edges(&mut nodes);
                    ensure_proc_none_node(&mut nodes);
                    connect_missing_proc_none(&mut nodes);
                    normalize_proc_edges(&mut nodes);
                    let report2 = validate_graph(&nodes);
                    print_report(&report2);
                    Self::save_stat_file(debug_dir, "graph_stats.json", &report_to_json(&report2));
                    if report2.is_structurally_valid {
                        return Ok(nodes);
                    }
                    return Err(anyhow::anyhow!(
                        "Graph quality not met after 2 edge generation attempts:\n{}",
                        report2.failures.join("\n")
                    ));
                }
                Err(fatal_e) => {
                    Self::save_debug_file(debug_dir, "raw_failed_edges_fix2.json", &cleaned2);
                    return Err(anyhow::anyhow!(
                        "Edge reparse failed:\n{}", format_json_error(&cleaned2, &fatal_e)
                    ));
                }
            }
        }
    }

    // ── LLM 调用 ──

    async fn call_llm(
        api_key: &str,
        api_base: Option<&str>,
        model: &str,
        prompt_name: &'static str,
        user_msg: &str,
    ) -> anyhow::Result<LlmResult> {
        let head = match prompt_name {
            "extractor_node_system" => include_str!("../prompt_template/extractor_node_system"),
            "extractor_edge_system" => include_str!("../prompt_template/extractor_edge_system"),
            _ => return Err(anyhow::anyhow!("Unknown prompt: {prompt_name}")),
        };

        let schema = match prompt_name {
            "extractor_node_system" => schema_for!(GraphNodeList),
            "extractor_edge_system" => schema_for!(EdgeList),
            _ => unreachable!(),
        };

        let system = format!("{head}\n\n{}", serde_json::to_string_pretty(&schema).unwrap());

        let runtime = AgentRuntime::<OpenAIProvider>::builder()
            .api_key(api_key.to_string())
            .base_url(api_base.map(|s| s.to_string()))
            .model(model.to_string())
            .build()?;

        let agent = Agent::builder().system_prompt(system).build();

        eprint!("[{prompt_name}] generating");
        let mut handle = agent.fire_stream(user_msg, &runtime).await?;
        while let Some(event) = handle.recv().await {
            if matches!(event, AgentEvent::Done) { break; }
            eprint!(".");
            let _ = std::io::Write::write(&mut std::io::stderr(), b"");
        }
        let resp = handle.await?;
        eprintln!(" done");

        Ok(LlmResult { content: resp.content })
    }

    fn build_node_summary(nodes: &[GraphNode]) -> String {
        let mut lines = Vec::new();
        for node in nodes {
            let type_name = match &node.mem_type {
                MemoryType::Semantic(_) => "Semantic",
                MemoryType::Situation(_) => "Situation",
                MemoryType::Procedure(_) => "Procedure",
            };
            let content = describe_node_content(node);
            lines.push(format!("- {} ({}) tags={:?} 内容: {}", node.id, type_name, node.tags, content));
        }
        lines.join("\n")
    }

    fn save_debug_file(debug_dir: Option<&Path>, filename: &str, content: &str) {
        match debug_dir {
            Some(dir) => {
                let path = dir.join(filename);
                let _ = std::fs::write(&path, content);
                eprintln!("  saved {}", path.display());
            }
            None => {
                let _ = std::fs::write(filename, content);
                eprintln!("  saved ./{filename}");
            }
        }
    }

    fn save_stat_file(debug_dir: Option<&Path>, filename: &str, content: &str) {
        match debug_dir {
            Some(dir) => { let _ = std::fs::write(dir.join(filename), content); }
            None => { let _ = std::fs::write(filename, content); }
        }
    }
}

fn describe_node_content(node: &GraphNode) -> String {
    match &node.mem_type {
        MemoryType::Semantic(sem) => sem.content.chars().take(60).collect(),
        MemoryType::Situation(sit) => match sit {
            SituationType::SpecificSituation(sp) => sp.narrative.chars().take(60).collect(),
            SituationType::AbstractSituation(_abs) => "(abstract situation)".to_string(),
        },
        MemoryType::Procedure(proc) => proc.get_action().get_content().chars().take(60).collect(),
    }
}
