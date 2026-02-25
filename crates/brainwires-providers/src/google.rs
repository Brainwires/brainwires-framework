use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use brainwires_core::{ChatResponse, ContentBlock, Message, MessageContent, Role, StreamChunk, Usage};
use brainwires_core::{ChatOptions, Provider};
use brainwires_core::Tool;

use super::rate_limiter::RateLimiter;

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

pub struct GoogleProvider {
    api_key: String,
    model: String,
    http_client: Client,
    rate_limiter: Option<std::sync::Arc<RateLimiter>>,
}

impl GoogleProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http_client: Client::new(),
            rate_limiter: None,
        }
    }

    /// Create a provider with rate limiting (requests per minute).
    pub fn with_rate_limit(api_key: String, model: String, requests_per_minute: u32) -> Self {
        Self {
            api_key,
            model,
            http_client: Client::new(),
            rate_limiter: Some(std::sync::Arc::new(RateLimiter::new(requests_per_minute))),
        }
    }

    /// Wait for rate-limit clearance (no-op if not configured).
    async fn acquire_rate_limit(&self) {
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }
    }

    /// Convert our Message format to Gemini's format
    fn convert_messages(&self, messages: &[Message]) -> Vec<GeminiMessage> {
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
                        .filter_map(|b| self.convert_content_block(b))
                        .collect(),
                };

                GeminiMessage {
                    role: role.to_string(),
                    parts,
                }
            })
            .collect()
    }

    fn convert_content_block(&self, block: &ContentBlock) -> Option<GeminiPart> {
        match block {
            ContentBlock::Text { text } => Some(GeminiPart::Text {
                text: text.clone(),
            }),
            ContentBlock::Image { source } => {
                match source {
                    brainwires_core::ImageSource::Base64 { media_type, data } => {
                        Some(GeminiPart::InlineData {
                            inline_data: GeminiInlineData {
                                mime_type: media_type.clone(),
                                data: data.clone(),
                            },
                        })
                    }
                }
            }
            ContentBlock::ToolUse { id: _id, name, input } => Some(GeminiPart::FunctionCall {
                function_call: GeminiFunctionCall {
                    name: name.clone(),
                    args: input.clone(),
                },
            }),
            ContentBlock::ToolResult { tool_use_id, content, .. } => {
                Some(GeminiPart::FunctionResponse {
                    function_response: GeminiFunctionResponse {
                        name: tool_use_id.clone(),
                        response: json!({ "result": content }),
                    },
                })
            }
        }
    }

    /// Convert our Tool format to Gemini's format
    fn convert_tools(&self, tools: &[Tool]) -> Vec<GeminiFunctionDeclaration> {
        tools
            .iter()
            .map(|t| GeminiFunctionDeclaration {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.input_schema.properties.clone().unwrap_or_default(),
            })
            .collect()
    }

    /// Get system instruction from messages
    fn get_system_instruction(&self, messages: &[Message]) -> Option<String> {
        messages
            .iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| m.text().map(|s| s.to_string()))
    }
}

#[async_trait]
impl Provider for GoogleProvider {
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
        let gemini_messages = self.convert_messages(messages);
        let system_instruction = options
            .system
            .clone()
            .or_else(|| self.get_system_instruction(messages));

        let mut request_body = json!({
            "contents": gemini_messages,
        });

        if let Some(sys) = system_instruction {
            request_body["system_instruction"] = json!({
                "parts": [{ "text": sys }]
            });
        }

        // Generation config
        let mut generation_config = json!({});
        if let Some(temp) = options.temperature {
            generation_config["temperature"] = json!(temp);
        }
        if let Some(max_tokens) = options.max_tokens {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }
        if let Some(top_p) = options.top_p {
            generation_config["topP"] = json!(top_p);
        }
        if !generation_config.as_object().unwrap().is_empty() {
            request_body["generationConfig"] = generation_config;
        }

        // Tools
        if let Some(tools_list) = tools {
            if !tools_list.is_empty() {
                request_body["tools"] = json!([{
                    "function_declarations": self.convert_tools(tools_list)
                }]);
            }
        }

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            GEMINI_API_BASE, self.model, self.api_key
        );

        self.acquire_rate_limit().await;
        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Google Gemini")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Google Gemini API error ({}): {}", status, error_text);
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Google Gemini response")?;

        let candidate = gemini_response
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No candidates in Gemini response"))?;

        // Convert response to our format
        let content = if candidate.content.parts.len() == 1 {
            match &candidate.content.parts[0] {
                GeminiPart::Text { text } => MessageContent::Text(text.clone()),
                _ => MessageContent::Blocks(
                    candidate
                        .content
                        .parts
                        .into_iter()
                        .filter_map(|part| match part {
                            GeminiPart::Text { text } => Some(ContentBlock::Text { text }),
                            GeminiPart::FunctionCall { function_call } => {
                                Some(ContentBlock::ToolUse {
                                    id: Uuid::new_v4().to_string(),
                                    name: function_call.name,
                                    input: function_call.args,
                                })
                            }
                            _ => None,
                        })
                        .collect(),
                ),
            }
        } else {
            MessageContent::Blocks(
                candidate
                    .content
                    .parts
                    .into_iter()
                    .filter_map(|part| match part {
                        GeminiPart::Text { text } => Some(ContentBlock::Text { text }),
                        GeminiPart::FunctionCall { function_call } => {
                            Some(ContentBlock::ToolUse {
                                id: Uuid::new_v4().to_string(),
                                name: function_call.name,
                                input: function_call.args,
                            })
                        }
                        _ => None,
                    })
                    .collect(),
            )
        };

        let usage = gemini_response.usage_metadata.map(|u| Usage {
            prompt_tokens: u.prompt_token_count,
            completion_tokens: u.candidates_token_count,
            total_tokens: u.total_token_count,
        }).unwrap_or_default();

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
            let gemini_messages = self.convert_messages(messages);
            let system_instruction = options
                .system
                .clone()
                .or_else(|| self.get_system_instruction(messages));

            let mut request_body = json!({
                "contents": gemini_messages,
            });

            if let Some(sys) = system_instruction {
                request_body["system_instruction"] = json!({
                    "parts": [{ "text": sys }]
                });
            }

            // Generation config
            let mut generation_config = json!({});
            if let Some(temp) = options.temperature {
                generation_config["temperature"] = json!(temp);
            }
            if let Some(max_tokens) = options.max_tokens {
                generation_config["maxOutputTokens"] = json!(max_tokens);
            }
            if let Some(top_p) = options.top_p {
                generation_config["topP"] = json!(top_p);
            }
            if !generation_config.as_object().unwrap().is_empty() {
                request_body["generationConfig"] = generation_config;
            }

            // Tools
            if let Some(tools_list) = tools {
                if !tools_list.is_empty() {
                    request_body["tools"] = json!([{
                        "function_declarations": self.convert_tools(tools_list)
                    }]);
                }
            }

            let url = format!(
                "{}/models/{}:streamGenerateContent?key={}",
                GEMINI_API_BASE, self.model, self.api_key
            );

            self.acquire_rate_limit().await;
            let response = match self
                .http_client
                .post(&url)
                .header("Content-Type", "application/json")
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
                yield Err(anyhow::anyhow!("Google Gemini API error ({}): {}", status, error_text));
                return;
            }

            // Parse streaming response (newline-delimited JSON)
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

                // Process complete JSON objects (delimited by newlines)
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() {
                        continue;
                    }

                    // Parse the JSON line
                    match serde_json::from_str::<GeminiStreamChunk>(&line) {
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
                                if candidate.finish_reason != "STOP" && !candidate.finish_reason.is_empty() {
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
                            tracing::warn!("Failed to parse Gemini stream chunk: {}", e);
                        }
                    }
                }
            }

            yield Ok(StreamChunk::Done);
        })
    }
}

// Gemini API types

#[derive(Debug, Serialize)]
struct GeminiMessage {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum GeminiPart {
    Text {
        text: String,
    },
    InlineData {
        inline_data: GeminiInlineData,
    },
    FunctionCall {
        function_call: GeminiFunctionCall,
    },
    FunctionResponse {
        function_response: GeminiFunctionResponse,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GeminiInlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(rename = "finishReason")]
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiUsageMetadata {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: u32,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: u32,
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiStreamChunk {
    candidates: Vec<GeminiCandidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires_core::ToolInputSchema;
    use std::collections::HashMap;

    #[test]
    fn test_google_provider_new() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "gemini-pro");
    }

    #[test]
    fn test_provider_name() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        assert_eq!(provider.name(), "google");
    }

    #[test]
    fn test_convert_messages_text() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
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
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
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
    fn test_get_system_instruction_found() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
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

        let system = provider.get_system_instruction(&messages);
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "You are helpful");
    }

    #[test]
    fn test_get_system_instruction_not_found() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Hello".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let system = provider.get_system_instruction(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_tools() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
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
    }

    #[test]
    fn test_convert_tools_empty() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let tools: Vec<Tool> = vec![];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_google_provider_new_empty_api_key() {
        let provider = GoogleProvider::new("".to_string(), "gemini-pro".to_string());
        assert_eq!(provider.api_key, "");
        assert_eq!(provider.model, "gemini-pro");
    }

    #[test]
    fn test_google_provider_new_empty_model() {
        let provider = GoogleProvider::new("test-key".to_string(), "".to_string());
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "");
    }

    #[test]
    fn test_google_provider_new_special_chars() {
        let provider = GoogleProvider::new(
            "key-with-special-!@#$%".to_string(),
            "model-1.5-pro".to_string()
        );
        assert_eq!(provider.api_key, "key-with-special-!@#$%");
        assert_eq!(provider.model, "model-1.5-pro");
    }

    #[test]
    fn test_convert_messages_assistant_role() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
                role: Role::Assistant,
                content: MessageContent::Text("I'm an assistant".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "model");
    }

    #[test]
    fn test_convert_messages_with_blocks() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
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
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[0].parts.len(), 2);
    }

    #[test]
    fn test_convert_messages_multiple_messages() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
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

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "model");
        assert_eq!(converted[2].role, "user");
    }

    #[test]
    fn test_convert_content_block_text() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let block = ContentBlock::Text {
            text: "Test text".to_string(),
        };

        let converted = provider.convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::Text { text } => assert_eq!(text, "Test text"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_convert_content_block_image() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let block = ContentBlock::Image {
            source: brainwires_core::ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "base64data".to_string(),
            },
        };

        let converted = provider.convert_content_block(&block);
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
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let block = ContentBlock::ToolUse {
            id: "tool-123".to_string(),
            name: "test_tool".to_string(),
            input: json!({"arg": "value"}),
        };

        let converted = provider.convert_content_block(&block);
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
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let block = ContentBlock::ToolResult {
            tool_use_id: "tool-123".to_string(),
            content: "Result content".to_string(),
            is_error: Some(false),
        };

        let converted = provider.convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::FunctionResponse { function_response } => {
                assert_eq!(function_response.name, "tool-123");
                assert_eq!(function_response.response, json!({"result": "Result content"}));
            }
            _ => panic!("Expected FunctionResponse variant"),
        }
    }

    #[test]
    fn test_convert_tools_multiple() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());

        let mut properties1 = HashMap::new();
        properties1.insert(
            "arg1".to_string(),
            json!({"type": "string"}),
        );

        let mut properties2 = HashMap::new();
        properties2.insert(
            "arg2".to_string(),
            json!({"type": "number"}),
        );

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
        assert_eq!(converted[0].name, "tool1");
        assert_eq!(converted[1].name, "tool2");
    }

    #[test]
    fn test_convert_tools_no_properties() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());

        let tools = vec![
            Tool {
                name: "simple_tool".to_string(),
                description: "No args".to_string(),
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
        assert_eq!(converted[0].name, "simple_tool");
        assert!(converted[0].parameters.is_empty());
    }

    #[test]
    fn test_get_system_instruction_multiple_system_messages() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
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
        let system = provider.get_system_instruction(&messages);
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "First system");
    }

    #[test]
    fn test_get_system_instruction_with_blocks() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text {
                        text: "System from blocks".to_string(),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        // get_system_instruction uses Message::text() which returns None for Blocks
        let system = provider.get_system_instruction(&messages);
        assert!(system.is_none());
    }

    #[test]
    fn test_convert_messages_empty() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages: Vec<Message> = vec![];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_convert_messages_only_system() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("Only system".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        // System messages are filtered out
        assert_eq!(converted.len(), 0);
    }

    #[test]
    fn test_convert_messages_mixed_content_types() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Text message".to_string()),
                name: None,
                metadata: None,
            },
            Message {
                role: Role::Assistant,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text {
                        text: "Block message".to_string(),
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].parts.len(), 1);
        assert_eq!(converted[1].parts.len(), 1);
    }

    #[test]
    fn test_convert_content_block_tool_use_empty_input() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let block = ContentBlock::ToolUse {
            id: "tool-456".to_string(),
            name: "empty_tool".to_string(),
            input: json!({}),
        };

        let converted = provider.convert_content_block(&block);
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
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let block = ContentBlock::ToolResult {
            tool_use_id: "tool-error".to_string(),
            content: "Error occurred".to_string(),
            is_error: Some(true),
        };

        let converted = provider.convert_content_block(&block);
        assert!(converted.is_some());
        match converted.unwrap() {
            GeminiPart::FunctionResponse { function_response } => {
                assert_eq!(function_response.name, "tool-error");
                assert_eq!(function_response.response, json!({"result": "Error occurred"}));
            }
            _ => panic!("Expected FunctionResponse variant"),
        }
    }

    #[test]
    fn test_convert_messages_with_tool_blocks() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
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
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].parts.len(), 2);
    }

    #[test]
    fn test_convert_messages_with_image_blocks() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text {
                        text: "What's in this image?".to_string(),
                    },
                    ContentBlock::Image {
                        source: brainwires_core::ImageSource::Base64 {
                            media_type: "image/jpeg".to_string(),
                            data: "fake_jpeg_data".to_string(),
                        },
                    },
                ]),
                name: None,
                metadata: None,
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].parts.len(), 2);
    }

    #[test]
    fn test_convert_tools_complex_schema() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());

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

        let tools = vec![
            Tool {
                name: "create_user".to_string(),
                description: "Creates a new user".to_string(),
                input_schema: ToolInputSchema::object(
                    properties.clone(),
                    vec!["name".to_string(), "age".to_string()]
                ),
                requires_approval: false,
                ..Default::default()
            },
        ];

        let converted = provider.convert_tools(&tools);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].name, "create_user");
        assert_eq!(converted[0].parameters.len(), 3);
        assert!(converted[0].parameters.contains_key("name"));
        assert!(converted[0].parameters.contains_key("age"));
        assert!(converted[0].parameters.contains_key("tags"));
    }

    #[test]
    fn test_get_system_instruction_empty_content() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
                role: Role::System,
                content: MessageContent::Text("".to_string()),
                name: None,
                metadata: None,
            },
        ];

        let system = provider.get_system_instruction(&messages);
        assert!(system.is_some());
        assert_eq!(system.unwrap(), "");
    }

    #[test]
    fn test_convert_messages_preserves_order() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
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

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 4);
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[1].role, "model");
        assert_eq!(converted[2].role, "user");
        assert_eq!(converted[3].role, "model");
    }

    #[test]
    fn test_convert_content_block_image_different_mime_types() {
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());

        let block_png = ContentBlock::Image {
            source: brainwires_core::ImageSource::Base64 {
                media_type: "image/png".to_string(),
                data: "png_data".to_string(),
            },
        };

        let block_jpeg = ContentBlock::Image {
            source: brainwires_core::ImageSource::Base64 {
                media_type: "image/jpeg".to_string(),
                data: "jpeg_data".to_string(),
            },
        };

        let converted_png = provider.convert_content_block(&block_png);
        let converted_jpeg = provider.convert_content_block(&block_jpeg);

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
        let provider = GoogleProvider::new("test-key".to_string(), "gemini-pro".to_string());
        let messages = vec![
            Message {
                role: Role::User,
                content: MessageContent::Blocks(vec![
                    ContentBlock::Text {
                        text: "Text part".to_string(),
                    },
                    ContentBlock::Image {
                        source: brainwires_core::ImageSource::Base64 {
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
            },
        ];

        let converted = provider.convert_messages(&messages);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].parts.len(), 4);
    }
}
