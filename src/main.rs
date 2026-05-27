pub mod data_model;
pub mod extractor;
pub mod io_src;
pub mod scraper;

use std::str::FromStr;

use anyhow::anyhow;
use async_openai::config::OpenAIConfig;
use clap::{CommandFactory, Parser};

use crate::{
    extractor::Extractor,
    io_src::{InputSource, OutputSource},
    scraper::Scraper,
};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    model: String,

    #[arg(short, long)]
    scrape: Option<String>,

    /// File path, content string, or "-" for stdin
    #[arg(short, long)]
    extract: Option<String>,

    #[arg(short, long)]
    question: Option<String>,

    #[arg(short, long)]
    output: String,

    #[arg(short, long)]
    api_base: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::fmt().compact().finish();
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!("starting...");

    let args = Args::parse();
    let api_key = std::env::var("SOUL_SCRAPER_KEY").expect("env var SOUL_SCRAPER_KEY not set.");

    let output =
        OutputSource::from_str(&args.output).map_err(|e| anyhow!("Fail to resolve output: {e}"))?;

    let openai_config = if let Some(api_base) = &args.api_base {
        OpenAIConfig::default()
            .with_api_base(api_base.clone())
            .with_api_key(api_key)
    } else {
        OpenAIConfig::default().with_api_key(api_key)
    };

    let scraper = Scraper::new(openai_config.clone());
    let extractor = Extractor::new(openai_config);

    let model = &args.model;

    if let Some(url) = &args.scrape {
        println!("Scraping content from {url}");

        let character_research = scraper
            .fire(url, model, Some(10))
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

        let extracted_data = extractor
            .extract(&content, model)
            .await
            .map_err(|e| anyhow!("Extract failed: {e}"))?;

        output
            .write(&serde_json::to_string_pretty(&extracted_data)?)
            .map_err(|e| anyhow!("Fail to write output: {e}"))?;

        println!("Extract completed!");
        return Ok(());
    }

    if let Some(input_str) = &args.question {
        let input = InputSource::from_str(input_str)?;
        let _content = input
            .resolve()
            .map_err(|e| anyhow!("Fail to resolve input: \n{e}"))?;

        todo!("question generation not yet implemented.");
    }

    eprintln!("Error: Please specify --scrape, --extract, or --question");
    eprintln!("{}", Args::command().render_help());
    std::process::exit(1);
}
