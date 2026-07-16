use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::anyhow;
use tokio::sync::Semaphore;

use crate::agents::{ExtractorAgent, QuestionerAgent, ScraperAgent};
use crate::data_model::extractor::GraphNode;
use crate::data_model::questioner::retrieve::RetrieveAssessInfo;
use crate::data_model::retrieve_question::{
    BlendSweepRaw, PerQueryExpectation, RetrQueryFileRaw, SubQuery, TestCaseQueryRaw, TestConfigRaw,
};

/// 管道阶段索引
pub const STAGE_SCRAPE: u32 = 0;
pub const STAGE_EXTRACT_NODE: u32 = 1;
pub const STAGE_EXTRACT_EDGE: u32 = 2;
pub const STAGE_QUESTION: u32 = 3;

pub fn stage_from_name(name: &str) -> u32 {
    match name {
        "extract_node" => STAGE_EXTRACT_NODE,
        "extract_edge" => STAGE_EXTRACT_EDGE,
        "question" => STAGE_QUESTION,
        _ => STAGE_SCRAPE,
    }
}

/// 批量处理配置
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub api_key: String,
    pub api_base: Option<String>,
    pub model: String,
    pub parallel: usize,
    pub out_dir: PathBuf,
    pub blend_sweep: Option<BlendSweepRaw>,
    pub resume_from: u32,
    pub ignore_already_fail: bool,
    pub resume: bool,
}

/// URL 条目（支持 name<TAB>url 格式）
struct UrlEntry {
    name: String,
    url: String,
    slug: String,
}

/// 原子日志输出：每条任务先积累到自己的 buffer，完成时一次性输出
#[derive(Clone)]
struct AtomicOutput {
    lock: Arc<std::sync::Mutex<()>>,
    done_count: Arc<AtomicUsize>,
    total: usize,
    start: Arc<std::sync::Mutex<Instant>>,
}

impl AtomicOutput {
    fn new(total: usize) -> Self {
        Self {
            lock: Arc::new(std::sync::Mutex::new(())),
            done_count: Arc::new(AtomicUsize::new(0)),
            total,
            start: Arc::new(std::sync::Mutex::new(Instant::now())),
        }
    }

    fn flush(&self, name: &str, status: &str, log: &[String], error: Option<&str>) {
        let _guard = self.lock.lock().unwrap();
        let done = self.done_count.fetch_add(1, Ordering::SeqCst) + 1;
        let elapsed = self.start.lock().unwrap().elapsed().as_secs();

        for line in log {
            eprintln!("  {line}");
        }
        let total = self.total;
        eprintln!("[{status}] [{done}/{total}] {name} ({done}/{total} finished, {elapsed}s)");
        if let Some(err) = error {
            eprintln!("    原因: {err}");
        }
    }
}

/// 任务日志缓冲
struct TaskLog {
    lines: Vec<String>,
}

impl TaskLog {
    fn new() -> Self {
        Self { lines: Vec::new() }
    }
    fn log(&mut self, msg: String) {
        eprintln!("  {msg}");
        self.lines.push(msg);
    }
    fn log_scrape(&mut self, name: &str) { self.log(format!("[{name}] scraping page...")); }
    fn log_extract(&mut self, name: &str) { self.log(format!("[{name}] extracting nodes & edges...")); }
    fn log_question(&mut self, name: &str) { self.log(format!("[{name}] generating questions...")); }
    fn log_skip(&mut self, name: &str, step: &str) { self.log(format!("[{name}] {step} already exists, skipping")); }
    fn log_success(&mut self, name: &str, step: &str) { self.log(format!("[{name}] {step} done")); }
}

/// 主入口
pub async fn run_batch(config: BatchConfig, urls_file: &str) -> anyhow::Result<()> {
    let entries = read_urls(urls_file)?;
    let total = entries.len();
    eprintln!("Loaded {total} URLs from {urls_file}");
    eprintln!("Output dir: {}", config.out_dir.display());
    eprintln!("Parallel: {}", config.parallel);
    if config.resume {
        eprintln!("Resume mode: skip stages where output already exists, fill missing ones");
    } else if config.resume_from > 0 {
        let stage_name = match config.resume_from {
            STAGE_EXTRACT_NODE => "extract_node",
            STAGE_EXTRACT_EDGE => "extract_edge",
            STAGE_QUESTION => "question",
            _ => "scrape",
        };
        eprintln!("Resume from: {stage_name} (regenerate from this stage onward)");
        if config.ignore_already_fail {
            eprintln!("Ignore already fail: raw_failed_* will be cleared before regeneration");
        }
    }

    let semaphore = Arc::new(Semaphore::new(config.parallel));
    let output = AtomicOutput::new(total);
    let mut handles = Vec::with_capacity(total);

    for entry in entries {
        let permit = semaphore.clone().acquire_owned().await?;
        let config = config.clone();
        let output = output.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let mut log = TaskLog::new();
            let result = process_single_url(&config, &entry, &mut log).await;
            let status = if result.success { "✓" } else { "✗" };
            output.flush(&entry.name, status, &log.lines, result.error.as_deref());
            result
        }));
    }

    let mut successful = 0;
    let mut failed = 0;
    for handle in handles {
        let result = handle.await?;
        if result.success { successful += 1; } else { failed += 1; }
    }

    eprintln!(
        "\nDone: {}/{} URLs processed ({} failed)",
        successful, total, failed
    );
    Ok(())
}

/// 如果启用了 ignore_already_fail，删除 out_dir 中匹配前缀的 raw_failed_* 文件
fn clean_raw_failed(out_dir: &Path, prefix: &str) {
    if !out_dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("raw_failed_") && name_str.contains(prefix) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// 处理单个 URL：scrape → extract_node → extract_edge → question
/// resume_from 决定截止阶段：该阶段及之后的产物全部重生成，之前的产物复用
async fn process_single_url(
    config: &BatchConfig,
    entry: &UrlEntry,
    log: &mut TaskLog,
) -> BatchResult {
    let out_dir = config.out_dir.join(&entry.slug);
    let resume = config.resume_from;
    let should_ignore_fail = config.ignore_already_fail;

    // ── stage 0: scrape ──
    if config.resume && out_dir.join("scrape.md").exists() {
        log.log_skip(&entry.name, "scrape");
    } else if resume == STAGE_SCRAPE {
        if let Err(e) = scrape_url(config, entry, &out_dir, log).await {
            return err_result(&entry, "scrape", e);
        }
    } else if !out_dir.join("scrape.md").exists() {
        if let Err(e) = scrape_url(config, entry, &out_dir, log).await {
            return err_result(&entry, "scrape", e);
        }
    } else {
        log.log_skip(&entry.name, "scrape");
    }

    // ── stage 1: extract_node ──
    if config.resume && out_dir.join("graph.json").exists() {
        log.log_skip(&entry.name, "extract");
    } else if resume <= STAGE_EXTRACT_NODE {
        if should_ignore_fail {
            clean_raw_failed(&out_dir, "node");
        }
        if let Err(e) = extract_nodes_only(config, entry, &out_dir, log).await {
            return err_result(&entry, "extract_node", e);
        }
    } else if !out_dir.join("graph_nodes.json").exists() && !out_dir.join("graph.json").exists() {
        if let Err(e) = extract_graph_full(config, entry, &out_dir, log).await {
            return err_result(&entry, "extract", e);
        }
    } else {
        log.log_skip(&entry.name, "extract_node");
    }

    // ── stage 2: extract_edge (finalize graph.json) ──
    if config.resume && out_dir.join("graph.json").exists() {
        log.log_skip(&entry.name, "extract_edge");
    } else if resume <= STAGE_EXTRACT_EDGE {
        if should_ignore_fail {
            clean_raw_failed(&out_dir, "edge");
        }
        if let Err(e) = extract_graph_edges(config, entry, &out_dir, log).await {
            return err_result(&entry, "extract_edge", e);
        }
    } else if !out_dir.join("graph.json").exists() {
        if let Err(e) = extract_graph_edges(config, entry, &out_dir, log).await {
            return err_result(&entry, "extract_edge", e);
        }
    } else {
        log.log_skip(&entry.name, "extract_edge");
    }

    // ── stage 3: question ──
    let graph_content = match std::fs::read_to_string(out_dir.join("graph.json")) {
        Ok(c) => c,
        Err(e) => return err_result(&entry, "read_graph", format!("cannot read graph.json: {e}")),
    };

    if config.resume && out_dir.join("question.json").exists() {
        log.log_skip(&entry.name, "question");
    } else if resume <= STAGE_QUESTION {
        if should_ignore_fail {
            clean_raw_failed(&out_dir, "question");
        }
        if let Err(e) = generate_questions(config, entry, &out_dir, &graph_content, log).await {
            return err_result(&entry, "question", e);
        }
    } else if !out_dir.join("question.json").exists() {
        if let Err(e) = generate_questions(config, entry, &out_dir, &graph_content, log).await {
            return err_result(&entry, "question", e);
        }
    } else {
        log.log_skip(&entry.name, "question");
    }

    BatchResult { name: entry.name.clone(), url: entry.url.clone(), success: true, error: None }
}

fn err_result(entry: &UrlEntry, stage: &str, e: impl std::fmt::Display) -> BatchResult {
    BatchResult {
        name: entry.name.clone(), url: entry.url.clone(), success: false,
        error: Some(format!("{stage} failed: {e}")),
    }
}

#[derive(Debug)]
pub struct BatchResult {
    pub name: String,
    pub url: String,
    pub success: bool,
    pub error: Option<String>,
}

/// scrape 步骤
async fn scrape_url(
    config: &BatchConfig,
    entry: &UrlEntry,
    out_dir: &Path,
    log: &mut TaskLog,
) -> anyhow::Result<()> {
    log.log_scrape(&entry.name);
    let result = ScraperAgent::scrape(
        &config.api_key, config.api_base.as_deref(), &config.model, &entry.url, 10,
    ).await?;

    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("scrape.md"), &result)?;
    log.log_success(&entry.name, "scrape");
    Ok(())
}

/// 仅提取节点（Phase 1），保存 graph_nodes.json
async fn extract_nodes_only(
    config: &BatchConfig,
    entry: &UrlEntry,
    out_dir: &Path,
    log: &mut TaskLog,
) -> anyhow::Result<()> {
    let scrape_content = std::fs::read_to_string(out_dir.join("scrape.md"))?;

    log.log_extract(&entry.name);
    let nodes = ExtractorAgent::extract_nodes_only(
        &config.api_key, config.api_base.as_deref(), &config.model,
        &scrape_content, Some(out_dir),
    ).await?;

    let json = serde_json::to_string_pretty(&nodes)?;
    std::fs::write(out_dir.join("graph_nodes.json"), &json)?;
    log.log_success(&entry.name, "extract_node");
    Ok(())
}

/// 完整提取：节点 + 边，保存 graph.json（向后兼容）
async fn extract_graph_full(
    config: &BatchConfig,
    entry: &UrlEntry,
    out_dir: &Path,
    log: &mut TaskLog,
) -> anyhow::Result<()> {
    let scrape_content = std::fs::read_to_string(out_dir.join("scrape.md"))?;

    log.log_extract(&entry.name);
    let graph = ExtractorAgent::extract(
        &config.api_key, config.api_base.as_deref(), &config.model,
        &scrape_content, Some(out_dir),
    ).await?;

    let json = serde_json::to_string_pretty(&graph)?;
    std::fs::write(out_dir.join("graph.json"), &json)?;
    std::fs::write(out_dir.join("graph_nodes.json"), &json)?;
    log.log_success(&entry.name, "extract");
    Ok(())
}

/// 基于已有节点生成边（Phase 2），保存 graph.json
async fn extract_graph_edges(
    config: &BatchConfig,
    entry: &UrlEntry,
    out_dir: &Path,
    log: &mut TaskLog,
) -> anyhow::Result<()> {
    // Read existing nodes (from graph_nodes.json if available, or graph.json)
    let nodes_json = out_dir.join("graph_nodes.json");
    let graph_json = out_dir.join("graph.json");
    let nodes: Vec<GraphNode> = if nodes_json.exists() {
        serde_json::from_str(&std::fs::read_to_string(&nodes_json)?)?
    } else if graph_json.exists() {
        serde_json::from_str(&std::fs::read_to_string(&graph_json)?)?
    } else {
        // No existing nodes — run full extraction
        return extract_graph_full(config, entry, out_dir, log).await;
    };

    log.log_extract(&entry.name);
    let mut nodes = nodes;
    ExtractorAgent::extract_edges_only(
        &config.api_key, config.api_base.as_deref(), &config.model,
        &mut nodes, Some(out_dir),
    ).await?;

    let json = serde_json::to_string_pretty(&nodes)?;
    std::fs::write(&graph_json, &json)?;
    log.log_success(&entry.name, "extract_edge");
    Ok(())
}

/// question 步骤
async fn generate_questions(
    config: &BatchConfig,
    entry: &UrlEntry,
    out_dir: &Path,
    graph_content: &str,
    log: &mut TaskLog,
) -> anyhow::Result<()> {
    let question_path = out_dir.join("question.json");
    if question_path.exists() {
        log.log_skip(&entry.name, "question");
        return Ok(());
    }

    log.log_question(&entry.name);
    let generated = QuestionerAgent::quest(
        &config.api_key, config.api_base.as_deref(), &config.model,
        Some(graph_content), None, Some(out_dir),
    ).await?;

    let retr_file = build_retr_query_file(&generated, &entry.slug, &config.blend_sweep);
    let json = serde_json::to_string_pretty(&retr_file)?;
    std::fs::write(&question_path, &json)?;
    log.log_success(&entry.name, "question");
    Ok(())
}

fn build_retr_query_file(
    info: &RetrieveAssessInfo,
    slug: &str,
    blend_sweep: &Option<BlendSweepRaw>,
) -> RetrQueryFileRaw {
    let mut cases = Vec::new();

    for (i, q) in info.queries.iter().enumerate() {
        let must = q.expected.must_include.clone();
        let epq = vec![PerQueryExpectation { q: 0, ranking: must.clone() }];
        cases.push(TestCaseQueryRaw {
            name: format!("query_{i}"),
            description: format!("atomic query {i}"),
            sub_queries: vec![SubQuery {
                priority: q.priority, tag: q.tag.clone(), variant: q.variant.clone(),
            }],
            expected_combined_ranking: combine_rankings(&epq),
            expected_per_query: epq,
            expected_actions: vec![],
        });
    }

    for set in &info.query_sets {
        let mut sq = Vec::new();
        let mut epq = Vec::new();
        for (j, q) in set.queries.iter().enumerate() {
            sq.push(SubQuery {
                priority: q.priority, tag: q.tag.clone(), variant: q.variant.clone(),
            });
            epq.push(PerQueryExpectation { q: j, ranking: q.expected.must_include.clone() });
        }
        cases.push(TestCaseQueryRaw {
            name: set.set_id.clone(),
            description: set.description.clone(),
            sub_queries: sq,
            expected_combined_ranking: combine_rankings(&epq),
            expected_per_query: epq,
            expected_actions: vec![],
        });
    }

    RetrQueryFileRaw {
        name: format!("batch_{slug}"),
        description: format!("Batch generated test data for {slug}"),
        graph_path: "graph.json".to_string(),
        config: TestConfigRaw {
            similarity_threshold: 0.0,
            max_results: 10,
            test_k_values: vec![1, 3, 5],
        },
        blend_sweep: blend_sweep.clone(),
        test_cases: cases,
    }
}

fn combine_rankings(per_query: &[PerQueryExpectation]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for pq in per_query {
        for id in &pq.ranking {
            if seen.insert(id.clone()) {
                result.push(id.clone());
            }
        }
    }
    result
}

/// 读取 URL 文件，支持两种格式：
///   1. 纯 URL（每行一个）
///   2. name<TAB>url（TAB 分隔，name 为角色名）
fn read_urls(file_path: &str) -> anyhow::Result<Vec<UrlEntry>> {
    let content = std::fs::read_to_string(file_path)?;
    let entries: Vec<UrlEntry> = content
        .lines()
        .map(|line| line.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| {
            if let Some(tab_pos) = line.find('\t') {
                let name = line[..tab_pos].trim().to_string();
                let url = line[tab_pos + 1..].trim().to_string();
                let slug = slugify_name(&name);
                UrlEntry { name, url, slug }
            } else {
                let url = line.to_string();
                let slug = url_to_slug(&url);
                let name = slug.clone();
                UrlEntry { name, url, slug }
            }
        })
        .collect();

    if entries.is_empty() {
        return Err(anyhow!("No valid entries found in {file_path}"));
    }

    // Check for duplicate slugs
    let mut seen = std::collections::HashSet::new();
    for e in &entries {
        if !seen.insert(&e.slug) {
            return Err(anyhow!("Duplicate slug: {} from URL {}", e.name, e.url));
        }
    }

    Ok(entries)
}

/// 从角色名生成目录友好的 slug
fn slugify_name(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    for ch in name.chars() {
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => {
                if !slug.ends_with('_') { slug.push('_'); }
            }
            c if c.is_ascii_whitespace() => {
                if !slug.ends_with('_') { slug.push('_'); }
            }
            '.' | '·' | '•' => {
                if !slug.ends_with('_') { slug.push('_'); }
            }
            c => slug.push(c),
        }
    }
    slug.trim_end_matches('_').to_string()
}

/// 从 URL 生成目录友好的 slug（兼容旧格式）
fn url_to_slug(url: &str) -> String {
    let cleaned = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');

    let mut slug = String::with_capacity(cleaned.len());
    for ch in cleaned.chars() {
        match ch {
            '/' | '?' | '&' | '=' | '%' | ':' | '.' | ',' | ';' | '+' | '~' | '#' | '@' | '!' | '$' | '\'' | '(' | ')' | '*' => {
                if !slug.ends_with('_') { slug.push('_'); }
            }
            c if c.is_alphanumeric() || c > '\u{00FF}' => slug.push(c),
            _ => {
                if !slug.ends_with('_') { slug.push('_'); }
            }
        }
    }
    slug.trim_end_matches('_').to_string()
}
