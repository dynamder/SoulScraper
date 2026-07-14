use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::anyhow;
use tokio::sync::Semaphore;

use crate::agents::{ExtractorAgent, QuestionerAgent, ScraperAgent};
use crate::data_model::questioner::retrieve::RetrieveAssessInfo;
use crate::data_model::retrieve_question::{
    PerQueryExpectation, RetrQueryFileRaw, SubQuery, TestCaseQueryRaw, TestConfigRaw,
};

/// 批量处理配置
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub api_key: String,
    pub api_base: Option<String>,
    pub model: String,
    pub parallel: usize,
    pub out_dir: PathBuf,
}

/// 单个 URL 处理结果
#[derive(Debug)]
pub struct BatchResult {
    pub slug: String,
    pub url: String,
    pub success: bool,
    pub error: Option<String>,
}

/// URL 条目
struct UrlEntry {
    url: String,
    slug: String,
}

/// 主入口：读取 URL 文件，并发处理每个 URL
pub async fn run_batch(config: BatchConfig, urls_file: &str) -> anyhow::Result<()> {
    let entries = read_urls(urls_file)?;
    let total = entries.len();
    println!("Loaded {total} URLs from {urls_file}");

    let semaphore = Arc::new(Semaphore::new(config.parallel));
    let mut handles = Vec::with_capacity(total);

    for entry in entries {
        let permit = semaphore.clone().acquire_owned().await?;
        let config = config.clone();

        handles.push(tokio::spawn(async move {
            let _permit = permit;
            let result = process_single_url(&config, &entry).await;
            let status = if result.success { "✓" } else { "✗" };
            println!("[{status}] {} ({})", result.slug, result.url);
            if let Some(ref err) = result.error {
                eprintln!("  error: {err}");
            }
            result
        }));
    }

    let mut successful = 0;
    let mut failed = 0;
    for handle in handles {
        let result = handle.await?;
        if result.success {
            successful += 1;
        } else {
            failed += 1;
        }
    }

    println!(
        "\nDone: {}/{} URLs processed ({} failed)",
        successful, total, failed
    );
    Ok(())
}

/// 处理单个 URL：scrape → extract → question
async fn process_single_url(config: &BatchConfig, entry: &UrlEntry) -> BatchResult {
    let out_dir = config.out_dir.join(&entry.slug);

    // Step 1: Scrape
    let scrape_result = scrape_url(config, &entry, &out_dir).await;
    if let Err(e) = scrape_result {
        return BatchResult {
            slug: entry.slug.clone(),
            url: entry.url.clone(),
            success: false,
            error: Some(format!("scrape failed: {e}")),
        };
    }

    // Step 2: Extract
    let extract_result = extract_graph(config, &entry, &out_dir).await;
    let graph_content = match extract_result {
        Ok(content) => content,
        Err(e) => {
            return BatchResult {
                slug: entry.slug.clone(),
                url: entry.url.clone(),
                success: false,
                error: Some(format!("extract failed: {e}")),
            };
        }
    };

    // Step 3: Question (retrieve)
    let question_result = generate_questions(config, &entry, &out_dir, &graph_content).await;
    if let Err(e) = question_result {
        return BatchResult {
            slug: entry.slug.clone(),
            url: entry.url.clone(),
            success: false,
            error: Some(format!("question failed: {e}")),
        };
    }

    BatchResult {
        slug: entry.slug.clone(),
        url: entry.url.clone(),
        success: true,
        error: None,
    }
}

/// Scrape 步骤
async fn scrape_url(
    config: &BatchConfig,
    entry: &UrlEntry,
    out_dir: &Path,
) -> anyhow::Result<()> {
    let scrape_path = out_dir.join("scrape.md");
    if scrape_path.exists() {
        println!("  scrape already exists, skipping");
        return Ok(());
    }

    println!("  scraping...");
    let result = ScraperAgent::scrape(
        &config.api_key,
        config.api_base.as_deref(),
        &config.model,
        &entry.url,
        10,
    )
    .await?;

    std::fs::create_dir_all(out_dir)?;
    std::fs::write(&scrape_path, &result)?;
    println!("  scrape saved to {}", scrape_path.display());
    Ok(())
}

/// Extract 步骤
async fn extract_graph(
    config: &BatchConfig,
    _entry: &UrlEntry,
    out_dir: &Path,
) -> anyhow::Result<String> {
    let extract_path = out_dir.join("graph.json");
    if extract_path.exists() {
        println!("  graph already exists, skipping");
        return Ok(std::fs::read_to_string(&extract_path)?);
    }

    let scrape_path = out_dir.join("scrape.md");
    let scrape_content = std::fs::read_to_string(&scrape_path)?;

    println!("  extracting...");
    let graph = ExtractorAgent::extract(
        &config.api_key,
        config.api_base.as_deref(),
        &config.model,
        &scrape_content,
        Some(out_dir),
    )
    .await?;

    let json = serde_json::to_string_pretty(&graph)?;
    std::fs::write(&extract_path, &json)?;
    println!("  graph saved to {}", extract_path.display());
    Ok(json)
}

/// Question 步骤
async fn generate_questions(
    config: &BatchConfig,
    entry: &UrlEntry,
    out_dir: &Path,
    graph_content: &str,
) -> anyhow::Result<()> {
    let question_path = out_dir.join("question.json");
    if question_path.exists() {
        println!("  questions already exists, skipping");
        return Ok(());
    }

    println!("  generating questions...");
    let generated = QuestionerAgent::quest(
        &config.api_key,
        config.api_base.as_deref(),
        &config.model,
        Some(graph_content),
        None,
        Some(out_dir),
    )
    .await?;

    let retr_file = build_retr_query_file(&generated, &entry.slug);
    let json = serde_json::to_string_pretty(&retr_file)?;
    std::fs::write(&question_path, &json)?;
    println!("  questions saved to {}", question_path.display());
    Ok(())
}

/// 将 RetrieveAssessInfo 组装为 RetrQueryFileRaw
fn build_retr_query_file(info: &RetrieveAssessInfo, slug: &str) -> RetrQueryFileRaw {
    let mut cases = Vec::new();

    for (i, q) in info.queries.iter().enumerate() {
        let must = q.expected.must_include.clone();
        let expected_per_query = vec![PerQueryExpectation {
            q: 0,
            ranking: must.clone(),
        }];
        cases.push(TestCaseQueryRaw {
            name: format!("query_{}", i),
            description: format!("原子查询 {}", i),
            sub_queries: vec![SubQuery {
                priority: q.priority,
                tag: q.tag.clone(),
                variant: q.variant.clone(),
            }],
            expected_combined_ranking: combine_rankings(&expected_per_query),
            expected_per_query,
            expected_actions: vec![],
        });
    }

    for set in &info.query_sets {
        let mut sub_queries = Vec::new();
        let mut expected_per_query = Vec::new();
        for (j, q) in set.queries.iter().enumerate() {
            let must = q.expected.must_include.clone();
            sub_queries.push(SubQuery {
                priority: q.priority,
                tag: q.tag.clone(),
                variant: q.variant.clone(),
            });
            expected_per_query.push(PerQueryExpectation {
                q: j,
                ranking: must,
            });
        }
        cases.push(TestCaseQueryRaw {
            name: set.set_id.clone(),
            description: set.description.clone(),
            sub_queries,
            expected_combined_ranking: combine_rankings(&expected_per_query),
            expected_per_query,
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
        blend_sweep: None,
        test_cases: cases,
    }
}

/// 将所有 expected_per_query 的 ranking 合并去重，生成 expected_combined_ranking
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

/// 从 URL 列表文件读取 URL
fn read_urls(file_path: &str) -> anyhow::Result<Vec<UrlEntry>> {
    let content = std::fs::read_to_string(file_path)?;
    let entries: Vec<UrlEntry> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|url| {
            let slug = url_to_slug(url);
            UrlEntry {
                url: url.to_string(),
                slug,
            }
        })
        .collect();

    if entries.is_empty() {
        return Err(anyhow!("No valid URLs found in {file_path}"));
    }

    Ok(entries)
}

/// 从 URL 生成目录友好的 slug
fn url_to_slug(url: &str) -> String {
    let cleaned = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');

    let mut slug = String::with_capacity(cleaned.len());
    for ch in cleaned.chars() {
        match ch {
            '/' | '?' | '&' | '=' | '%' | ':' | '.' | ',' | ';' | '+' | '~' | '#' | '@' | '!' | '$' | '\'' | '(' | ')' | '*' => {
                if !slug.ends_with('_') {
                    slug.push('_');
                }
            }
            c if c.is_alphanumeric() || c > '\u{00FF}' => slug.push(c),
            _ => {
                if !slug.ends_with('_') {
                    slug.push('_');
                }
            }
        }
    }
    slug.trim_end_matches('_').to_string()
}
