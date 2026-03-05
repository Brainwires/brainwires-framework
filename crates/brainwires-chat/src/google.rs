//! Chat-layer wrapper around [`GoogleClient`] that implements the
//! [`Provider`] trait from `brainwires_core`.
//!
//! All Gemini-specific type conversions live here so that the low-level client
//! crate (`brainwires-providers`) stays free of any `brainwires_core` domain
//! types.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use serde_json::json;
use uuid::Uuid;

use brainwires_core::{
    ChatOptions, ChatResponse, ContentBlock, ImageSource, Message, MessageContent, Role,
    StreamChunk, Tool, Usage,
};
use brainwires_core::Provider;
use brainwires_providers::google::{
    GeminiFunctionCall, GeminiFunctionDeclaration, GeminiFunctionResponse, GeminiGenerationConfig,
    GeminiInlineData, GeminiMessage, GeminiPart, GeminiRequest, GeminiStreamChunk,
    GeminiSystemInstruction, GeminiToolSet, GoogleClient,
};

/// High-level Google Gemini chat provider.
///
/// Wraps a shared [`GoogleClient`] and implements `brainwires_core::Provider`,
/// converting between core domain types and Gemini wire types.
pub struct GoogleChatProvider {
    client: Arc<GoogleClient>,
    model: String,
}

impl GoogleChatProvider {
    /// Create a new chat provider from an existing client and model name.
    pub fn new(client: Arc<GoogleClient>, model: String) -> Self {
        Self { client, model }
    }

    // -----------------------------------------------------------------
    // Conversion helpers  (core types -> Gemini wire types)
    // -----------------------------------------------------------------

    /// Convert core [`Message`] slices to Gemini's format.
    fn convert_messages(messages: &[Message]) -> Vec<GeminiMessage> {
        messages
            .iter()
            .filter(|m| m.role != Role::System) // System goes in separate field
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "model",
                    _ => "user",
                };

                let parts = match &m.content {
                    MessageContent::Text(text) => vec![GeminiPart::Text {
                        text: text.clone(),
                    }],
                    MessageContent::Blocks(blocks) => blocks
                        .iter()
                        .filter_map(Self::convert_content_block)
                        .collect(),
                };

                GeminiMessage {
                    role: role.to_string(),
                    parts,
                }
            })
            .collect()
    }

    fn convert_content_block(block: &ContentBlock) -> Option<GeminiPart> {
        match block {
            ContentBlock::Text { text } => Some(GeminiPart::Text {
                text: text.clone(),
            }),
            ContentBlock::Image { source } => match source {
                ImageSource::Base64 { media_type, data } => Some(GeminiPart::InlineData {
                    inline_data: GeminiInlineData {
                        mime_type: media_type.clone(),
                        data: data.clone(),
                    },
                }),
            },
            ContentBlock::ToolUse {
                id: _id,
                name,
                input,
            } => Some(GeminiPart::FunctionCall {
                function_call: GeminiFunctionCall {
                    name: name.clone(),
                    args: input.clone(),
                },
            }),
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                ..
            } => Some(GeminiPart::FunctionResponse {
                function_response: GeminiFunctionResponse {
                    name: tool_use_id.clone(),
                    response: json!({ "result": content }),
                },
            }),
        }
    }

    /// Convert core [`Tool`] slices to Gemini function declarations.
    fn convert_tools(tools: &[Tool]) -> Vec<GeminiFunctionDeclaration> {
        tools
            .iter()
            .map(|t| GeminiFunctionDeclaration {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.input_schema.properties.clone().unwrap_or_default(),
            })
            .collect()
    }

    /// Extract a system instruction from the message list.
    fn get_system_instruction(messages: &[Message]) -> Option<String> {
        messages
            .iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| m.text().map(|s| s.to_string()))
    }

    /// Build a [`GeminiRequest`] from core domain types.
    fn build_request(
        messages: &[Message],
        tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> GeminiRequest {
        let contents = Self::convert_messages(messages);

        let system_text = options
            .system
            .clone()
            .or_else(|| Self::get_system_instruction(messages));

        let system_instruction = system_text.map(|text| GeminiSystemInstruction {
            parts: vec![GeminiPart::Text { text }],
        });

        // Generation config
        let generation_config = {
            let has_any =
                options.temperature.is_some() || options.max_tokens.is_some() || options.top_p.is_some();
            if has_any {
                Some(GeminiGenerationConfig {
                    temperature: options.temperature,
                    max_output_tokens: options.max_tokens,
                    top_p: options.top_p,
                })
            } else {
                None
            }
        };

        // Tools
        let gemini_tools = match tools {
            Some(tools_list) if !tools_list.is_empty() => {
                Some(vec![GeminiToolSet {
                    function_declarations: Self::convert_tools(tools_list),
                }])
            }
            _ => None,
        };

        GeminiRequest {
            contents,
            system_instruction,
            generation_config,
            tools: gemini_tools,
        }
    }

    // -----------------------------------------------------------------
    // Response conversion  (Gemini wire types -> core types)
    // -----------------------------------------------------------------

    /// Convert a Gemini candidate's parts into a core [`MessageContent`].
    fn convert_candidate_content(parts: Vec<GeminiPart>) -> MessageContent {
        if parts.len() == 1 {
            if let GeminiPart::Text { ref text } = parts[0] {
                return MessageContent::Text(text.clone());
            }
        }

        MessageContent::Blocks(
            parts
                .into_iter()
                .filter_map(|part| match part {
                    GeminiPart::Text { text } => Some(ContentBlock::Text { text }),
                    GeminiPart::FunctionCall { function_call } => Some(ContentBlock::ToolUse {
                        id: Uuid::new_v4().to_string(),
                        name: function_call.name,
                        input: function_call.args,
                    }),
                    _ => None,
                })
                .collect(),
        )
    }
}

#[async_trait]
impl Provider for GoogleChatProvider {
    fn name(&self) -> &str {
        "google"
    }

    #[tracing::instrument(name = "provider.chat", skip_all, fields(provider = "google", model = %self.model))]
    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        let request = Self::build_request(messages, tools, options);
        let gemini_response = self.client.generate_content(&request).await?;

        let candidate = gemini_response
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No candidates in Gemini response"))?;

        let content = Self::convert_candidate_content(candidate.content.parts);

        let usage = gemini_response
            .usage_metadata
            .map(|u| Usage {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.total_token_count,
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            message: Message {
                role: Role::Assistant,
                content,
                name: None,
                metadata: None,
            },
            usage,
            finish_reason: Some(candidate.finish_reason),
        })
    }

    fn stream_chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: Option<&'a [Tool]>,
        options: &'a ChatOptions,
    ) -> BoxStream<'a, Result<StreamChunk>> {
        tracing::info!(provider = "google", model = %self.model, "provider.stream started");
        Box::pin(async_stream::stream! {
            let request = Self::build_request(messages, tools, options);
            let mut stream = self.client.stream_generate_content(&request);

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Some(candidate) = chunk.candidates.into_iter().next() {
                            for part in candidate.content.parts {
                                match part {
                                    GeminiPart::Text { text } => {
                                        yield Ok(StreamChunk::Text(text));
                                    }
                                    GeminiPart::FunctionCall { function_call } => {
                                        yield Ok(StreamChunk::ToolUse {
                                            id: Uuid::new_v4().to_string(),
                                            name: function_call.name,
                                        });
                                    }
                                    _ => {}
                                }
                            }

                            // Check if this is the last chunk
                            if candidate.finish_reason != "STOP"
                                && !candidate.finish_reason.is_empty()
                            {
                                yield Ok(StreamChunk::Done);
                            }
                        }

                        if let Some(usage) = chunk.usage_metadata {
                            yield Ok(StreamChunk::Usage(Usage {
                                prompt_tokens: usage.prompt_token_count,
                                completion_tokens: usage.candidates_token_count,
                                total_tokens: usage.total_token_count,
                            }));
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                    }
                }
            }

            yield Ok(StreamChunk::Done);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires_core::ToolInputSchema;
    use std::collections::HashMap;

    // Helper to create a GoogleChatProvider for testing conversion methods.
    // We only test the pure conversion functions here; the actual HTTP calls
    // are integration-tested elsewhere.

    #[test]
    fn test_convert_messages_text() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Text("Hello".to_string()),
            name: None,
            metadata: None,
        }];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
    }

    #[test]
    fn test_convert_messages_filters_system() {
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

        let converted = GoogleChatProvider::convert_messages(&messages);
        // System message should be filtered out
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
    }

    #[test]
    fn test_get_system_instruction_found() {
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

        let system = GoogleChatProvider::get_system_instruction(&messages);
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "You are helpful");
    }

    #[test]
    fn test_get_system_instruction_not_found() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Text("Hello".to_string()),
            name: None,
            metadata: None,
        }];

        let system = GoogleChatProvider::get_system_instruction(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_tools() {
        let mut properties = HashMap::new();
        properties.insert(
            "arg1".to_string(),
            json!({
                "type": "string",
                "description": "First argument"
            }),
        );

        let tools = vec![Tool {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: ToolInputSchema::object(properties.clone(), vec!["arg1".to_string()]),
            requires_approval: false,
            ..Default::default()
        }];

        let converted = GoogleChatProvider::convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].name, "test_tool");
        assert_eq!(converted[0].description, "A test tool");
    }

    #[test]
    fn test_convert_tools_empty() {
        let tools: Vec<Tool> = vec![];

        let converted = GoogleChatProvider::convert_tools(&tools);
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_convert_messages_assistant_role() {
        let messages = vec![Message {
            role: Role::Assistant,
            content: MessageContent::Text("I'm an assistant".to_string()),
            name: None,
            metadata: None,
        }];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "model");
    }

    #[test]
    fn test_convert_messages_with_blocks() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Blocks(vec![
                ContentBlock::Text {
                    text: "First block".to_string(),
                },
                ContentBlock::Text {
                    text: "Second block".to_string(),
                },
            ]),
            name: None,
            metadata: None,
        }];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[0].parts.len(), 2);
    }

    #[test]
    fn test_convert_messages_multiple_messages() {
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("First".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("Second".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::User,
                content: MessageContent::Text("Third".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "model");
        assert_eq!(converted[2].role, "user");
    }

    #[test]
    fn test_convert_content_block_text() {
        let block = ContentBlock::Text {
            text: "Test text".to_string(),
        };

        let converted = GoogleChatProvider::convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::Text { text } => assert_eq!(text, "Test text"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_convert_content_block_image() {
        let block = ContentBlock::Image {
            source: ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "base64data".to_string(),
            },
        };

        let converted = GoogleChatProvider::convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::InlineData { inline_data } => {
                assert_eq!(inline_data.mime_type, "image/png");
                assert_eq!(inline_data.data, "base64data");
            }
            _ => panic!("Expected InlineData variant"),
        }
    }

    #[test]
    fn test_convert_content_block_tool_use() {
        let block = ContentBlock::ToolUse {
            id: "tool-123".to_string(),
            name: "test_tool".to_string(),
            input: json!({"arg": "value"}),
        };

        let converted = GoogleChatProvider::convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::FunctionCall { function_call } => {
                assert_eq!(function_call.name, "test_tool");
                assert_eq!(function_call.args, json!({"arg": "value"}));
            }
            _ => panic!("Expected FunctionCall variant"),
        }
    }

    #[test]
    fn test_convert_content_block_tool_result() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "tool-123".to_string(),
            content: "Result content".to_string(),
            is_error: Some(false),
        };

        let converted = GoogleChatProvider::convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::FunctionResponse { function_response } => {
                assert_eq!(function_response.name, "tool-123");
                assert_eq!(
                    function_response.response,
                    json!({"result": "Result content"})
                );
            }
            _ => panic!("Expected FunctionResponse variant"),
        }
    }

    #[test]
    fn test_convert_tools_multiple() {
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

        let converted = GoogleChatProvider::convert_tools(&tools);
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].name, "tool1");
        assert_eq!(converted[1].name, "tool2");
    }

    #[test]
    fn test_convert_tools_no_properties() {
        let tools = vec![Tool {
            name: "simple_tool".to_string(),
            description: "No args".to_string(),
            input_schema: ToolInputSchema {
                schema_type: "object".to_string(),
                properties: None,
                required: None,
            },
            requires_approval: false,
            ..Default::default()
        }];

        let converted = GoogleChatProvider::convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].name, "simple_tool");
        assert!(converted[0].parameters.is_empty());
    }

    #[test]
    fn test_get_system_instruction_multiple_system_messages() {
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("First system".to_string()),
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

        // Should return the first system message found
        let system = GoogleChatProvider::get_system_instruction(&messages);
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "First system");
    }

    #[test]
    fn test_get_system_instruction_with_blocks() {
        let messages = vec![Message {
            role: Role::System,
            content: MessageContent::Blocks(vec![ContentBlock::Text {
                text: "System from blocks".to_string(),
            }]),
            name: None,
            metadata: None,
        }];

        // get_system_instruction uses Message::text() which returns None for Blocks
        let system = GoogleChatProvider::get_system_instruction(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_messages_empty() {
        let messages: Vec<Message> = vec![];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_convert_messages_only_system() {
        let messages = vec![Message {
            role: Role::System,
            content: MessageContent::Text("Only system".to_string()),
            name: None,
            metadata: None,
        }];

        let converted = GoogleChatProvider::convert_messages(&messages);
        // System messages are filtered out
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_convert_messages_mixed_content_types() {
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Text message".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(vec![ContentBlock::Text {
                    text: "Block message".to_string(),
                }]),
                name: None,
                metadata: None,
            },
        ];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].parts.len(), 1);
        assert_eq!(converted[1].parts.len(), 1);
    }

    #[test]
    fn test_convert_content_block_tool_use_empty_input() {
        let block = ContentBlock::ToolUse {
            id: "tool-456".to_string(),
            name: "empty_tool".to_string(),
            input: json!({}),
        };

        let converted = GoogleChatProvider::convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::FunctionCall { function_call } => {
                assert_eq!(function_call.name, "empty_tool");
                assert_eq!(function_call.args, json!({}));
            }
            _ => panic!("Expected FunctionCall variant"),
        }
    }

    #[test]
    fn test_convert_content_block_tool_result_error() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "tool-error".to_string(),
            content: "Error occurred".to_string(),
            is_error: Some(true),
        };

        let converted = GoogleChatProvider::convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::FunctionResponse { function_response } => {
                assert_eq!(function_response.name, "tool-error");
                assert_eq!(
                    function_response.response,
                    json!({"result": "Error occurred"})
                );
            }
            _ => panic!("Expected FunctionResponse variant"),
        }
    }

    #[test]
    fn test_convert_messages_with_tool_blocks() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Blocks(vec![
                ContentBlock::Text {
                    text: "Use a tool".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "calculator".to_string(),
                    input: json!({"operation": "add", "a": 1, "b": 2}),
                },
            ]),
            name: None,
            metadata: None,
        }];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].parts.len(), 2);
    }

    #[test]
    fn test_convert_messages_with_image_blocks() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Blocks(vec![
                ContentBlock::Text {
                    text: "What's in this image?".to_string(),
                },
                ContentBlock::Image {
                    source: ImageSource::Base64 {
                        media_type: "image/jpeg".to_string(),
                        data: "fake_jpeg_data".to_string(),
                    },
                },
            ]),
            name: None,
            metadata: None,
        }];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].parts.len(), 2);
    }

    #[test]
    fn test_convert_tools_complex_schema() {
        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            json!({
                "type": "string",
                "description": "User name"
            }),
        );
        properties.insert(
            "age".to_string(),
            json!({
                "type": "integer",
                "description": "User age",
                "minimum": 0
            }),
        );
        properties.insert(
            "tags".to_string(),
            json!({
                "type": "array",
                "items": {"type": "string"}
            }),
        );

        let tools = vec![Tool {
            name: "create_user".to_string(),
            description: "Creates a new user".to_string(),
            input_schema: ToolInputSchema::object(
                properties.clone(),
                vec!["name".to_string(), "age".to_string()],
            ),
            requires_approval: false,
            ..Default::default()
        }];

        let converted = GoogleChatProvider::convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].name, "create_user");
        assert_eq!(converted[0].parameters.len(), 3);
        assert!(converted[0].parameters.contains_key("name"));
        assert!(converted[0].parameters.contains_key("age"));
        assert!(converted[0].parameters.contains_key("tags"));
    }

    #[test]
    fn test_get_system_instruction_empty_content() {
        let messages = vec![Message {
            role: Role::System,
            content: MessageContent::Text("".to_string()),
            name: None,
            metadata: None,
        }];

        let system = GoogleChatProvider::get_system_instruction(&messages);
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "");
    }

    #[test]
    fn test_convert_messages_preserves_order() {
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("First".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("Second".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::User,
                content: MessageContent::Text("Third".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("Fourth".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 4);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "model");
        assert_eq!(converted[2].role, "user");
        assert_eq!(converted[3].role, "model");
    }

    #[test]
    fn test_convert_content_block_image_different_mime_types() {
        let block_png = ContentBlock::Image {
            source: ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "png_data".to_string(),
            },
        };

        let block_jpeg = ContentBlock::Image {
            source: ImageSource::Base64 {
                media_type: "image/jpeg".to_string(),
                data: "jpeg_data".to_string(),
            },
        };

        let converted_png = GoogleChatProvider::convert_content_block(&block_png);
        let converted_jpeg = GoogleChatProvider::convert_content_block(&block_jpeg);

        assert!(converted_png.is_some());
        assert!(converted_jpeg.is_some());

        match converted_png.unwrap() {
            GeminiPart::InlineData { inline_data } => {
                assert_eq!(inline_data.mime_type, "image/png");
            }
            _ => panic!("Expected InlineData variant"),
        }

        match converted_jpeg.unwrap() {
            GeminiPart::InlineData { inline_data } => {
                assert_eq!(inline_data.mime_type, "image/jpeg");
            }
            _ => panic!("Expected InlineData variant"),
        }
    }

    #[test]
    fn test_convert_messages_with_all_content_block_types() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Blocks(vec![
                ContentBlock::Text {
                    text: "Text part".to_string(),
                },
                ContentBlock::Image {
                    source: ImageSource::Base64 {
                        media_type: "image/png".to_string(),
                        data: "img_data".to_string(),
                    },
                },
                ContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "test_tool".to_string(),
                    input: json!({"key": "value"}),
                },
                ContentBlock::ToolResult {
                    tool_use_id: "tool-1".to_string(),
                    content: "Tool output".to_string(),
                    is_error: Some(false),
                },
            ]),
            name: None,
            metadata: None,
        }];

        let converted = GoogleChatProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].parts.len(), 4);
    }

    #[test]
    fn test_build_request_minimal() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Text("Hello".to_string()),
            name: None,
            metadata: None,
        }];
        let options = ChatOptions {
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            system: None,
        };

        let req = GoogleChatProvider::build_request(&messages, None, &options);
        assert_eq!(req.contents.len(), 1);
        assert!(req.system_instruction.is_none());
        assert!(req.generation_config.is_none());
        assert!(req.tools.is_none());
    }

    #[test]
    fn test_build_request_with_system() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Text("Hello".to_string()),
            name: None,
            metadata: None,
        }];
        let options = ChatOptions {
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            system: Some("Be helpful".to_string()),
        };

        let req = GoogleChatProvider::build_request(&messages, None, &options);
        assert!(req.system_instruction.is_some());
    }

    #[test]
    fn test_build_request_with_generation_config() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Text("Hello".to_string()),
            name: None,
            metadata: None,
        }];
        let options = ChatOptions {
            temperature: Some(0.5),
            max_tokens: Some(1024),
            top_p: None,
            stop: None,
            system: None,
        };

        let req = GoogleChatProvider::build_request(&messages, None, &options);
        assert!(req.generation_config.is_some());
        let gc = req.generation_config.unwrap();
        assert_eq!(gc.temperature, Some(0.5));
        assert_eq!(gc.max_output_tokens, Some(1024));
    }

    #[test]
    fn test_build_request_with_tools() {
        let messages = vec![Message {
            role: Role::User,
            content: MessageContent::Text("Hello".to_string()),
            name: None,
            metadata: None,
        }];
        let options = ChatOptions::default();
        let mut properties = HashMap::new();
        properties.insert("x".to_string(), json!({"type": "string"}));
        let tools = vec![Tool {
            name: "my_tool".to_string(),
            description: "desc".to_string(),
            input_schema: ToolInputSchema::object(properties, vec![]),
            requires_approval: false,
            ..Default::default()
        }];

        let req = GoogleChatProvider::build_request(&messages, Some(&tools), &options);
        assert!(req.tools.is_some());
        assert_eq!(req.tools.unwrap()[0].function_declarations.len(), 1);
    }

    #[test]
    fn test_convert_candidate_content_single_text() {
        let parts = vec![GeminiPart::Text {
            text: "Hello world".to_string(),
        }];
        let content = GoogleChatProvider::convert_candidate_content(parts);
        match content {
            MessageContent::Text(t) => assert_eq!(t, "Hello world"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_convert_candidate_content_multiple_parts() {
        let parts = vec![
            GeminiPart::Text {
                text: "Part 1".to_string(),
            },
            GeminiPart::Text {
                text: "Part 2".to_string(),
            },
        ];
        let content = GoogleChatProvider::convert_candidate_content(parts);
        match content {
            MessageContent::Blocks(blocks) => assert_eq!(blocks.len(), 2),
            _ => panic!("Expected Blocks variant"),
        }
    }

    #[test]
    fn test_convert_candidate_content_with_function_call() {
        let parts = vec![GeminiPart::FunctionCall {
            function_call: GeminiFunctionCall {
                name: "do_thing".to_string(),
                args: json!({"a": 1}),
            },
        }];
        let content = GoogleChatProvider::convert_candidate_content(parts);
        match content {
            MessageContent::Blocks(blocks) => {
                assert_eq!(blocks.len(), 1);
                match &blocks[0] {
                    ContentBlock::ToolUse { name, input, .. } => {
                        assert_eq!(name, "do_thing");
                        assert_eq!(*input, json!({"a": 1}));
                    }
                    _ => panic!("Expected ToolUse block"),
                }
            }
            _ => panic!("Expected Blocks variant"),
        }
    }
}
