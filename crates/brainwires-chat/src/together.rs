//! Together AI chat provider — thin wrapper over [`OpenAiChatProvider`].

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use brainwires_core::{ChatOptions, ChatResponse, Message, Provider, StreamChunk, Tool};
use brainwires_providers::openai::OpenAiClient;

use super::openai::OpenAiChatProvider;

const TOGETHER_API_URL: &str = "https://api.together.xyz/v1/chat/completions";

/// Together AI chat provider (OpenAI-compatible).
pub struct TogetherChatProvider {
    inner: OpenAiChatProvider,
}

impl TogetherChatProvider {
    /// Create a new Together AI chat provider.
    pub fn new(api_key: String, model: String) -> Self {
        let client = Arc::new(
            OpenAiClient::new(api_key, model.clone())
                .with_base_url(TOGETHER_API_URL.to_string()),
        );
        Self {
            inner: OpenAiChatProvider::new(client, model),
        }
    }
}

#[async_trait]
impl Provider for TogetherChatProvider {
    fn name(&self) -> &str {
        "together"
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
