//! OpenAI Responses API provider (`POST /v1/responses`).
//!
//! The Responses API is a stateful alternative to Chat Completions.
//! Key differences:
//! - Input is `input` not `messages` — supports text string or structured items
//! - Output contains typed items (`message`, `function_call`, `function_call_output`)
//! - `previous_response_id` chains conversations server-side
//! - Streaming uses `response.output_text.delta` events
//! - Supports built-in tools: web search, code interpreter, MCP servers

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use brainwires_core::{
    ChatOptions, ChatResponse, ContentBlock, Message, MessageContent, Provider, Role, StreamChunk,
    Tool, Usage,
};
use crate::rate_limiter::RateLimiter;

const OPENAI_RESPONSES_URL: &str = "https://api.openai.com/v1/responses";

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// An input item for the Responses API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseInputItem {
    /// A message from user/assistant/system.
    Message {
        /// Role: "user", "assistant", "system".
        role: String,
        /// Content text.
        content: String,
    },
    /// A function call output (tool result).
    FunctionCallOutput {
        /// The call ID this result is for.
        call_id: String,
        /// The output text.
        output: String,
    },
}

/// A tool definition in the Responses API format.
#[derive(Debug, Clone, Serialize)]
pub struct ResponseTool {
    /// Tool type: "function".
    pub r#type: String,
    /// Function name.
    pub name: String,
    /// Description.
    pub description: String,
    /// JSON Schema for parameters.
    pub parameters: serde_json::Value,
}

/// Response from the Responses API.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseApiResponse {
    /// Response ID (for chaining with `previous_response_id`).
    pub id: String,
    /// Output items.
    pub output: Vec<ResponseOutputItem>,
    /// Usage statistics.
    #[serde(default)]
    pub usage: Option<ResponseUsage>,
}

/// An output item from the Responses API.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputItem {
    /// A text message.
    Message {
        /// Role (always "assistant").
        role: String,
        /// Content blocks.
        content: Vec<ResponseContentBlock>,
    },
    /// A function call.
    FunctionCall {
        /// Unique call ID.
        id: String,
        /// Function name.
        name: String,
        /// JSON arguments.
        arguments: String,
        /// Call ID.
        call_id: String,
    },
}

/// Content block within a message output item.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseContentBlock {
    /// Output text.
    OutputText {
        /// The text content.
        text: String,
    },
}

/// Usage info from the Responses API.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseUsage {
    /// Input tokens.
    pub input_tokens: u32,
    /// Output tokens.
    pub output_tokens: u32,
    /// Total tokens.
    #[serde(default)]
    pub total_tokens: Option<u32>,
}

/// Streaming event from the Responses API.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseStreamEvent {
    /// Event type (e.g. "response.output_text.delta", "response.completed").
    #[serde(rename = "type")]
    pub event_type: String,
    /// Delta text (for text delta events).
    #[serde(default)]
    pub delta: Option<String>,
    /// Full response (for completed events).
    #[serde(default)]
    pub response: Option<ResponseApiResponse>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// HTTP client for the OpenAI Responses API.
pub struct ResponsesClient {
    api_key: String,
    base_url: String,
    http_client: Client,
    rate_limiter: Option<Arc<RateLimiter>>,
}

impl ResponsesClient {
    /// Create a new Responses API client.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: OPENAI_RESPONSES_URL.to_string(),
            http_client: Client::new(),
            rate_limiter: None,
        }
    }

    /// Set a custom base URL.
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Set rate limiting.
    pub fn with_rate_limit(mut self, rpm: u32) -> Self {
        self.rate_limiter = Some(Arc::new(RateLimiter::new(rpm)));
        self
    }

    async fn acquire_rate_limit(&self) {
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }
    }

    /// Send a non-streaming request.
    pub async fn create_response(
        &self,
        model: &str,
        input: Vec<ResponseInputItem>,
        instructions: Option<&str>,
        tools: Option<&[ResponseTool]>,
        options: &ChatOptions,
        previous_response_id: Option<&str>,
    ) -> Result<ResponseApiResponse> {
        let mut body = json!({
            "model": model,
            "input": input,
        });

        if let Some(instructions) = instructions {
            body["instructions"] = json!(instructions);
        }
        if let Some(tools) = tools {
            if !tools.is_empty() {
                body["tools"] = json!(tools);
                body["tool_choice"] = json!("auto");
            }
        }
        if let Some(prev_id) = previous_response_id {
            body["previous_response_id"] = json!(prev_id);
        }
        if let Some(temp) = options.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = options.max_tokens {
            body["max_output_tokens"] = json!(max_tokens);
        }
        if let Some(top_p) = options.top_p {
            body["top_p"] = json!(top_p);
        }

        self.acquire_rate_limit().await;
        let response = self
            .http_client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Responses API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Responses API error ({}): {}", status, error_text);
        }

        response
            .json()
            .await
            .context("Failed to parse Responses API response")
    }

    /// Open a streaming response.
    pub fn stream_response<'a>(
        &'a self,
        model: &'a str,
        input: Vec<ResponseInputItem>,
        instructions: Option<&'a str>,
        tools: Option<&'a [ResponseTool]>,
        options: &'a ChatOptions,
        previous_response_id: Option<&'a str>,
    ) -> BoxStream<'a, Result<ResponseStreamEvent>> {
        Box::pin(async_stream::stream! {
            let mut body = json!({
                "model": model,
                "input": input,
                "stream": true,
            });

            if let Some(instructions) = instructions {
                body["instructions"] = json!(instructions);
            }
            if let Some(tools) = tools {
                if !tools.is_empty() {
                    body["tools"] = json!(tools);
                    body["tool_choice"] = json!("auto");
                }
            }
            if let Some(prev_id) = previous_response_id {
                body["previous_response_id"] = json!(prev_id);
            }
            if let Some(temp) = options.temperature {
                body["temperature"] = json!(temp);
            }
            if let Some(max_tokens) = options.max_tokens {
                body["max_output_tokens"] = json!(max_tokens);
            }
            if let Some(top_p) = options.top_p {
                body["top_p"] = json!(top_p);
            }

            self.acquire_rate_limit().await;
            let response = match self
                .http_client
                .post(&self.base_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    yield Err(e.into());
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                yield Err(anyhow::anyhow!("Responses API error ({}): {}", status, error_text));
                return;
            }

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        yield Err(e.into());
                        continue;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(pos) = buffer.find("\n\n") {
                    let event_data = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    if let Some(data) = event_data.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            return;
                        }

                        match serde_json::from_str::<ResponseStreamEvent>(data) {
                            Ok(event) => {
                                yield Ok(event);
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse Responses stream event: {}", e);
                            }
                        }
                    }
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Provider trait implementation
// ---------------------------------------------------------------------------

/// Chat provider backed by the OpenAI Responses API.
pub struct OpenAiResponsesProvider {
    client: Arc<ResponsesClient>,
    model: String,
    provider_name: String,
}

impl OpenAiResponsesProvider {
    /// Create a new Responses API provider.
    pub fn new(client: Arc<ResponsesClient>, model: String) -> Self {
        Self {
            client,
            model,
            provider_name: "openai-responses".to_string(),
        }
    }

    /// Set a custom provider name.
    pub fn with_provider_name(mut self, name: impl Into<String>) -> Self {
        self.provider_name = name.into();
        self
    }
}

/// Convert brainwires-core messages to Responses API input items.
fn convert_messages_to_input(messages: &[Message]) -> (Vec<ResponseInputItem>, Option<String>) {
    let mut items = Vec::new();
    let mut system_prompt = None;

    for msg in messages {
        match msg.role {
            Role::System => {
                if let Some(text) = msg.text() {
                    system_prompt = Some(text.to_string());
                }
            }
            Role::User | Role::Assistant => {
                let role = match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    _ => "user",
                };

                if let Some(text) = msg.text() {
                    items.push(ResponseInputItem::Message {
                        role: role.to_string(),
                        content: text.to_string(),
                    });
                }

                // Handle tool results in user messages
                if let MessageContent::Blocks(blocks) = &msg.content {
                    for block in blocks {
                        if let ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            ..
                        } = block
                        {
                            items.push(ResponseInputItem::FunctionCallOutput {
                                call_id: tool_use_id.clone(),
                                output: content.clone(),
                            });
                        }
                    }
                }
            }
            Role::Tool => {
                if let Some(text) = msg.text() {
                    items.push(ResponseInputItem::FunctionCallOutput {
                        call_id: msg.name.clone().unwrap_or_default(),
                        output: text.to_string(),
                    });
                }
            }
        }
    }

    (items, system_prompt)
}

/// Convert brainwires-core tools to Responses API tool definitions.
fn convert_tools(tools: &[Tool]) -> Vec<ResponseTool> {
    tools
        .iter()
        .map(|t| ResponseTool {
            r#type: "function".to_string(),
            name: t.name.clone(),
            description: t.description.clone(),
            parameters: serde_json::to_value(&t.input_schema).unwrap_or(json!({})),
        })
        .collect()
}

/// Parse a Responses API response into a brainwires-core ChatResponse.
fn parse_response(resp: ResponseApiResponse) -> Result<ChatResponse> {
    let mut text_parts = Vec::new();
    let mut content_blocks = Vec::new();

    for item in &resp.output {
        match item {
            ResponseOutputItem::Message { content, .. } => {
                for block in content {
                    match block {
                        ResponseContentBlock::OutputText { text } => {
                            text_parts.push(text.clone());
                            content_blocks.push(ContentBlock::Text { text: text.clone() });
                        }
                    }
                }
            }
            ResponseOutputItem::FunctionCall {
                call_id,
                name,
                arguments,
                ..
            } => {
                let input: serde_json::Value =
                    serde_json::from_str(arguments).unwrap_or(json!({}));
                content_blocks.push(ContentBlock::ToolUse {
                    id: call_id.clone(),
                    name: name.clone(),
                    input,
                });
            }
        }
    }

    let content = if content_blocks.len() == 1 {
        if let Some(ContentBlock::Text { text }) = content_blocks.first() {
            MessageContent::Text(text.clone())
        } else {
            MessageContent::Blocks(content_blocks)
        }
    } else {
        MessageContent::Blocks(content_blocks)
    };

    let usage = resp.usage.map_or(
        Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
        |u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.total_tokens.unwrap_or(u.input_tokens + u.output_tokens),
        },
    );

    Ok(ChatResponse {
        message: Message {
            role: Role::Assistant,
            content,
            name: None,
            metadata: None,
        },
        usage,
        finish_reason: Some("stop".to_string()),
    })
}

#[async_trait]
impl Provider for OpenAiResponsesProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    #[tracing::instrument(name = "provider.chat", skip_all, fields(provider = %self.provider_name, model = %self.model))]
    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        let (input, system) = convert_messages_to_input(messages);
        let response_tools: Vec<ResponseTool> =
            tools.map(|t| convert_tools(t)).unwrap_or_default();
        let tools_ref = if response_tools.is_empty() {
            None
        } else {
            Some(response_tools.as_slice())
        };

        let instructions = system.as_deref().or(options.system.as_deref());

        let resp = self
            .client
            .create_response(&self.model, input, instructions, tools_ref, options, None)
            .await?;

        parse_response(resp)
    }

    fn stream_chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: Option<&'a [Tool]>,
        options: &'a ChatOptions,
    ) -> BoxStream<'a, Result<StreamChunk>> {
        tracing::info!(provider = %self.provider_name, model = %self.model, "provider.stream started");

        let (input, system) = convert_messages_to_input(messages);
        let response_tools: Vec<ResponseTool> =
            tools.map(|t| convert_tools(t)).unwrap_or_default();

        Box::pin(async_stream::stream! {
            let tools_ref = if response_tools.is_empty() {
                None
            } else {
                Some(response_tools.as_slice())
            };

            let instructions = system.as_deref().or(options.system.as_deref());

            let mut raw_stream = self.client.stream_response(
                &self.model,
                input,
                instructions,
                tools_ref,
                options,
                None,
            );

            while let Some(event_result) = raw_stream.next().await {
                match event_result {
                    Ok(event) => {
                        match event.event_type.as_str() {
                            "response.output_text.delta" => {
                                if let Some(delta) = event.delta {
                                    yield Ok(StreamChunk::Text(delta));
                                }
                            }
                            "response.completed" => {
                                if let Some(resp) = event.response {
                                    if let Some(usage) = resp.usage {
                                        yield Ok(StreamChunk::Usage(Usage {
                                            prompt_tokens: usage.input_tokens,
                                            completion_tokens: usage.output_tokens,
                                            total_tokens: usage.total_tokens
                                                .unwrap_or(usage.input_tokens + usage.output_tokens),
                                        }));
                                    }
                                }
                                yield Ok(StreamChunk::Done);
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                    }
                }
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires_core::ToolInputSchema;
    use std::collections::HashMap;

    #[test]
    fn test_convert_messages_simple() {
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let (items, system) = convert_messages_to_input(&messages);
        assert_eq!(items.len(), 1);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_messages_with_system() {
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("You are helpful".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::User,
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let (items, system) = convert_messages_to_input(&messages);
        assert_eq!(items.len(), 1); // System is extracted, not an input item
        assert_eq!(system, Some("You are helpful".to_string()));
    }

    #[test]
    fn test_convert_tools() {
        let mut properties = HashMap::new();
        properties.insert("q".to_string(), json!({"type": "string"}));

        let tools = vec![Tool {
            name: "search".to_string(),
            description: "Search the web".to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["q".to_string()]),
            requires_approval: false,
            ..Default::default()
        }];

        let converted = convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].name, "search");
        assert_eq!(converted[0].r#type, "function");
    }

    #[test]
    fn test_parse_response_text() {
        let resp = ResponseApiResponse {
            id: "resp_123".to_string(),
            output: vec![ResponseOutputItem::Message {
                role: "assistant".to_string(),
                content: vec![ResponseContentBlock::OutputText {
                    text: "Hello!".to_string(),
                }],
            }],
            usage: Some(ResponseUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: Some(15),
            }),
        };

        let chat_response = parse_response(resp).unwrap();
        assert_eq!(chat_response.message.role, Role::Assistant);
        assert_eq!(chat_response.usage.prompt_tokens, 10);
        assert_eq!(chat_response.usage.completion_tokens, 5);

        if let MessageContent::Text(text) = &chat_response.message.content {
            assert_eq!(text, "Hello!");
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_parse_response_with_function_call() {
        let resp = ResponseApiResponse {
            id: "resp_456".to_string(),
            output: vec![
                ResponseOutputItem::Message {
                    role: "assistant".to_string(),
                    content: vec![ResponseContentBlock::OutputText {
                        text: "Let me search".to_string(),
                    }],
                },
                ResponseOutputItem::FunctionCall {
                    id: "fc_1".to_string(),
                    name: "search".to_string(),
                    arguments: r#"{"q":"test"}"#.to_string(),
                    call_id: "call_1".to_string(),
                },
            ],
            usage: None,
        };

        let chat_response = parse_response(resp).unwrap();
        if let MessageContent::Blocks(blocks) = &chat_response.message.content {
            assert_eq!(blocks.len(), 2);
            matches!(&blocks[1], ContentBlock::ToolUse { name, .. } if name == "search");
        } else {
            panic!("Expected blocks content");
        }
    }

    #[test]
    fn test_input_item_serialization() {
        let item = ResponseInputItem::Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["type"], "message");
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "Hello");
    }

    #[test]
    fn test_function_call_output_serialization() {
        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call_1".to_string(),
            output: "result".to_string(),
        };
        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(json["type"], "function_call_output");
        assert_eq!(json["call_id"], "call_1");
    }

    #[test]
    fn test_responses_provider_name() {
        let client = Arc::new(ResponsesClient::new("test-key".to_string()));
        let provider = OpenAiResponsesProvider::new(client, "gpt-4o".to_string());
        assert_eq!(provider.name(), "openai-responses");
    }

    #[test]
    fn test_responses_provider_custom_name() {
        let client = Arc::new(ResponsesClient::new("test-key".to_string()));
        let provider = OpenAiResponsesProvider::new(client, "gpt-4o".to_string())
            .with_provider_name("custom-responses");
        assert_eq!(provider.name(), "custom-responses");
    }
}
