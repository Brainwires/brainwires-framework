//! Brainwires HTTP relay chat provider.
//!
//! Delegates to the `BrainwiresHttpProvider` in `brainwires-providers`.

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use brainwires_core::{ChatOptions, ChatResponse, Message, Provider, StreamChunk, Tool};

/// Brainwires Studio HTTP relay chat provider.
pub struct BrainwiresHttpChatProvider {
    inner: brainwires_providers::BrainwiresHttpProvider,
}

impl BrainwiresHttpChatProvider {
    /// Create a new Brainwires HTTP chat provider.
    pub fn new(api_key: String, backend_url: String, model: String) -> Self {
        Self {
            inner: brainwires_providers::BrainwiresHttpProvider::new(api_key, backend_url, model),
        }
    }
}

#[async_trait]
impl Provider for BrainwiresHttpChatProvider {
    fn name(&self) -> &str {
        "brainwires"
    }

    fn max_output_tokens(&self) -> Option<u32> {
        self.inner.max_output_tokens()
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
