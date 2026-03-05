use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::rate_limiter::RateLimiter;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

// ---------------------------------------------------------------------------
// API client
// ---------------------------------------------------------------------------

/// Low-level Anthropic (Claude) API client.
///
/// This struct handles authentication, rate-limiting, and HTTP transport.
/// It exposes raw API methods that return Anthropic-native types; higher-level
/// abstractions (e.g. the `Provider` trait) live in the `brainwires-chat` crate.
pub struct AnthropicClient {
    api_key: String,
    model: String,
    http_client: Client,
    rate_limiter: Option<std::sync::Arc<RateLimiter>>,
}

impl AnthropicClient {
    /// Create a new Anthropic client with the given API key and model.
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            model,
            http_client: Client::new(),
            rate_limiter: None,
        }
    }

    /// Create a client with rate limiting (requests per minute).
    pub fn with_rate_limit(api_key: String, model: String, requests_per_minute: u32) -> Self {
        Self {
            api_key,
            model,
            http_client: Client::new(),
            rate_limiter: Some(std::sync::Arc::new(RateLimiter::new(requests_per_minute))),
        }
    }

    /// Return the model name this client was created with.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Return the API key (useful when constructing an `AnthropicModelLister`).
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Wait for rate-limit clearance (no-op if not configured).
    async fn acquire_rate_limit(&self) {
        if let Some(ref limiter) = self.rate_limiter {
            limiter.acquire().await;
        }
    }

    // -----------------------------------------------------------------------
    // Raw API methods
    // -----------------------------------------------------------------------

    /// Send a non-streaming request to `/v1/messages` and return the parsed
    /// response.
    pub async fn messages(&self, req: &AnthropicRequest) -> Result<AnthropicResponse> {
        let mut request_body = json!({
            "model": req.model,
            "messages": req.messages,
            "max_tokens": req.max_tokens,
            "stream": false,
        });

        if let Some(ref sys) = req.system {
            request_body["system"] = json!(sys);
        }
        if let Some(temp) = req.temperature {
            request_body["temperature"] = json!(temp);
        }
        if let Some(top_p) = req.top_p {
            request_body["top_p"] = json!(top_p);
        }
        if let Some(ref stop) = req.stop_sequences {
            request_body["stop_sequences"] = json!(stop);
        }
        if let Some(ref tools) = req.tools {
            request_body["tools"] = json!(tools);
        }

        self.acquire_rate_limit().await;

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
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Anthropic API error ({}): {}", status, error_text);
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        Ok(anthropic_response)
    }

    /// Send a streaming request to `/v1/messages` and return a stream of raw
    /// SSE events.
    pub fn stream_messages<'a>(
        &'a self,
        req: &'a AnthropicRequest,
    ) -> BoxStream<'a, Result<AnthropicStreamEvent>> {
        Box::pin(async_stream::stream! {
            let mut request_body = json!({
                "model": req.model,
                "messages": req.messages,
                "max_tokens": req.max_tokens,
                "stream": true,
            });

            if let Some(ref sys) = req.system {
                request_body["system"] = json!(sys);
            }
            if let Some(temp) = req.temperature {
                request_body["temperature"] = json!(temp);
            }
            if let Some(top_p) = req.top_p {
                request_body["top_p"] = json!(top_p);
            }
            if let Some(ref stop) = req.stop_sequences {
                request_body["stop_sequences"] = json!(stop);
            }
            if let Some(ref tools) = req.tools {
                request_body["tools"] = json!(tools);
            }

            self.acquire_rate_limit().await;

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
                            continue;
                        }

                        match serde_json::from_str::<AnthropicStreamEvent>(data) {
                            Ok(event) => {
                                yield Ok(event);
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

    /// Paginated listing of models available via `/v1/models`.
    pub async fn list_models(&self) -> Result<Vec<AnthropicModelEntry>> {
        let mut all_models = Vec::new();
        let mut after_id: Option<String> = None;

        loop {
            let mut url = format!("{}?limit=1000", ANTHROPIC_MODELS_URL);
            if let Some(ref cursor) = after_id {
                url.push_str(&format!("&after_id={}", cursor));
            }

            let resp = self
                .http_client
                .get(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .send()
                .await
                .context("Failed to list Anthropic models")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "Anthropic models API returned {}: {}",
                    status,
                    body
                ));
            }

            let page: AnthropicListResponse = resp
                .json()
                .await
                .context("Failed to parse Anthropic models response")?;

            for entry in page.data {
                all_models.push(entry);
            }

            if !page.has_more {
                break;
            }
            after_id = page.last_id;
        }

        Ok(all_models)
    }
}

// ---------------------------------------------------------------------------
// Request type
// ---------------------------------------------------------------------------

/// A request to the Anthropic `/v1/messages` endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicRequest {
    /// Model identifier (e.g. `"claude-sonnet-4-20250514"`).
    pub model: String,
    /// Conversation messages.
    pub messages: Vec<AnthropicMessage>,
    /// Optional system prompt (sent as a top-level field, not a message).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Maximum number of tokens to generate.
    pub max_tokens: u32,
    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Nucleus sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Tools available to the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AnthropicTool>>,
    /// Whether to stream the response.
    #[serde(default)]
    pub stream: bool,
}

// ---------------------------------------------------------------------------
// Anthropic API serde types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicContentBlock {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnthropicTool {
    pub name: String,
    pub description: String,
    pub input_schema: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnthropicResponse {
    pub content: Vec<AnthropicContentBlock>,
    pub stop_reason: String,
    pub usage: AnthropicUsage,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnthropicUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub delta: Option<AnthropicDelta>,
    pub usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnthropicDelta {
    pub text: Option<String>,
}

// ---------------------------------------------------------------------------
// Model listing
// ---------------------------------------------------------------------------

use crate::model_listing::{
    AnthropicListResponse, AnthropicModelEntry, AvailableModel, ModelCapability, ModelLister,
};

const ANTHROPIC_MODELS_URL: &str = "https://api.anthropic.com/v1/models";

/// Lists models available from the Anthropic API.
pub struct AnthropicModelLister {
    api_key: String,
    http_client: Client,
}

impl AnthropicModelLister {
    /// Create a new model lister with the given API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: Client::new(),
        }
    }
}

#[async_trait]
impl ModelLister for AnthropicModelLister {
    async fn list_models(&self) -> Result<Vec<AvailableModel>> {
        let mut all_models = Vec::new();
        let mut after_id: Option<String> = None;

        loop {
            let mut url = format!("{}?limit=1000", ANTHROPIC_MODELS_URL);
            if let Some(ref cursor) = after_id {
                url.push_str(&format!("&after_id={}", cursor));
            }

            let resp = self
                .http_client
                .get(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .send()
                .await
                .context("Failed to list Anthropic models")?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(anyhow::anyhow!(
                    "Anthropic models API returned {}: {}",
                    status,
                    body
                ));
            }

            let page: AnthropicListResponse = resp
                .json()
                .await
                .context("Failed to parse Anthropic models response")?;

            for entry in &page.data {
                all_models.push(AvailableModel {
                    id: entry.id.clone(),
                    display_name: Some(entry.display_name.clone()),
                    provider: crate::ProviderType::Anthropic,
                    capabilities: vec![
                        ModelCapability::Chat,
                        ModelCapability::ToolUse,
                        ModelCapability::Vision,
                    ],
                    owned_by: Some("anthropic".to_string()),
                    context_window: None,
                    max_output_tokens: None,
                    created_at: None,
                });
            }

            if !page.has_more {
                break;
            }
            after_id = page.last_id;
        }

        Ok(all_models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_client_new() {
        let client = AnthropicClient::new("test-key".to_string(), "claude-3-sonnet".to_string());
        assert_eq!(client.api_key, "test-key");
        assert_eq!(client.model, "claude-3-sonnet");
    }

    #[test]
    fn test_client_model_accessor() {
        let client = AnthropicClient::new("test-key".to_string(), "claude-3-sonnet".to_string());
        assert_eq!(client.model(), "claude-3-sonnet");
    }

    #[test]
    fn test_client_api_key_accessor() {
        let client = AnthropicClient::new("test-key".to_string(), "claude-3-sonnet".to_string());
        assert_eq!(client.api_key(), "test-key");
    }

    #[test]
    fn test_anthropic_client_with_empty_api_key() {
        let client = AnthropicClient::new("".to_string(), "claude-3-sonnet".to_string());
        assert_eq!(client.api_key, "");
        assert_eq!(client.model, "claude-3-sonnet");
    }

    #[test]
    fn test_anthropic_client_with_special_characters_in_api_key() {
        let api_key = "sk-ant-api03-!@#$%^&*()_+-=[]{}|;':\",./<>?".to_string();
        let client = AnthropicClient::new(api_key.clone(), "claude-3-opus".to_string());
        assert_eq!(client.api_key, api_key);
    }

    #[test]
    fn test_anthropic_client_with_various_model_names() {
        let models = vec![
            "claude-3-opus-20240229",
            "claude-3-sonnet-20240229",
            "claude-3-haiku-20240307",
            "claude-2.1",
            "claude-2.0",
            "custom-model-123",
        ];

        for model in models {
            let client = AnthropicClient::new("test-key".to_string(), model.to_string());
            assert_eq!(client.model, model);
        }
    }

    #[test]
    fn test_anthropic_constants() {
        assert_eq!(ANTHROPIC_API_URL, "https://api.anthropic.com/v1/messages");
        assert_eq!(ANTHROPIC_VERSION, "2023-06-01");
    }

    #[test]
    fn test_anthropic_request_serialization() {
        let req = AnthropicRequest {
            model: "claude-3-sonnet".to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: vec![AnthropicContentBlock::Text {
                    text: "Hello".to_string(),
                }],
            }],
            system: Some("You are helpful".to_string()),
            max_tokens: 4096,
            temperature: Some(0.7),
            top_p: None,
            stop_sequences: None,
            tools: None,
            stream: false,
        };

        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-3-sonnet");
        assert_eq!(json["max_tokens"], 4096);
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 1e-6, "temperature {temp} not close to 0.7");
        assert!(json.get("top_p").is_none());
        assert!(json.get("stop_sequences").is_none());
        assert!(json.get("tools").is_none());
    }

    #[test]
    fn test_anthropic_content_block_text_serde() {
        let block = AnthropicContentBlock::Text {
            text: "hello".to_string(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "hello");
    }

    #[test]
    fn test_anthropic_content_block_tool_use_serde() {
        let block = AnthropicContentBlock::ToolUse {
            id: "tool-1".to_string(),
            name: "search".to_string(),
            input: serde_json::json!({"query": "test"}),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "tool_use");
        assert_eq!(json["id"], "tool-1");
        assert_eq!(json["name"], "search");
        assert_eq!(json["input"]["query"], "test");
    }

    #[test]
    fn test_anthropic_content_block_tool_result_serde() {
        let block = AnthropicContentBlock::ToolResult {
            tool_use_id: "tool-1".to_string(),
            content: "result text".to_string(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["tool_use_id"], "tool-1");
        assert_eq!(json["content"], "result text");
    }

    #[test]
    fn test_anthropic_message_serialization() {
        let msg = AnthropicMessage {
            role: "user".to_string(),
            content: vec![
                AnthropicContentBlock::Text {
                    text: "Look at this".to_string(),
                },
                AnthropicContentBlock::ToolUse {
                    id: "t1".to_string(),
                    name: "read".to_string(),
                    input: serde_json::json!({}),
                },
            ],
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_anthropic_response_deserialization() {
        let json = r#"{
            "content": [
                {"type": "text", "text": "Hello!"}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        }"#;
        let resp: AnthropicResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.stop_reason, "end_turn");
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 5);
        assert_eq!(resp.content.len(), 1);
    }

    #[test]
    fn test_anthropic_stream_event_deserialization() {
        let json = r#"{
            "type": "content_block_delta",
            "delta": {"text": "Hi"},
            "usage": null
        }"#;
        let event: AnthropicStreamEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "content_block_delta");
        assert_eq!(event.delta.unwrap().text.unwrap(), "Hi");
    }

    #[test]
    fn test_anthropic_stream_event_message_delta() {
        let json = r#"{
            "type": "message_delta",
            "delta": null,
            "usage": {"input_tokens": 0, "output_tokens": 42}
        }"#;
        let event: AnthropicStreamEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "message_delta");
        assert_eq!(event.usage.unwrap().output_tokens, 42);
    }

    #[test]
    fn test_anthropic_tool_serialization() {
        let mut schema = std::collections::HashMap::new();
        schema.insert(
            "query".to_string(),
            serde_json::json!({"type": "string", "description": "Search query"}),
        );
        let tool = AnthropicTool {
            name: "search".to_string(),
            description: "Search the web".to_string(),
            input_schema: schema,
        };
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["name"], "search");
        assert!(json["input_schema"]["query"].is_object());
    }
}
