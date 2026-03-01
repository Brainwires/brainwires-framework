use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use brainwires_core::{ChatResponse, Message, StreamChunk};
use brainwires_core::{ChatOptions, Provider};
use brainwires_core::Tool;

use super::openai::OpenAIProvider;

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

/// Groq provider — thin wrapper around OpenAI-compatible API.
pub struct GroqProvider {
    inner: OpenAIProvider,
}

impl GroqProvider {
    pub fn new(api_key: String, model: String) -> Self {
        let inner = OpenAIProvider::new(api_key, model)
            .with_base_url(GROQ_API_URL.to_string());
        Self { inner }
    }
}

#[async_trait]
impl Provider for GroqProvider {
    fn name(&self) -> &str {
        "groq"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groq_provider_name() {
        let provider = GroqProvider::new("test-key".to_string(), "llama-3.3-70b-versatile".to_string());
        assert_eq!(provider.name(), "groq");
    }
}
