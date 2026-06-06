pub mod retrieve;
use clap::Args;
use serde::Deserialize;

#[derive(Debug)]
pub enum QuestionType {
    Retrieve,
    Consolidate,
    Forget,
}

#[derive(Debug, Args)]
pub struct QuestionArgs {
    #[arg(long)]
    pub retrieve: bool,
    #[arg(long)]
    pub consolidate: bool,
    #[arg(long)]
    pub forget: bool,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long)]
    pub tendency: Option<String>,
}

pub trait Quest {
    type Output: for<'de> Deserialize<'de>;
    async fn quest(&self, model: &str, query: Option<&str>) -> anyhow::Result<Self::Output>;
}
