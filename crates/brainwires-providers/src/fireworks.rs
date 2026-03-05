use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use brainwires_core::{ChatOptions, ChatResponse, Message, Provider, StreamChunk, Tool};

use super::OpenAIProvider;

const FIREWORKS_API_URL: &str = "https://api.fireworks.ai/inference/v1/chat/completions";

/// Fireworks AI provider (OpenAI-compatible API).
/// Fireworks AI provider (OpenAI-compatible).
pub struct FireworksProvider {
    inner: OpenAIProvider,
}

impl FireworksProvider {
    /// Create a new Fireworks AI provider with the given API key and model.
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            inner: OpenAIProvider::new(api_key, model).with_base_url(FIREWORKS_API_URL.to_string()),
        }
    }
}

#[async_trait]
impl Provider for FireworksProvider {
    fn name(&self) -> &str {
        "fireworks"
    }

    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> Result<ChatResponse> {
        self.inner.chat(messages, tools, options).await
    }

    fn stream_chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: Option<&'a [Tool]>,
        options: &'a ChatOptions,
    ) -> BoxStream<'a, Result<StreamChunk>> {
        self.inner.stream_chat(messages, tools, options)
    }
}
