use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, ChatCompletionResponseStream,
        CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
    },
};
use futures::{StreamExt, TryFutureExt};
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use serde_json::Error;

use crate::data_model::{
    extractor::{ExtractedGraph, ExtractedInfo},
    soul_mem::{MemoryLink, MemoryNote},
};

pub struct Extractor {
    llm_client: async_openai::Client<OpenAIConfig>,
}
impl Extractor {
    pub fn new(config: OpenAIConfig) -> Self {
        Self {
            llm_client: async_openai::Client::with_config(config),
        }
    }
    pub async fn extract(
        &self,
        character_research: &str,
        model: &str,
    ) -> anyhow::Result<ExtractedInfo> {
        let system_prompt_head = include_str!("./prompt_template/extractor_system");
        let info_schema = schema_for!(ExtractedInfo);
        let system_prompt = format!(
            "{system_prompt_head}\n\n{}",
            serde_json::to_string_pretty(&info_schema).unwrap()
        );

        let messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessage::from(system_prompt).into(),
            ChatCompletionRequestUserMessage::from(format!(
                "根据以下角色信息进行提取: \n\n{character_research}"
            ))
            .into(),
        ];

        let stream = self.create_stream(messages, model).await?;
        tracing::info!("stream created");
        tracing::info!("processing stream");
        let response = self.process_stream(stream).await?;

        let extracted_info = serde_json::from_str::<ExtractedInfo>(&response);

        let fixed_extracted_info = match extracted_info {
            Ok(info) => info,
            Err(e) => {
                tracing::warn!("json deserialization failed, try fixing...");
                let fix_response = self
                    .try_fix_json(&response, &character_research, model, e)
                    .await?;
                let fixed_info = serde_json::from_str::<ExtractedInfo>(&fix_response)
                    .map_err(|fatal_err| {
                        let _ = std::fs::write("raw_response_debug.json", &fix_response);
                        tracing::error!("fatal error in info deserialization after trying fix. received: \n {fix_response}");
                        fatal_err
                    })?;
                fixed_info
            }
        };

        Ok(fixed_extracted_info)
    }

    async fn create_stream(
        &self,
        messages: Vec<ChatCompletionRequestMessage>,
        model: &str,
    ) -> Result<ChatCompletionResponseStream, OpenAIError> {
        let request = CreateChatCompletionRequestArgs::default()
            .max_completion_tokens(1000000u32)
            .model(model)
            .messages(messages)
            .build()?;

        self.llm_client.chat().create_stream(request).await
    }

    async fn process_stream(
        &self,
        mut stream: ChatCompletionResponseStream,
    ) -> Result<String, OpenAIError> {
        let mut extracted_graph_str = String::new();

        let mut chunk_cnt = 0;
        let mut chunk_batch_cnt = 0;

        while let Some(result) = stream.next().await {
            let mut response = result?;
            let choice = response.choices.remove(0);
            if let Some(text) = choice.delta.content {
                extracted_graph_str.push_str(&text);
                chunk_cnt += 1;
                if chunk_cnt % 50 == 0 {
                    chunk_batch_cnt += 1;
                    tracing::info!("received 50 x {chunk_batch_cnt} chunks...");
                }
            }
        }
        for (i, c) in extracted_graph_str.chars().enumerate() {
            if c.is_control() && c != '\t' && c != '\n' && c != '\r' {
                tracing::warn!("Control character found at index {}: {:x}", i, c as u32);
            }
        }
        Ok(extracted_graph_str)
    }

    async fn try_fix_json(
        &self,
        json_str: &str,
        character_research: &str,
        model: &str,
        de_err: Error,
    ) -> anyhow::Result<String> {
        let fixer_system_head = include_str!("./prompt_template/extractor_fix_system");
        let info_schema = schema_for!(ExtractedInfo);
        let fixer_system = format!(
            "{fixer_system_head}\n{}",
            serde_json::to_string_pretty(&info_schema).unwrap()
        );

        let messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessage::from(fixer_system).into(),
            ChatCompletionRequestUserMessage::from(format!(
                "根据以下角色信息和json进行修复: \n\n#角色信息\n{character_research}\n\n#损坏的Json\n{json_str}\n\n#json错误原因\n{de_err}"
            ))
            .into(),
        ];

        let stream = self.create_stream(messages, model).await?;
        tracing::info!("json fix stream created");
        tracing::info!("processing json fix stream");
        self.process_stream(stream).await.map_err(|e| e.into())
    }
}
