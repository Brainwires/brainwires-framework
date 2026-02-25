use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use brainwires_core::{ChatResponse, ContentBlock, Message, MessageContent, Role, StreamChunk, Usage};
use brainwires_core::{ChatOptions, Provider};
use brainwires_core::Tool;

use super::rate_limiter::RateLimiter;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAIProvider {
    api_key: String,
    model: String,
    http_client: Client,
    organization_id: Option<String>,
    rate_limiter: Option<std::sync::Arc<RateLimiter>>,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http_client: Client::new(),
            organization_id: None,
            rate_limiter: None,
        }
    }

    /// Create a provider with rate limiting (requests per minute).
    pub fn with_rate_limit(api_key: String, model: String, requests_per_minute: u32) -> Self {
        Self {
            api_key,
            model,
            http_client: Client::new(),
            organization_id: None,
            rate_limiter: Some(std::sync::Arc::new(RateLimiter::new(requests_per_minute))),
        }
    }

    /// Wait for rate-limit clearance (no-op if not configured).
    async fn acquire_rate_limit(&self) {
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }
    }

    pub fn with_organization(mut self, org_id: String) -> Self {
        self.organization_id = Some(org_id);
        self
    }

    /// Convert our Message format to OpenAI's format
    fn convert_messages(&self, messages: &[Message]) -> Vec<OpenAIMessage> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                    Role::Tool => "tool",
                };

                let content = match &m.content {
                    MessageContent::Text(text) => OpenAIContent::Text(text.clone()),
                    MessageContent::Blocks(blocks) => {
                        // Check if we have multiple blocks or special types
                        if blocks.len() == 1 {
                            match &blocks[0] {
                                ContentBlock::Text { text } => OpenAIContent::Text(text.clone()),
                                _ => OpenAIContent::Array(
                                    blocks
                                        .iter()
                                        .filter_map(|b| self.convert_content_block(b))
                                        .collect(),
                                ),
                            }
                        } else {
                            OpenAIContent::Array(
                                blocks
                                    .iter()
                                    .filter_map(|b| self.convert_content_block(b))
                                    .collect(),
                            )
                        }
                    }
                };

                OpenAIMessage {
                    role: role.to_string(),
                    content,
                    name: m.name.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                }
            })
            .collect()
    }

    fn convert_content_block(&self, block: &ContentBlock) -> Option<OpenAIContentPart> {
        match block {
            ContentBlock::Text { text } => Some(OpenAIContentPart::Text {
                text: text.clone(),
            }),
            ContentBlock::Image { source } => {
                // Convert image source to OpenAI format
                match source {
                    brainwires_core::ImageSource::Base64 { media_type, data } => {
                        Some(OpenAIContentPart::ImageUrl {
                            image_url: OpenAIImageUrl {
                                url: format!("data:{};base64,{}", media_type, data),
                            },
                        })
                    }
                }
            }
            _ => None,
        }
    }

    /// Convert our Tool format to OpenAI's format
    fn convert_tools(&self, tools: &[Tool]) -> Vec<OpenAITool> {
        tools
            .iter()
            .map(|t| OpenAITool {
                r#type: "function".to_string(),
                function: OpenAIFunction {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.input_schema.properties.clone().unwrap_or_default(),
                },
            })
            .collect()
    }

    /// Check if this is an O1 model (no streaming, no system messages)
    fn is_o1_model(&self) -> bool {
        self.model.starts_with("o1-") || self.model.starts_with("o3-")
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    #[tracing::instrument(name = "provider.chat", skip_all, fields(provider = "openai", model = %self.model))]
    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        let openai_messages = self.convert_messages(messages);

        let mut request_body = json!({
            "model": self.model,
            "messages": openai_messages,
        });

        // O1 models don't support max_tokens, temperature, or system messages
        if !self.is_o1_model() {
            if let Some(max_tokens) = options.max_tokens {
                request_body["max_tokens"] = json!(max_tokens);
            }
            if let Some(temp) = options.temperature {
                request_body["temperature"] = json!(temp);
            }
            if let Some(top_p) = options.top_p {
                request_body["top_p"] = json!(top_p);
            }
        }

        if let Some(tools_list) = tools {
            if !tools_list.is_empty() {
                request_body["tools"] = json!(self.convert_tools(tools_list));
            }
        }

        let mut request = self
            .http_client
            .post(OPENAI_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json");

        if let Some(org_id) = &self.organization_id {
            request = request.header("OpenAI-Organization", org_id);
        }

        self.acquire_rate_limit().await;
        let response = request
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("OpenAI API error ({}): {}", status, error_text);
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        let choice = openai_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No choices in OpenAI response"))?;

        // Convert response to our format
        let content = match choice.message.content {
            OpenAIContent::Text(text) => MessageContent::Text(text),
            OpenAIContent::Array(parts) => MessageContent::Blocks(
                parts
                    .into_iter()
                    .filter_map(|part| match part {
                        OpenAIContentPart::Text { text, .. } => Some(ContentBlock::Text { text }),
                        _ => None,
                    })
                    .collect(),
            ),
        };

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content,
                name: None,
                metadata: None,
            },
            usage: Usage {
                prompt_tokens: openai_response.usage.prompt_tokens,
                completion_tokens: openai_response.usage.completion_tokens,
                total_tokens: openai_response.usage.total_tokens,
            },
            finish_reason: Some(choice.finish_reason),
        })
    }

    fn stream_chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: Option<&'a [Tool]>,
        options: &'a ChatOptions,
    ) -> BoxStream<'a, Result<StreamChunk>> {
        tracing::info!(provider = "openai", model = %self.model, "provider.stream started");
        // O1 models don't support streaming
        if self.is_o1_model() {
            return Box::pin(async_stream::stream! {
                // Fall back to non-streaming for O1 models
                match self.chat(messages, tools, options).await {
                    Ok(response) => {
                        if let Some(text) = response.message.text() {
                            yield Ok(StreamChunk::Text(text.to_string()));
                        }
                        yield Ok(StreamChunk::Usage(response.usage));
                        yield Ok(StreamChunk::Done);
                    }
                    Err(e) => {
                        yield Err(e);
                    }
                }
            });
        }

        Box::pin(async_stream::stream! {
            let openai_messages = self.convert_messages(messages);

            let mut request_body = json!({
                "model": self.model,
                "messages": openai_messages,
                "stream": true,
            });

            if let Some(max_tokens) = options.max_tokens {
                request_body["max_tokens"] = json!(max_tokens);
            }
            if let Some(temp) = options.temperature {
                request_body["temperature"] = json!(temp);
            }
            if let Some(top_p) = options.top_p {
                request_body["top_p"] = json!(top_p);
            }

            if let Some(tools_list) = tools {
                if !tools_list.is_empty() {
                    request_body["tools"] = json!(self.convert_tools(tools_list));
                }
            }

            let mut request = self
                .http_client
                .post(OPENAI_API_URL)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json");

            if let Some(org_id) = &self.organization_id {
                request = request.header("OpenAI-Organization", org_id);
            }

            self.acquire_rate_limit().await;
            let response = match request.json(&request_body).send().await {
                Ok(r) => r,
                Err(e) => {
                    yield Err(e.into());
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                yield Err(anyhow::anyhow!("OpenAI API error ({}): {}", status, error_text));
                return;
            }

            // Parse SSE stream
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

                // Process complete events (delimited by \n\n)
                while let Some(pos) = buffer.find("\n\n") {
                    let event_data = buffer[..pos].to_string();
                    buffer = buffer[pos + 2..].to_string();

                    // Parse SSE event
                    if let Some(data) = event_data.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            yield Ok(StreamChunk::Done);
                            continue;
                        }

                        match serde_json::from_str::<OpenAIStreamChunk>(data) {
                            Ok(chunk) => {
                                if let Some(choice) = chunk.choices.into_iter().next() {
                                    if let Some(delta) = choice.delta {
                                        if let Some(content) = delta.content {
                                            yield Ok(StreamChunk::Text(content));
                                        }
                                        if let Some(tool_calls) = delta.tool_calls {
                                            for tool_call in tool_calls {
                                                yield Ok(StreamChunk::ToolUse {
                                                    id: tool_call.id.unwrap_or_default(),
                                                    name: tool_call.function.name.unwrap_or_default(),
                                                });
                                            }
                                        }
                                    }
                                }

                                if let Some(usage) = chunk.usage {
                                    yield Ok(StreamChunk::Usage(Usage {
                                        prompt_tokens: usage.prompt_tokens,
                                        completion_tokens: usage.completion_tokens,
                                        total_tokens: usage.total_tokens,
                                    }));
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse OpenAI stream chunk: {}", e);
                            }
                        }
                    }
                }
            }
        })
    }
}

// OpenAI API types

#[derive(Debug, Serialize)]
struct OpenAIMessage {
    role: String,
    content: OpenAIContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum OpenAIContent {
    Text(String),
    Array(Vec<OpenAIContentPart>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAIContentPart {
    Text {
        text: String,
    },
    ImageUrl {
        image_url: OpenAIImageUrl,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAIImageUrl {
    url: String,
}

#[derive(Debug, Serialize)]
struct OpenAITool {
    r#type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Serialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    r#type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    content: OpenAIContent,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: Option<OpenAIStreamDelta>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires_core::ToolInputSchema;
    use std::collections::HashMap;

    #[test]
    fn test_openai_provider_new() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "gpt-4");
        assert!(provider.organization_id.is_none());
    }

    #[test]
    fn test_openai_provider_with_organization() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string())
            .with_organization("org-123".to_string());
        assert!(provider.organization_id.is_some());
        assert_eq!(provider.organization_id.unwrap(), "org-123");
    }

    #[test]
    fn test_provider_name() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_is_o1_model_true() {
        let provider = OpenAIProvider::new("test-key".to_string(), "o1-preview".to_string());
        assert!(provider.is_o1_model());
    }

    #[test]
    fn test_is_o1_model_false() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        assert!(!provider.is_o1_model());
    }

    #[test]
    fn test_convert_messages_text() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
    }

    #[test]
    fn test_convert_messages_system() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("You are helpful".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "system");
    }

    #[test]
    fn test_convert_tools() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let mut properties = HashMap::new();
        properties.insert(
            "arg1".to_string(),
            json!({
                "type": "string",
                "description": "First argument"
            }),
        );

        let tools = vec![
            Tool {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                input_schema: ToolInputSchema::object(properties.clone(), vec!["arg1".to_string()]),
                requires_approval: false,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].r#type, "function");
        assert_eq!(converted[0].function.name, "test_tool");
    }

    #[test]
    fn test_convert_tools_empty() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let tools: Vec<Tool> = vec![];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_is_o3_model_true() {
        let provider = OpenAIProvider::new("test-key".to_string(), "o3-preview".to_string());
        assert!(provider.is_o1_model());
    }

    #[test]
    fn test_is_o1_mini_model_true() {
        let provider = OpenAIProvider::new("test-key".to_string(), "o1-mini".to_string());
        assert!(provider.is_o1_model());
    }

    #[test]
    fn test_openai_provider_new_with_different_models() {
        let models = vec!["gpt-4-turbo", "gpt-3.5-turbo", "gpt-4o"];
        for model in models {
            let provider = OpenAIProvider::new("test-key".to_string(), model.to_string());
            assert_eq!(provider.model, model);
            assert_eq!(provider.api_key, "test-key");
        }
    }

    #[test]
    fn test_convert_messages_assistant_role() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("I can help with that".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "assistant");
    }

    #[test]
    fn test_convert_messages_tool_role() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::Tool,
                content: MessageContent::Text("Tool response".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "tool");
    }

    #[test]
    fn test_convert_messages_with_name() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Hello".to_string()),
                name: Some("user_1".to_string()),
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].name, Some("user_1".to_string()));
    }

    #[test]
    fn test_convert_messages_with_text_block() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text { text: "Hello world".to_string() },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        match &converted[0].content {
            OpenAIContent::Text(text) => assert_eq!(text, "Hello world"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_convert_messages_with_multiple_blocks() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text { text: "First block".to_string() },
                    ContentBlock::Text { text: "Second block".to_string() },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        match &converted[0].content {
            OpenAIContent::Array(parts) => assert_eq!(parts.len(), 2),
            _ => panic!("Expected array content"),
        }
    }

    #[test]
    fn test_convert_messages_with_image_block() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Image {
                        source: brainwires_core::ImageSource::Base64 {
                            media_type: "image/png".to_string(),
                            data: "base64data".to_string(),
                        },
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        match &converted[0].content {
            OpenAIContent::Array(parts) => {
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    OpenAIContentPart::ImageUrl { image_url } => {
                        assert!(image_url.url.starts_with("data:image/png;base64,"));
                    }
                    _ => panic!("Expected image url content"),
                }
            }
            _ => panic!("Expected array content"),
        }
    }

    #[test]
    fn test_convert_messages_mixed_text_and_image() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text { text: "Check this image".to_string() },
                    ContentBlock::Image {
                        source: brainwires_core::ImageSource::Base64 {
                            media_type: "image/jpeg".to_string(),
                            data: "imagedata".to_string(),
                        },
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        match &converted[0].content {
            OpenAIContent::Array(parts) => assert_eq!(parts.len(), 2),
            _ => panic!("Expected array content"),
        }
    }

    #[test]
    fn test_convert_content_block_text() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let block = ContentBlock::Text { text: "Test text".to_string() };

        let converted = provider.convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            OpenAIContentPart::Text { text } => assert_eq!(text, "Test text"),
            _ => panic!("Expected text part"),
        }
    }

    #[test]
    fn test_convert_content_block_image() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let block = ContentBlock::Image {
            source: brainwires_core::ImageSource::Base64 {
                media_type: "image/webp".to_string(),
                data: "webpdata".to_string(),
            },
        };

        let converted = provider.convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            OpenAIContentPart::ImageUrl { image_url } => {
                assert_eq!(image_url.url, "data:image/webp;base64,webpdata");
            }
            _ => panic!("Expected image url part"),
        }
    }

    #[test]
    fn test_convert_content_block_tool_use() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let block = ContentBlock::ToolUse {
            id: "tool-1".to_string(),
            name: "test_tool".to_string(),
            input: json!({"key": "value"}),
        };

        // Tool use blocks should return None for OpenAI
        let converted = provider.convert_content_block(&block);
        assert!(converted.is_none());
    }

    #[test]
    fn test_convert_multiple_messages() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
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
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("Hi there!".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "system");
        assert_eq!(converted[1].role, "user");
        assert_eq!(converted[2].role, "assistant");
    }

    #[test]
    fn test_convert_tools_multiple() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let mut properties1 = HashMap::new();
        properties1.insert("arg1".to_string(), json!({"type": "string"}));

        let mut properties2 = HashMap::new();
        properties2.insert("arg2".to_string(), json!({"type": "number"}));

        let tools = vec![
            Tool {
                name: "tool1".to_string(),
                description: "First tool".to_string(),
                input_schema: ToolInputSchema::object(properties1, vec![]),
                requires_approval: false,
                ..Default::default()
            },
            Tool {
                name: "tool2".to_string(),
                description: "Second tool".to_string(),
                input_schema: ToolInputSchema::object(properties2, vec![]),
                requires_approval: true,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].function.name, "tool1");
        assert_eq!(converted[1].function.name, "tool2");
    }

    #[test]
    fn test_convert_tools_without_properties() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let tools = vec![
            Tool {
                name: "simple_tool".to_string(),
                description: "A simple tool".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: None,
                    required: None,
                },
                requires_approval: false,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].function.name, "simple_tool");
        assert!(converted[0].function.parameters.is_empty());
    }

    #[test]
    fn test_organization_id_chaining() {
        let provider = OpenAIProvider::new("key".to_string(), "gpt-4".to_string())
            .with_organization("org-abc".to_string());

        assert_eq!(provider.organization_id, Some("org-abc".to_string()));
        assert_eq!(provider.api_key, "key");
        assert_eq!(provider.model, "gpt-4");
    }

    #[test]
    fn test_empty_api_key() {
        let provider = OpenAIProvider::new("".to_string(), "gpt-4".to_string());
        assert_eq!(provider.api_key, "");
    }

    #[test]
    fn test_empty_model() {
        let provider = OpenAIProvider::new("key".to_string(), "".to_string());
        assert_eq!(provider.model, "");
    }

    #[test]
    fn test_convert_messages_empty() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages: Vec<Message> = vec![];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_openai_content_text_serialization() {
        let content = OpenAIContent::Text("Hello".to_string());
        let serialized = serde_json::to_string(&content).unwrap();
        assert_eq!(serialized, "\"Hello\"");
    }

    #[test]
    fn test_openai_content_array_serialization() {
        let content = OpenAIContent::Array(vec![
            OpenAIContentPart::Text { text: "Test".to_string() },
        ]);
        let serialized = serde_json::to_string(&content).unwrap();
        assert!(serialized.contains("Test"));
    }

    #[test]
    fn test_openai_message_serialization_without_optional_fields() {
        let msg = OpenAIMessage {
            role: "user".to_string(),
            content: OpenAIContent::Text("Hello".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let serialized = serde_json::to_value(&msg).unwrap();
        assert!(!serialized.get("name").is_some());
        assert!(!serialized.get("tool_calls").is_some());
        assert!(!serialized.get("tool_call_id").is_some());
    }

    #[test]
    fn test_openai_message_serialization_with_optional_fields() {
        let msg = OpenAIMessage {
            role: "user".to_string(),
            content: OpenAIContent::Text("Hello".to_string()),
            name: Some("user_1".to_string()),
            tool_calls: None,
            tool_call_id: Some("tc-123".to_string()),
        };

        let serialized = serde_json::to_value(&msg).unwrap();
        assert_eq!(serialized["name"], "user_1");
        assert_eq!(serialized["tool_call_id"], "tc-123");
    }

    #[test]
    fn test_openai_tool_serialization() {
        let tool = OpenAITool {
            r#type: "function".to_string(),
            function: OpenAIFunction {
                name: "test_fn".to_string(),
                description: "Test function".to_string(),
                parameters: HashMap::new(),
            },
        };

        let serialized = serde_json::to_value(&tool).unwrap();
        assert_eq!(serialized["type"], "function");
        assert_eq!(serialized["function"]["name"], "test_fn");
    }

    #[test]
    fn test_openai_response_deserialization() {
        let json = r#"{
            "choices": [{
                "message": {
                    "content": "Test response"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;

        let response: OpenAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.choices.len(), 1);
        assert_eq!(response.usage.prompt_tokens, 10);
        assert_eq!(response.usage.completion_tokens, 5);
        assert_eq!(response.usage.total_tokens, 15);
    }

    #[test]
    fn test_openai_stream_chunk_deserialization() {
        let json = r#"{
            "choices": [{
                "delta": {
                    "content": "Hello"
                }
            }]
        }"#;

        let chunk: OpenAIStreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices.len(), 1);
        assert!(chunk.choices[0].delta.is_some());
    }

    #[test]
    fn test_openai_stream_chunk_with_usage() {
        let json = r#"{
            "choices": [],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 10,
                "total_tokens": 30
            }
        }"#;

        let chunk: OpenAIStreamChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.usage.is_some());
        let usage = chunk.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 20);
        assert_eq!(usage.completion_tokens, 10);
    }

    #[test]
    fn test_openai_content_part_image_deserialization() {
        let json = r#"{
            "type": "image_url",
            "image_url": {
                "url": "data:image/png;base64,abc123"
            }
        }"#;

        let part: OpenAIContentPart = serde_json::from_str(json).unwrap();
        match part {
            OpenAIContentPart::ImageUrl { image_url } => {
                assert_eq!(image_url.url, "data:image/png;base64,abc123");
            }
            _ => panic!("Expected image url part"),
        }
    }

    #[test]
    fn test_openai_tool_call_deserialization() {
        let json = r#"{
            "id": "call_123",
            "type": "function",
            "function": {
                "name": "get_weather",
                "arguments": "{\"city\":\"London\"}"
            }
        }"#;

        let tool_call: OpenAIToolCall = serde_json::from_str(json).unwrap();
        assert_eq!(tool_call.id, Some("call_123".to_string()));
        assert_eq!(tool_call.r#type, "function");
        assert_eq!(tool_call.function.name, Some("get_weather".to_string()));
    }

    #[test]
    fn test_convert_messages_preserves_order() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("System message".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::User,
                content: MessageContent::Text("User message 1".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("Assistant message".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::User,
                content: MessageContent::Text("User message 2".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 4);
        assert_eq!(converted[0].role, "system");
        assert_eq!(converted[1].role, "user");
        assert_eq!(converted[2].role, "assistant");
        assert_eq!(converted[3].role, "user");
    }

    #[test]
    fn test_different_image_media_types() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let media_types = vec!["image/png", "image/jpeg", "image/webp", "image/gif"];

        for media_type in media_types {
            let block = ContentBlock::Image {
                source: brainwires_core::ImageSource::Base64 {
                    media_type: media_type.to_string(),
                    data: "data123".to_string(),
                },
            };

            let converted = provider.convert_content_block(&block);
            assert!(converted.is_some());
            match converted.unwrap() {
                OpenAIContentPart::ImageUrl { image_url } => {
                    assert!(image_url.url.starts_with(&format!("data:{};base64,", media_type)));
                }
                _ => panic!("Expected image url part"),
            }
        }
    }

    #[test]
    fn test_is_o1_model_with_various_names() {
        let o1_models = vec!["o1-preview", "o1-mini", "o1-turbo", "o3-preview", "o3-mini"];
        let non_o1_models = vec!["gpt-4", "gpt-3.5-turbo", "gpt-4o", "gpt-4-turbo", "o1", "o3"];

        for model in o1_models {
            let provider = OpenAIProvider::new("key".to_string(), model.to_string());
            assert!(provider.is_o1_model(), "Expected {} to be detected as o1 model", model);
        }

        for model in non_o1_models {
            let provider = OpenAIProvider::new("key".to_string(), model.to_string());
            assert!(!provider.is_o1_model(), "Expected {} to not be detected as o1 model", model);
        }
    }

    #[test]
    fn test_convert_tools_with_complex_parameters() {
        let provider = OpenAIProvider::new("test-key".to_string(), "gpt-4".to_string());
        let mut properties = HashMap::new();
        properties.insert(
            "location".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "city": {"type": "string"},
                    "country": {"type": "string"}
                },
                "required": ["city"]
            }),
        );
        properties.insert(
            "units".to_string(),
            json!({
                "type": "string",
                "enum": ["celsius", "fahrenheit"]
            }),
        );

        let tools = vec![
            Tool {
                name: "get_weather".to_string(),
                description: "Get weather for a location".to_string(),
                input_schema: ToolInputSchema::object(properties.clone(), vec!["location".to_string()]),
                requires_approval: false,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].function.name, "get_weather");
        assert_eq!(converted[0].function.parameters.len(), 2);
        assert!(converted[0].function.parameters.contains_key("location"));
        assert!(converted[0].function.parameters.contains_key("units"));
    }
}
