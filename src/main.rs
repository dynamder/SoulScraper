pub mod data_model;
pub mod extractor;
pub mod io_src;
pub mod scraper;

use std::{path::PathBuf, str::FromStr};

use anyhow::anyhow;
use async_openai::config::OpenAIConfig;
use clap::{CommandFactory, Parser};
use reqwest::Client;
use schemars::{schema_for, schema_for_value};
use secrecy::SecretString;
use serde_json::{Value, json};

use crate::{
    data_model::extractor::{ExtractedInfo, ExtractedNode},
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
    // println!(
    //     "{}",
    //     serde_json::to_string_pretty(&schema_for!(ExtractedInfo)).unwrap()
    // );
    //

    // let test_node = schema_for_value!(json!({
    //   "node_id": "ep1",
    //   "tags": ["过去", "转变", "效忠"],
    //   "mem_type": {
    //     "mem_kind": "Situation",
    //     "sit_kind": "SpecificSituation",
    //     "narrative": "我原本是一名吸血鬼猎人，为了追杀蕾米莉亚而接近红魔馆。在交手中，我被大小姐击败了。她非但没有杀我，还赏识我的能力，赐予我‘十六夜咲夜’的名字，并收我为女仆。从那时起，我发誓效忠于她，从女仆一步步升为女仆长，与她越来越亲近。",
    //     "time_span": "2001-01-01T00:00:00",
    //     "context": {
    //       "location": {
    //         "name": "红魔馆",
    //         "coordinates": ""
    //       },
    //       "participants": [
    //         {"name": "十六夜咲夜", "role": "吸血鬼猎人"},
    //         {"name": "蕾米莉亚·斯卡雷特", "role": "吸血鬼主人"}
    //       ],
    //       "emotions": [
    //         {"name": "敬畏", "intensity": 0.8},
    //         {"name": "感激", "intensity": 0.9}
    //       ],
    //       "environment": {
    //         "atmosphere": "战斗后的宁静与命运的转折",
    //         "tone": "庄严而温暖"
    //       },
    //       "event": [
    //         {
    //           "action": "击败并饶恕",
    //           "action_intensity": 0.9,
    //           "initiator": "蕾米莉亚·斯卡雷特",
    //           "target": "十六夜咲夜"
    //         }
    //       ],
    //       "sensory_data": []
    //     }
    //   }
    // }));
    // if test_node != schema_for!(ExtractedNode) {
    //     panic!(
    //         "schema mismatch!, llm:{}, rust: {}",
    //         serde_json::to_string_pretty(&test_node).unwrap(),
    //         serde_json::to_string_pretty(&schema_for!(ExtractedNode)).unwrap()
    //     );
    // }

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
        let content = input
            .resolve()
            .map_err(|e| anyhow!("Fail to resolve input: \n{e}"))?;

        todo!("question generation not yet implemented.");
    }

    eprintln!("Error: Please specify --scrape, --extract, or --question");
    eprintln!("{}", Args::command().render_help());
    std::process::exit(1);
}
