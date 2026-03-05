//! Ollama chat provider.
//!
//! Delegates to the `OllamaProvider` in `brainwires-providers` which still
//! implements `Provider` directly (Ollama has no separate API client/chat split
//! since it only does chat).

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use brainwires_core::{ChatOptions, ChatResponse, Message, Provider, StreamChunk, Tool};

/// Ollama local model chat provider.
pub struct OllamaChatProvider {
    inner: brainwires_providers::OllamaProvider,
}

impl OllamaChatProvider {
    /// Create a new Ollama chat provider.
    pub fn new(model: String, base_url: Option<String>) -> Self {
        Self {
            inner: brainwires_providers::OllamaProvider::new(model, base_url),
        }
    }

    /// Create with rate limiting.
    pub fn with_rate_limit(model: String, base_url: Option<String>, rpm: u32) -> Self {
        Self {
            inner: brainwires_providers::OllamaProvider::with_rate_limit(model, base_url, rpm),
        }
    }
}

#[async_trait]
impl Provider for OllamaChatProvider {
    fn name(&self) -> &str {
        "ollama"
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
