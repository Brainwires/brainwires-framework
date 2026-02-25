use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use brainwires_core::{ChatResponse, ContentBlock, Message, MessageContent, Role, StreamChunk, Usage};
use brainwires_core::{ChatOptions, Provider};
use brainwires_core::Tool;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    http_client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http_client: Client::new(),
        }
    }

    /// Convert our Message format to Anthropic's format
    fn convert_messages(&self, messages: &[Message]) -> Vec<AnthropicMessage> {
        messages
            .iter()
            .filter(|m| m.role != Role::System) // System goes in separate field
            .map(|m| AnthropicMessage {
                role: match m.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    _ => "user".to_string(),
                },
                content: match &m.content {
                    MessageContent::Text(text) => vec![AnthropicContentBlock::Text {
                        text: text.clone(),
                    }],
                    MessageContent::Blocks(blocks) => blocks
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::Text { text } => Some(AnthropicContentBlock::Text {
                                text: text.clone(),
                            }),
                            ContentBlock::ToolUse { id, name, input } => {
                                Some(AnthropicContentBlock::ToolUse {
                                    id: id.clone(),
                                    name: name.clone(),
                                    input: input.clone(),
                                })
                            }
                            ContentBlock::ToolResult {
                                tool_use_id,
                                content,
                                ..
                            } => Some(AnthropicContentBlock::ToolResult {
                                tool_use_id: tool_use_id.clone(),
                                content: content.clone(),
                            }),
                            _ => None,
                        })
                        .collect(),
                },
            })
            .collect()
    }

    /// Convert our Tool format to Anthropic's format
    fn convert_tools(&self, tools: &[Tool]) -> Vec<AnthropicTool> {
        tools
            .iter()
            .map(|t| AnthropicTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.properties.clone().unwrap_or_default(),
            })
            .collect()
    }

    /// Get system message from messages
    fn get_system_message(&self, messages: &[Message]) -> Option<String> {
        messages
            .iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| m.text().map(|s| s.to_string()))
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        let anthropic_messages = self.convert_messages(messages);
        let system = options
            .system
            .clone()
            .or_else(|| self.get_system_message(messages));

        let mut request_body = json!({
            "model": self.model,
            "messages": anthropic_messages,
            "max_tokens": options.max_tokens.unwrap_or(4096),
        });

        if let Some(sys) = system {
            request_body["system"] = json!(sys);
        }

        if let Some(temp) = options.temperature {
            request_body["temperature"] = json!(temp);
        }

        if let Some(tools_list) = tools {
            request_body["tools"] = json!(self.convert_tools(tools_list));
        }

        let response = self
            .http_client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Anthropic API error ({}): {}", status, error_text);
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        // Convert response to our format
        let content = if anthropic_response.content.len() == 1 {
            match &anthropic_response.content[0] {
                AnthropicContentBlock::Text { text } => MessageContent::Text(text.clone()),
                _ => MessageContent::Blocks(
                    anthropic_response
                        .content
                        .into_iter()
                        .filter_map(|block| match block {
                            AnthropicContentBlock::Text { text } => {
                                Some(ContentBlock::Text { text })
                            }
                            AnthropicContentBlock::ToolUse { id, name, input } => {
                                Some(ContentBlock::ToolUse { id, name, input })
                            }
                            _ => None,
                        })
                        .collect(),
                ),
            }
        } else {
            MessageContent::Blocks(
                anthropic_response
                    .content
                    .into_iter()
                    .filter_map(|block| match block {
                        AnthropicContentBlock::Text { text } => Some(ContentBlock::Text { text }),
                        AnthropicContentBlock::ToolUse { id, name, input } => {
                            Some(ContentBlock::ToolUse { id, name, input })
                        }
                        _ => None,
                    })
                    .collect(),
            )
        };

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content,
                name: None,
                metadata: None,
            },
            usage: Usage {
                prompt_tokens: anthropic_response.usage.input_tokens,
                completion_tokens: anthropic_response.usage.output_tokens,
                total_tokens: anthropic_response.usage.input_tokens
                    + anthropic_response.usage.output_tokens,
            },
            finish_reason: Some(anthropic_response.stop_reason),
        })
    }

    fn stream_chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: Option<&'a [Tool]>,
        options: &'a ChatOptions,
    ) -> BoxStream<'a, Result<StreamChunk>> {
        Box::pin(async_stream::stream! {
            let anthropic_messages = self.convert_messages(messages);
            let system = options
                .system
                .clone()
                .or_else(|| self.get_system_message(messages));

            let mut request_body = json!({
                "model": self.model,
                "messages": anthropic_messages,
                "max_tokens": options.max_tokens.unwrap_or(4096),
                "stream": true,
            });

            if let Some(sys) = system {
                request_body["system"] = json!(sys);
            }

            if let Some(temp) = options.temperature {
                request_body["temperature"] = json!(temp);
            }

            if let Some(tools_list) = tools {
                request_body["tools"] = json!(self.convert_tools(tools_list));
            }

            let response = match self
                .http_client
                .post(ANTHROPIC_API_URL)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("content-type", "application/json")
                .json(&request_body)
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
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                yield Err(anyhow::anyhow!("Anthropic API error ({}): {}", status, error_text));
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

                        match serde_json::from_str::<AnthropicStreamEvent>(data) {
                            Ok(event) => {
                                match event.event_type.as_str() {
                                    "content_block_delta" => {
                                        if let Some(delta) = event.delta {
                                            if let Some(text) = delta.text {
                                                yield Ok(StreamChunk::Text(text));
                                            }
                                        }
                                    }
                                    "message_delta" => {
                                        if let Some(usage) = event.usage {
                                            yield Ok(StreamChunk::Usage(Usage {
                                                prompt_tokens: 0,
                                                completion_tokens: usage.output_tokens,
                                                total_tokens: usage.output_tokens,
                                            }));
                                        }
                                    }
                                    "message_stop" => {
                                        yield Ok(StreamChunk::Done);
                                    }
                                    _ => {}
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to parse Anthropic stream event: {}", e);
                            }
                        }
                    }
                }
            }
        })
    }
}

// Anthropic API types

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    stop_reason: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicDelta>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires_core::ToolInputSchema;
    use std::collections::HashMap;

    #[test]
    fn test_anthropic_provider_new() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "claude-3-sonnet");
    }

    #[test]
    fn test_provider_name() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        assert_eq!(provider.name(), "anthropic");
    }

    #[test]
    fn test_convert_messages_text() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
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
    fn test_convert_messages_filters_system() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("System prompt".to_string()),
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

        let converted = provider.convert_messages(&messages);
        // System message should be filtered out
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
    }

    #[test]
    fn test_convert_messages_with_blocks() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text { text: "Response".to_string() },
                    ContentBlock::ToolUse {
                        id: "tool-1".to_string(),
                        name: "test_tool".to_string(),
                        input: json!({"arg": "value"}),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "assistant");
        assert_eq!(converted[0].content.len(), 2);
    }

    #[test]
    fn test_convert_messages_with_tool_result() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::ToolResult {
                        tool_use_id: "tool-1".to_string(),
                        content: "Result".to_string(),
                        is_error: Some(false),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].content.len(), 1);
    }

    #[test]
    fn test_convert_tools() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
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
        assert_eq!(converted[0].name, "test_tool");
        assert_eq!(converted[0].description, "A test tool");
        assert!(converted[0].input_schema.contains_key("arg1"));
    }

    #[test]
    fn test_convert_tools_empty() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let tools: Vec<Tool> = vec![];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_get_system_message_found() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("You are a helpful assistant".to_string()),
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

        let system = provider.get_system_message(&messages);
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "You are a helpful assistant");
    }

    #[test]
    fn test_get_system_message_not_found() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let system = provider.get_system_message(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_messages_multiple_roles() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Question".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("Answer".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::User,
                content: MessageContent::Text("Follow-up".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "assistant");
        assert_eq!(converted[2].role, "user");
    }

    #[test]
    fn test_convert_tools_multiple() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let tools = vec![
            Tool {
                name: "tool1".to_string(),
                description: "First tool".to_string(),
                input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
                requires_approval: false,
                ..Default::default()
            },
            Tool {
                name: "tool2".to_string(),
                description: "Second tool".to_string(),
                input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
                requires_approval: true,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].name, "tool1");
        assert_eq!(converted[1].name, "tool2");
    }

    #[test]
    fn test_anthropic_provider_with_empty_api_key() {
        let provider = AnthropicProvider::new("".to_string(), "claude-3-sonnet".to_string());
        assert_eq!(provider.api_key, "");
        assert_eq!(provider.model, "claude-3-sonnet");
    }

    #[test]
    fn test_anthropic_provider_with_special_characters_in_api_key() {
        let api_key = "sk-ant-api03-!@#$%^&*()_+-=[]{}|;':\",./<>?".to_string();
        let provider = AnthropicProvider::new(api_key.clone(), "claude-3-opus".to_string());
        assert_eq!(provider.api_key, api_key);
    }

    #[test]
    fn test_anthropic_provider_with_various_model_names() {
        let models = vec![
            "claude-3-opus-20240229",
            "claude-3-sonnet-20240229",
            "claude-3-haiku-20240307",
            "claude-2.1",
            "claude-2.0",
            "custom-model-123",
        ];

        for model in models {
            let provider = AnthropicProvider::new("test-key".to_string(), model.to_string());
            assert_eq!(provider.model, model);
        }
    }

    #[test]
    fn test_convert_messages_empty_list() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages: Vec<Message> = vec![];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_convert_messages_with_special_characters() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Hello! <>&\"'\n\t\r".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        if let AnthropicContentBlock::Text { text } = &converted[0].content[0] {
            assert!(text.contains("<>&\"'"));
        } else {
            panic!("Expected text block");
        }
    }

    #[test]
    fn test_convert_messages_with_unicode() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("你好世界 🌍 こんにちは".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        if let AnthropicContentBlock::Text { text } = &converted[0].content[0] {
            assert!(text.contains("你好世界"));
            assert!(text.contains("🌍"));
            assert!(text.contains("こんにちは"));
        } else {
            panic!("Expected text block");
        }
    }

    #[test]
    fn test_convert_messages_filters_image_blocks() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text { text: "Look at this".to_string() },
                    ContentBlock::Image {
                        source: brainwires_core::ImageSource::Base64 {
                            media_type: "image/png".to_string(),
                            data: "base64data".to_string(),
                        },
                    },
                    ContentBlock::Text { text: "What do you see?".to_string() },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        // Image block should be filtered out (returns None in filter_map)
        assert_eq!(converted[0].content.len(), 2);
    }

    #[test]
    fn test_convert_messages_only_system_messages() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("System 1".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::System,
                content: MessageContent::Text("System 2".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        // All system messages should be filtered out
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_convert_messages_blocks_with_only_images() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Image {
                        source: brainwires_core::ImageSource::Base64 {
                            media_type: "image/png".to_string(),
                            data: "base64_1".to_string(),
                        },
                    },
                    ContentBlock::Image {
                        source: brainwires_core::ImageSource::Base64 {
                            media_type: "image/jpeg".to_string(),
                            data: "base64_2".to_string(),
                        },
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        // All image blocks should be filtered out
        assert_eq!(converted[0].content.len(), 0);
    }

    #[test]
    fn test_convert_messages_mixed_content_blocks() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text { text: "Let me help".to_string() },
                    ContentBlock::ToolUse {
                        id: "tool-123".to_string(),
                        name: "search".to_string(),
                        input: json!({"query": "test"}),
                    },
                    ContentBlock::Text { text: "Here's what I found".to_string() },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].content.len(), 3);
        assert_eq!(converted[0].role, "assistant");
    }

    #[test]
    fn test_convert_messages_tool_result_with_error() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::ToolResult {
                        tool_use_id: "tool-456".to_string(),
                        content: "Error: File not found".to_string(),
                        is_error: Some(true),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].content.len(), 1);
        if let AnthropicContentBlock::ToolResult { tool_use_id, content } = &converted[0].content[0] {
            assert_eq!(tool_use_id, "tool-456");
            assert!(content.contains("Error"));
        } else {
            panic!("Expected tool result block");
        }
    }

    #[test]
    fn test_convert_tools_with_complex_schema() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let mut properties = HashMap::new();
        properties.insert(
            "nested_object".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "field1": {"type": "string"},
                    "field2": {"type": "number"}
                },
                "required": ["field1"]
            }),
        );
        properties.insert(
            "array_field".to_string(),
            json!({
                "type": "array",
                "items": {"type": "string"}
            }),
        );

        let tools = vec![
            Tool {
                name: "complex_tool".to_string(),
                description: "A tool with complex schema".to_string(),
                input_schema: ToolInputSchema::object(properties.clone(), vec!["nested_object".to_string()]),
                requires_approval: false,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert!(converted[0].input_schema.contains_key("nested_object"));
        assert!(converted[0].input_schema.contains_key("array_field"));
    }

    #[test]
    fn test_convert_tools_with_no_properties() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let tools = vec![
            Tool {
                name: "simple_tool".to_string(),
                description: "A tool with no input".to_string(),
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
        assert_eq!(converted[0].input_schema.len(), 0);
    }

    #[test]
    fn test_convert_tools_with_special_characters_in_names() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let tools = vec![
            Tool {
                name: "tool_with_underscores_123".to_string(),
                description: "Tool with special chars: !@#$%^&*()".to_string(),
                input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
                requires_approval: false,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].name, "tool_with_underscores_123");
        assert!(converted[0].description.contains("!@#$%^&*()"));
    }

    #[test]
    fn test_get_system_message_with_blocks() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text { text: "System instruction".to_string() },
                ]),
                name: None,
                metadata: None,
            },
        ];

        // System messages with Blocks content return None from text() method
        let system = provider.get_system_message(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_get_system_message_multiple_system_messages() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("First system".to_string()),
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
                role: Role::System,
                content: MessageContent::Text("Second system".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let system = provider.get_system_message(&messages);
        // Should return the first system message found
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "First system");
    }

    #[test]
    fn test_get_system_message_empty_messages() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages: Vec<Message> = vec![];

        let system = provider.get_system_message(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_messages_preserves_tool_use_id() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let tool_id = "call_abc123xyz789";
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(vec![
                    ContentBlock::ToolUse {
                        id: tool_id.to_string(),
                        name: "calculate".to_string(),
                        input: json!({"expression": "2+2"}),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        if let AnthropicContentBlock::ToolUse { id, name, input } = &converted[0].content[0] {
            assert_eq!(id, tool_id);
            assert_eq!(name, "calculate");
            assert_eq!(input["expression"], "2+2");
        } else {
            panic!("Expected tool use block");
        }
    }

    #[test]
    fn test_convert_messages_large_text_content() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let large_text = "a".repeat(10000);
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text(large_text.clone()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        if let AnthropicContentBlock::Text { text } = &converted[0].content[0] {
            assert_eq!(text.len(), 10000);
        } else {
            panic!("Expected text block");
        }
    }

    #[test]
    fn test_convert_messages_multiple_tool_uses() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(vec![
                    ContentBlock::ToolUse {
                        id: "tool-1".to_string(),
                        name: "search".to_string(),
                        input: json!({"query": "rust"}),
                    },
                    ContentBlock::ToolUse {
                        id: "tool-2".to_string(),
                        name: "calculate".to_string(),
                        input: json!({"expr": "1+1"}),
                    },
                    ContentBlock::ToolUse {
                        id: "tool-3".to_string(),
                        name: "fetch".to_string(),
                        input: json!({"url": "example.com"}),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].content.len(), 3);
    }

    #[test]
    fn test_convert_messages_alternating_roles() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Q1".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("A1".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::User,
                content: MessageContent::Text("Q2".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("A2".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::User,
                content: MessageContent::Text("Q3".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 5);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "assistant");
        assert_eq!(converted[2].role, "user");
        assert_eq!(converted[3].role, "assistant");
        assert_eq!(converted[4].role, "user");
    }

    #[test]
    fn test_convert_tools_with_empty_description() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let tools = vec![
            Tool {
                name: "minimal_tool".to_string(),
                description: "".to_string(),
                input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
                requires_approval: false,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].description, "");
    }

    #[test]
    fn test_convert_messages_tool_result_empty_content() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::ToolResult {
                        tool_use_id: "tool-999".to_string(),
                        content: "".to_string(),
                        is_error: Some(false),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].content.len(), 1);
        if let AnthropicContentBlock::ToolResult { tool_use_id, content } = &converted[0].content[0] {
            assert_eq!(tool_use_id, "tool-999");
            assert_eq!(content, "");
        } else {
            panic!("Expected tool result block");
        }
    }

    #[test]
    fn test_convert_messages_complex_nested_json() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let complex_input = json!({
            "nested": {
                "level1": {
                    "level2": {
                        "array": [1, 2, 3],
                        "string": "value",
                        "bool": true
                    }
                }
            }
        });

        let messages = vec![
            Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(vec![
                    ContentBlock::ToolUse {
                        id: "complex-tool".to_string(),
                        name: "process".to_string(),
                        input: complex_input.clone(),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        if let AnthropicContentBlock::ToolUse { input, .. } = &converted[0].content[0] {
            assert_eq!(input["nested"]["level1"]["level2"]["array"], json!([1, 2, 3]));
        } else {
            panic!("Expected tool use block with complex input");
        }
    }

    #[test]
    fn test_convert_messages_empty_text_block() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text { text: "".to_string() },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].content.len(), 1);
        if let AnthropicContentBlock::Text { text } = &converted[0].content[0] {
            assert_eq!(text, "");
        } else {
            panic!("Expected empty text block");
        }
    }

    #[test]
    fn test_anthropic_constants() {
        assert_eq!(ANTHROPIC_API_URL, "https://api.anthropic.com/v1/messages");
        assert_eq!(ANTHROPIC_VERSION, "2023-06-01");
    }

    #[test]
    fn test_convert_messages_whitespace_only_text() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("   \n\t\r   ".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        if let AnthropicContentBlock::Text { text } = &converted[0].content[0] {
            assert!(text.contains("\n"));
            assert!(text.contains("\t"));
        } else {
            panic!("Expected text block with whitespace");
        }
    }

    #[test]
    fn test_convert_tools_very_long_description() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let long_desc = "This is a very long description. ".repeat(100);
        let tools = vec![
            Tool {
                name: "verbose_tool".to_string(),
                description: long_desc.clone(),
                input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
                requires_approval: false,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].description.len(), long_desc.len());
    }

    #[test]
    fn test_convert_messages_sequential_tool_results() {
        let provider = AnthropicProvider::new("test-key".to_string(), "claude-3-sonnet".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::ToolResult {
                        tool_use_id: "tool-1".to_string(),
                        content: "Result 1".to_string(),
                        is_error: Some(false),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id: "tool-2".to_string(),
                        content: "Result 2".to_string(),
                        is_error: Some(false),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id: "tool-3".to_string(),
                        content: "Result 3".to_string(),
                        is_error: Some(false),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].content.len(), 3);
    }
}
