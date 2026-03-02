use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use brainwires_core::{ChatOptions, ChatResponse, Message, Provider, StreamChunk, Tool};

use super::OpenAIProvider;

const TOGETHER_API_URL: &str = "https://api.together.xyz/v1/chat/completions";

/// Together AI provider (OpenAI-compatible API).
pub struct TogetherProvider {
    inner: OpenAIProvider,
}

impl TogetherProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            inner: OpenAIProvider::new(api_key, model).with_base_url(TOGETHER_API_URL.to_string()),
        }
    }
}

#[async_trait]
impl Provider for TogetherProvider {
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
