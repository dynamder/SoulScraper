use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionResponseStream, CreateChatCompletionRequestArgs,
    },
};
use futures::StreamExt;

use crate::{data_model::questioner::retrieve::RetrieveAssessInfo, questioner::Quest};

pub struct RetrieveQuestioner {
    llm_client: async_openai::Client<OpenAIConfig>,
}
impl RetrieveQuestioner {
    pub fn new(config: OpenAIConfig) -> Self {
        Self {
            llm_client: async_openai::Client::with_config(config),
        }
    }

    fn prepare_msgs(&self, query: Option<&str>) -> Vec<ChatCompletionRequestMessage> {
        todo!()
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
        let mut assess_info_str = String::new();

        let mut chunk_cnt = 0;
        let mut chunk_batch_cnt = 0;

        while let Some(result) = stream.next().await {
            let mut response = result?;
            let choice = response.choices.remove(0);
            if let Some(text) = choice.delta.content {
                assess_info_str.push_str(&text);
                chunk_cnt += 1;
                if chunk_cnt % 50 == 0 {
                    chunk_batch_cnt += 1;
                    tracing::info!("received 50 x {chunk_batch_cnt} chunks...");
                }
            }
        }
        // for (i, c) in assess_info_str.chars().enumerate() {
        //     if c.is_control() && c != '\t' && c != '\n' && c != '\r' {
        //         tracing::warn!("Control character found at index {}: {:x}", i, c as u32);
        //     }
        // }
        Ok(assess_info_str)
    }
}

impl Quest for RetrieveQuestioner {
    type Output = RetrieveAssessInfo;
    async fn quest(&self, model: &str, query: Option<&str>) -> anyhow::Result<Self::Output> {
        let init_msgs = self.prepare_msgs(query);
        let stream = self.create_stream(init_msgs, model).await?;
        tracing::info!("stream created");
        tracing::info!("processing stream...");
        let raw_response = self.process_stream(stream).await?;

        let retrieve_assess_info =
            serde_json::from_str::<Self::Output>(&raw_response).map_err(|e| {
                tracing::error!("Fatal: fail to deserialize. received:\n{raw_response}");
                e
            })?;
        Ok(retrieve_assess_info)
    }
}
