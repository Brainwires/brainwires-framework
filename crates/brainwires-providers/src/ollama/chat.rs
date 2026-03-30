//! Ollama chat provider.
//!
//! Delegates to the `OllamaProvider` which still implements `Provider`
//! directly (Ollama has no separate API client/chat split since it only
//! does chat).

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

use brainwires_core::{ChatOptions, ChatResponse, Message, Provider, StreamChunk, Tool};

/// Ollama local model chat provider.
pub struct OllamaChatProvider {
    inner: super::OllamaProvider,
    #[cfg(feature = "analytics")]
    model: String,
    #[cfg(feature = "analytics")]
    analytics_collector: Option<std::sync::Arc<brainwires_analytics::AnalyticsCollector>>,
}

impl OllamaChatProvider {
    /// Create a new Ollama chat provider.
    pub fn new(model: String, base_url: Option<String>) -> Self {
        Self {
            inner: super::OllamaProvider::new(model.clone(), base_url),
            #[cfg(feature = "analytics")]
            model,
            #[cfg(feature = "analytics")]
            analytics_collector: None,
        }
    }

    /// Create with rate limiting.
    pub fn with_rate_limit(model: String, base_url: Option<String>, rpm: u32) -> Self {
        Self {
            inner: super::OllamaProvider::with_rate_limit(model.clone(), base_url, rpm),
            #[cfg(feature = "analytics")]
            model,
            #[cfg(feature = "analytics")]
            analytics_collector: None,
        }
    }

    /// Attach an analytics collector to this provider.
    #[cfg(feature = "analytics")]
    pub fn with_analytics(mut self, collector: std::sync::Arc<brainwires_analytics::AnalyticsCollector>) -> Self {
        self.analytics_collector = Some(collector);
        self
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
        #[cfg(feature = "analytics")]
        let _started = std::time::Instant::now();
        let response = self.inner.chat(messages, tools, options).await?;
        #[cfg(feature = "analytics")]
        if let Some(ref collector) = self.analytics_collector {
            use brainwires_analytics::AnalyticsEvent;
            collector.record(AnalyticsEvent::ProviderCall {
                session_id: None,
                provider: "ollama".to_string(),
                model: self.model.clone(),
                prompt_tokens: response.usage.prompt_tokens,
                completion_tokens: response.usage.completion_tokens,
                duration_ms: _started.elapsed().as_millis() as u64,
                cost_usd: 0.0,
                success: true,
                timestamp: chrono::Utc::now(),
            });
        }
        Ok(response)
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
