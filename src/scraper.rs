use std::default;
use std::fmt::format;
use std::mem::needs_drop;
use std::sync::Arc;

use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use async_openai::types::chat::ChatChoiceStream;
use async_openai::types::chat::ChatCompletionMessageToolCall;
use async_openai::types::chat::ChatCompletionMessageToolCallChunk;
use async_openai::types::chat::ChatCompletionRequestAssistantMessage;
use async_openai::types::chat::ChatCompletionRequestMessage;
use async_openai::types::chat::ChatCompletionRequestSystemMessage;
use async_openai::types::chat::ChatCompletionRequestToolMessage;
use async_openai::types::chat::ChatCompletionRequestUserMessage;
use async_openai::types::chat::ChatCompletionResponseStream;
use async_openai::types::chat::ChatCompletionTool;
use async_openai::types::chat::CreateChatCompletionRequestArgs;
use async_openai::types::chat::FinishReason;
use async_openai::types::chat::FunctionObjectArgs;
use serde_json::Value;
use serde_json::json;

use futures::StreamExt;
use tracing::instrument;

#[derive(Debug)]
pub struct Fetcher {
    client: reqwest::Client,
}

impl Fetcher {
    pub fn new() -> Self {
        Fetcher {
            client: reqwest::Client::new(),
        }
    }
    pub fn tool_json() -> ChatCompletionTool {
        ChatCompletionTool {
            function: FunctionObjectArgs::default()
                .name("web_fetch")
                .description("fetch the content from the given url.")
                .parameters(json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "the content url, e.g. https://zh.moegirl.org.cn/%E8%8B%A5%E5%8F%B6%E7%9D%A6#.E6.97.A9.E5.B9.B4",
                        }
                    },
                    "required": ["url"],
                })).build().unwrap()
        }
    }
    pub async fn fetch(&self, url: &str) -> Result<String, reqwest::Error> {
        let request = self.client.get(url).build()?;
        self.client.execute(request).await?.text().await
    }
}

#[derive(Debug)]
pub struct Scraper {
    fetcher: Arc<Fetcher>,
    llm_client: async_openai::Client<OpenAIConfig>,
}
impl Scraper {
    pub fn new(llm_config: OpenAIConfig) -> Self {
        Scraper {
            fetcher: Arc::new(Fetcher::new()),
            llm_client: async_openai::Client::with_config(llm_config),
        }
    }

    pub async fn fire(
        &self,
        url: &str,
        model: &str,
        max_iterations: Option<usize>,
    ) -> anyhow::Result<String> {
        let system_prompt = include_str!("./prompt_template/scraper_system");
        let mut messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessage::from(system_prompt).into(),
            ChatCompletionRequestUserMessage::from(format!("从这个url开始进行角色研究 {url}"))
                .into(),
        ];
        let mut final_content: Option<String> = None;

        for i in 0..max_iterations.unwrap_or(10) {
            tracing::info!("iteration: {i}/10");
            let stream = self.create_chat_stream(model, messages.clone()).await?;

            tracing::info!("stream created");

            let (content, tool_calls, finish_reason) = self.process_stream(stream).await?;

            tracing::info!("stream processed");

            if finish_reason == FinishReason::Stop {
                final_content = content;
                break;
            }
            final_content = content.clone();

            //push assistant message with tool calls
            messages.push(
                ChatCompletionRequestAssistantMessage {
                    content: content.map(|c| c.into()),
                    tool_calls: tool_calls
                        .clone()
                        .map(|t| t.into_iter().map(|c| c.into()).collect::<Vec<_>>()),
                    ..Default::default()
                }
                .into(),
            );

            //execute tool calls
            if finish_reason == FinishReason::ToolCalls {
                if let Some(tool_calls) = tool_calls {
                    let tool_responses = self.execute_tools(tool_calls).await;
                    tracing::info!("tool calls executed");
                    messages.extend(tool_responses.into_iter().map(|r| r.into()));
                } else {
                    messages.push(
                        ChatCompletionRequestToolMessage {
                            content: "empty tool call".into(),
                            tool_call_id: "null".to_string(),
                        }
                        .into(),
                    );
                }
            }
            match finish_reason {
                FinishReason::Stop => unreachable!(),
                FinishReason::FunctionCall => {
                    panic!("unsupported openai response, FinishReason::FunctionCall is deprecated.")
                }
                FinishReason::ContentFilter => {
                    panic!("Content filtered, try rerun the command or change model")
                }
                FinishReason::Length => {
                    tracing::warn!(
                        "The llm response was clipped due to exceeding length, the content might be incomplete."
                    );
                    break;
                }
                FinishReason::ToolCalls => {
                    tracing::warn!("Llm responsed with empty tool call.");
                }
            }
        }

        if let Some(final_content) = final_content {
            return Ok(final_content);
        }
        Err(anyhow::anyhow!("No content returned"))
    }
    async fn create_chat_stream(
        &self,
        model: &str,
        messages: Vec<ChatCompletionRequestMessage>,
    ) -> Result<ChatCompletionResponseStream, OpenAIError> {
        let request = CreateChatCompletionRequestArgs::default()
            .max_completion_tokens(100000u32)
            .model(model)
            .messages(messages)
            .tools(Fetcher::tool_json())
            .build()?;
        let mut stream = self.llm_client.chat().create_stream(request).await?;
        Ok(stream)
    }
    async fn process_stream(
        &self,
        mut stream: ChatCompletionResponseStream,
    ) -> anyhow::Result<(
        Option<String>,
        Option<Vec<ChatCompletionMessageToolCall>>,
        FinishReason,
    )> {
        let mut content = String::new();
        let mut tool_calls = Vec::new();
        tracing::info!("start processing stream");

        let mut chunk_cnt = 0;
        let mut chunk_batch_cnt = 0;

        while let Some(result) = stream.next().await {
            let mut response = result?;
            let choice = response.choices.remove(0);

            if let Some(content_delta) = &choice.delta.content {
                //tracing::info!("received content_delta: {content_delta}");
                content.push_str(&content_delta);
                chunk_cnt += 1;
            }

            if let Some(tool_call_chunks) = choice.delta.tool_calls {
                for chunk in tool_call_chunks {
                    Self::accumulate_tool_delta(&mut tool_calls, chunk);
                    chunk_cnt += 1;
                }
            }
            if chunk_cnt % 50 == 0 {
                chunk_batch_cnt += 1;
                tracing::info!("received 50 x {chunk_batch_cnt} chunks");
            }

            if let Some(finish_reason) = choice.finish_reason {
                tracing::info!("finish_reason: {finish_reason:?}");

                let content = if content.is_empty() {
                    None
                } else {
                    Some(content)
                };

                let tool_calls = if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                };

                return Ok((content, tool_calls, finish_reason));
            }
        }
        Err(anyhow::anyhow!("Unexpected Stream Error"))
    }

    fn accumulate_tool_delta(
        tool_calls: &mut Vec<ChatCompletionMessageToolCall>,
        chunk: ChatCompletionMessageToolCallChunk,
    ) {
        //tracing::info!("accumulate_tool_delta: index={}", chunk.index);
        let index = chunk.index as usize;
        while tool_calls.len() <= index {
            tool_calls.push(ChatCompletionMessageToolCall {
                id: String::new(),
                function: Default::default(),
            });
        }

        let tool_call = &mut tool_calls[index];
        if let Some(id) = chunk.id {
            tool_call.id = id;
        }
        if let Some(function_chunk) = chunk.function {
            if let Some(name) = function_chunk.name {
                tool_call.function.name = name;
            }
            if let Some(arguments) = function_chunk.arguments {
                tool_call.function.arguments.push_str(&arguments);
            }
        }
        // tracing::info!(
        //     "accumulated tool: index={}, id={}, name={}, arguments={}",
        //     index,
        //     tool_call.id,
        //     tool_call.function.name,
        //     tool_call.function.arguments
        // )
    }

    async fn execute_tools(
        &self,
        tool_calls: Vec<ChatCompletionMessageToolCall>,
    ) -> Vec<ChatCompletionRequestToolMessage> {
        let mut handles = Vec::new();
        let mut results = Vec::new();

        for tool_call in tool_calls {
            let name = tool_call.function.name.clone();
            let tool_call_id = tool_call.id.clone();

            if name != "web_fetch" {
                results.push(ChatCompletionRequestToolMessage {
                    content: "No such tool".into(),
                    tool_call_id,
                });
                continue;
            }

            let args = serde_json::from_str::<Value>(&tool_call.function.arguments);

            let args = if let Ok(args_json) = args {
                if let Some(url) = args_json["url"].as_str() {
                    url.to_string()
                } else {
                    results.push(ChatCompletionRequestToolMessage {
                        content: "Invalid arguments: missing url".into(),
                        tool_call_id,
                    });
                    continue;
                }
            } else {
                results.push(ChatCompletionRequestToolMessage {
                    content: "Bad argument json".into(),
                    tool_call_id,
                });
                continue;
            };

            tracing::info!("fetching {args}");

            let fetcher_clone = Arc::clone(&self.fetcher);
            handles.push((
                tool_call_id,
                tokio::spawn(async move { fetcher_clone.fetch(&args).await }),
            ));
        }

        for (tool_call_id, handle) in handles {
            let tool_response = handle.await;
            match tool_response {
                Ok(success_tool_calling) => match success_tool_calling {
                    Ok(tool_result) => results.push(ChatCompletionRequestToolMessage {
                        content: tool_result.into(),
                        tool_call_id,
                    }),
                    Err(e) => results.push(ChatCompletionRequestToolMessage {
                        content: format!("Fetch error: {e}").into(),
                        tool_call_id,
                    }),
                },
                Err(e) => {
                    if e.is_panic() {
                        results.push(ChatCompletionRequestToolMessage {
                            content: format!("Fetcher Panic: {}", e).into(),
                            tool_call_id,
                        });
                    } else if e.is_cancelled() {
                        results.push(ChatCompletionRequestToolMessage {
                            content: format!("Fetcher Cancelled: {}", e).into(),
                            tool_call_id,
                        });
                    }
                }
            }
        }

        results
    }
}
