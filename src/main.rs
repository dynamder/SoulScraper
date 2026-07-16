pub mod agents;
pub mod batch;
pub mod data_model;
pub mod graph_quality;
pub mod io_src;
pub mod tools;
pub mod util;

use std::str::FromStr;

use anyhow::anyhow;
use clap::{Args as ClapArgs, CommandFactory, Parser};

use crate::{
    agents::{ExtractorAgent, QuestionerAgent, ScraperAgent},
    batch::{BatchConfig, run_batch},
    data_model::questioner::retrieve::RetrieveAssessInfo,
    data_model::retrieve_question::{
        BlendSweepRaw, PerQueryExpectation, RetrQueryFileRaw, SubQuery, TestCaseQueryRaw,
        TestConfigRaw,
    },
    io_src::{InputSource, OutputSource},
    util::combine_rankings,
};

#[derive(Debug)]
enum QuestionType {
    Retrieve,
    Consolidate,
    Forget,
}

#[derive(Debug, ClapArgs)]
struct QuestionArgs {
    #[arg(long)]
    retrieve: bool,
    #[arg(long)]
    consolidate: bool,
    #[arg(long)]
    forget: bool,
    #[arg(long)]
    query: Option<String>,
    #[arg(long)]
    tendency: Option<String>,
    /// 测试套件名称（输出 RetrQueryFileRaw 时使用）
    #[arg(long, default_value = "retr_sim_smoke_zh")]
    name: String,
    /// 测试套件描述
    #[arg(long, default_value = "向量相似性搜索冒烟测试")]
    description: String,
    /// 指向 Graph JSON 的相对路径
    #[arg(long, default_value = "../graphs/graph.json")]
    graph_path: String,
    /// 相似度阈值
    #[arg(long, default_value_t = 0.0)]
    similarity_threshold: f32,
    /// 每次搜索最大返回数
    #[arg(long, default_value_t = 10)]
    max_results: usize,
    /// 评估 k 值列表（逗号分隔）
    #[arg(long, default_value = "1,3,5")]
    test_k_values: String,
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    model: String,

    #[arg(short, long)]
    scrape: Option<String>,

    #[arg(short, long)]
    extract: Option<String>,

    #[command(flatten)]
    question: Option<QuestionArgs>,

    /// 批量模式：从文件中逐行读取 URL，对每个 URL 执行 scrape → extract → question
    #[arg(short, long)]
    batch: Option<String>,

    /// 批量模式输出根目录
    #[arg(long)]
    out_dir: Option<String>,

    /// 批量模式并发数
    #[arg(long, default_value_t = 1)]
    parallel: usize,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(short, long)]
    api_base: Option<String>,

    /// 权重扫描：tag 权重列表（逗号分隔，如 0.3,0.5,0.7），适用于 question 和 batch 模式
    #[arg(long)]
    blend_tag_sweep: Option<String>,
}

/// 将 LLM 输出的 RetrieveAssessInfo 转换为 test_cases 数组
fn build_test_cases(info: &RetrieveAssessInfo) -> Vec<TestCaseQueryRaw> {
    let mut cases = Vec::new();

    // 每个原子查询 → 单个 test_case
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

    // 每个查询集 → 一个 test_case（含多个 sub_query 变体）
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

    cases
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt().compact().finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("starting...");

    let args = Args::parse();
    let api_key = std::env::var("SOUL_SCRAPER_KEY").expect("env var SOUL_SCRAPER_KEY not set.");

    let model = &args.model;
    let api_base = args.api_base.as_deref();

    if let Some(batch_file) = &args.batch {
        let out_dir = args
            .out_dir
            .ok_or_else(|| anyhow!("--out-dir is required when using --batch"))?;
        let blend_sweep = args.blend_tag_sweep.as_ref().map(|s| {
            let values: Vec<f64> = s
                .split(',')
                .map(|v| v.trim().parse().expect("Invalid blend tag sweep value"))
                .collect();
            BlendSweepRaw { tag_sweep: Some(values), pairs: None }
        });

        let batch_config = BatchConfig {
            api_key: api_key.clone(),
            api_base: args.api_base.clone(),
            model: model.to_string(),
            parallel: args.parallel,
            out_dir: std::path::PathBuf::from(&out_dir),
            blend_sweep,
        };
        run_batch(batch_config, batch_file).await?;
        return Ok(());
    }

    let output_str = args
        .output
        .as_deref()
        .ok_or_else(|| anyhow!("--output is required for single-step mode"))?;
    let output =
        OutputSource::from_str(output_str).map_err(|e| anyhow!("Fail to resolve output: {e}"))?;

    if let Some(url) = &args.scrape {
        println!("Scraping content from {url}");

        let character_research = ScraperAgent::scrape(&api_key, api_base, model, url, 10)
            .await
            .map_err(|e| anyhow!("Scrape failed: {e}"))?;

        output
            .write(&character_research)
            .map_err(|e| anyhow!("Fail to write output: {e}"))?;

        println!("Scrape completed!");
        return Ok(());
    }

    if let Some(input_str) = &args.extract {
        let input = InputSource::from_str(input_str)?;
        println!("{input}");
        let content = input
            .resolve()
            .map_err(|e| anyhow!("Fail to resolve input: \n{e}"))?;

        let extracted_data = ExtractorAgent::extract(&api_key, api_base, model, &content, None)
            .await
            .map_err(|e| anyhow!("Extract failed: {e}"))?;

        output
            .write(&serde_json::to_string_pretty(&extracted_data)?)
            .map_err(|e| anyhow!("Fail to write output: {e}"))?;

        println!("Extract completed!");
        return Ok(());
    }

    if let Some(question_args) = &args.question {
        let QuestionArgs {
            retrieve,
            consolidate,
            forget,
            query,
            tendency,
            name,
            description,
            graph_path,
            similarity_threshold,
            max_results,
            test_k_values,
        } = question_args;

        let question_mode = match (retrieve, consolidate, forget) {
            (true, false, false) => QuestionType::Retrieve,
            (false, true, false) => QuestionType::Consolidate,
            (false, false, true) => QuestionType::Forget,
            _ => anyhow::bail!(
                "Invalid question mode, only one of --retrieve, --consolidate, or --forget should be specified"
            ),
        };

        let query_content = if let Some(query) = query.as_ref() {
            let input = InputSource::from_str(query)?;
            let content = input
                .resolve()
                .map_err(|e| anyhow!("Fail to resolve input: \n{e}"))?;
            Some(content)
        } else {
            None
        };

        match question_mode {
            QuestionType::Retrieve => {
                let generated_question = QuestionerAgent::quest(
                    &api_key,
                    api_base,
                    model,
                    query_content.as_deref(),
                    tendency.as_deref(),
                    None,
                )
                .await?;

                let k_values: Vec<usize> = test_k_values
                    .split(',')
                    .map(|s| s.trim().parse().map_err(|e| anyhow!("Invalid k value: {e}")))
                    .collect::<Result<Vec<_>, _>>()?;

                let test_cases = build_test_cases(&generated_question);

                let blend_sweep = args.blend_tag_sweep.as_ref().map(|s| {
                    let values: Vec<f64> = s
                        .split(',')
                        .map(|v| v.trim().parse().expect("Invalid blend tag sweep value"))
                        .collect();
                    BlendSweepRaw {
                        tag_sweep: Some(values),
                        pairs: None,
                    }
                });

                let retr_file = RetrQueryFileRaw {
                    name: name.clone(),
                    description: description.clone(),
                    graph_path: graph_path.clone(),
                    config: TestConfigRaw {
                        similarity_threshold: *similarity_threshold,
                        max_results: *max_results,
                        test_k_values: k_values,
                    },
                    blend_sweep,
                    test_cases,
                };

                output
                    .write(&serde_json::to_string_pretty(&retr_file)?)
                    .map_err(|e| anyhow!("Fail to write output: {e}"))?;
            }
            QuestionType::Consolidate => {
                todo!("not support yet")
            }
            QuestionType::Forget => {
                todo!("not support yet")
            }
        }

        println!("Question completed!");
        return Ok(());
    }

    eprintln!("Error: Please specify --scrape, --extract, or --question");
    eprintln!("{}", Args::command().render_help());
    std::process::exit(1);
}
