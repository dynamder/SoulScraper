use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestUserMessage, ChatCompletionResponseStream,
        CreateChatCompletionRequestArgs,
    },
};
use futures::StreamExt;
use schemars::schema_for;

use crate::{data_model::questioner::retrieve::RetrieveAssessInfo, questioner::Quest};

pub struct RetrieveQuestioner {
    llm_client: async_openai::Client<OpenAIConfig>,
    tendency: Option<String>,
}
impl RetrieveQuestioner {
    pub fn new(config: OpenAIConfig) -> Self {
        Self {
            llm_client: async_openai::Client::with_config(config),
            tendency: None,
        }
    }

    /// 设置检索查询的生成倾向
    pub fn with_tendency(mut self, tendency: String) -> Self {
        self.tendency = Some(tendency);
        self
    }

    fn prepare_msgs(&self, query: Option<&str>) -> Vec<ChatCompletionRequestMessage> {
        let system_prompt_head =
            include_str!("../prompt_template/questioner/retrieve_system");
        let info_schema = schema_for!(RetrieveAssessInfo);
        let system_prompt = format!(
            "{system_prompt_head}\n\n{}",
            serde_json::to_string_pretty(&info_schema).unwrap()
        );

        let user_msg = match (query, &self.tendency) {
            (Some(q), Some(t)) => format!(
                "根据以下角色记忆图谱，自动生成检索查询集合。倾向：{t}\n\n{q}"
            ),
            (Some(q), None) => format!(
                "根据以下角色记忆图谱，自动生成检索查询集合：\n\n{q}"
            ),
            (None, Some(t)) => format!("自动生成检索查询集合。倾向：{t}"),
            (None, None) => "自动生成检索查询集合。".to_string(),
        };

        vec![
            ChatCompletionRequestSystemMessage::from(system_prompt).into(),
            ChatCompletionRequestUserMessage::from(user_msg).into(),
        ]
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
        Ok(assess_info_str)
    }

    async fn try_fix_json(
        &self,
        json_str: &str,
        query: Option<&str>,
        model: &str,
        de_err: serde_json::Error,
    ) -> anyhow::Result<String> {
        let fixer_head = "你是一个 JSON 修复助手。以下 JSON 解析失败，请根据错误信息和原始上下文修复它，使其符合要求的 Schema。仅输出修复后的 JSON，不包含任何解释。";
        let info_schema = schema_for!(RetrieveAssessInfo);
        let fixer_system = format!(
            "{fixer_head}\n{}",
            serde_json::to_string_pretty(&info_schema).unwrap()
        );

        let user_msg = match (query, &self.tendency) {
            (Some(q), Some(t)) => format!(
                "角色记忆图谱:\n{q}\n\n生成倾向:\n{t}\n\n# 损坏的 JSON\n{json_str}\n\n# 错误原因\n{de_err}"
            ),
            (Some(q), None) => format!(
                "角色记忆图谱:\n{q}\n\n# 损坏的 JSON\n{json_str}\n\n# 错误原因\n{de_err}"
            ),
            (None, Some(t)) => format!(
                "生成倾向:\n{t}\n\n# 损坏的 JSON\n{json_str}\n\n# 错误原因\n{de_err}"
            ),
            (None, None) => format!("# 损坏的 JSON\n{json_str}\n\n# 错误原因\n{de_err}"),
        };

        let messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessage::from(fixer_system).into(),
            ChatCompletionRequestUserMessage::from(user_msg).into(),
        ];

        let stream = self.create_stream(messages, model).await?;
        tracing::info!("json fix stream created");
        self.process_stream(stream).await.map_err(|e| e.into())
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

        let retrieve_assess_info = serde_json::from_str::<Self::Output>(&raw_response);

        let fixed = match retrieve_assess_info {
            Ok(info) => info,
            Err(e) => {
                tracing::warn!("json deserialization failed, trying fix...");
                let fix_response =
                    self.try_fix_json(&raw_response, query, model, e).await?;
                serde_json::from_str::<Self::Output>(&fix_response).map_err(|fatal_err| {
                    tracing::error!(
                        "fatal error in deserialization after fix. received:\n{fix_response}"
                    );
                    fatal_err
                })?
            }
        };

        Ok(fixed)
    }
}
