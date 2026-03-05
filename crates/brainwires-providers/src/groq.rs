use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::BoxStream;

use brainwires_core::{ChatResponse, Message, StreamChunk};
use brainwires_core::{ChatOptions, Provider};
use brainwires_core::Tool;

use super::openai::OpenAIProvider;

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

/// Groq provider — thin wrapper around OpenAI-compatible API.
/// Groq inference API provider (OpenAI-compatible).
pub struct GroqProvider {
    inner: OpenAIProvider,
}

impl GroqProvider {
    /// Create a new Groq provider with the given API key and model.
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

// ---------------------------------------------------------------------------
// Model listing
// ---------------------------------------------------------------------------

use reqwest::Client;
use crate::model_listing::{
    AvailableModel, ModelCapability, ModelLister, OpenAIListResponse,
    infer_openai_capabilities,
};

const GROQ_MODELS_URL: &str = "https://api.groq.com/openai/v1/models";

/// Lists models available from the Groq API (OpenAI-compatible format).
pub struct GroqModelLister {
    api_key: String,
    http_client: Client,
}

impl GroqModelLister {
    /// Create a new model lister with the given API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http_client: Client::new(),
        }
    }
}

#[async_trait]
impl ModelLister for GroqModelLister {
    async fn list_models(&self) -> Result<Vec<AvailableModel>> {
        let resp = self
            .http_client
            .get(GROQ_MODELS_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .context("Failed to list Groq models")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Groq models API returned {}: {}",
                status,
                body
            ));
        }

        let list: OpenAIListResponse = resp.json().await
            .context("Failed to parse Groq models response")?;

        let models = list
            .data
            .into_iter()
            .map(|entry| {
                let mut caps = infer_openai_capabilities(&entry.id);
                // Groq models are primarily chat models; ensure Chat is always present
                if !caps.contains(&ModelCapability::Chat) {
                    caps.insert(0, ModelCapability::Chat);
                }
                AvailableModel {
                    id: entry.id,
                    display_name: None,
                    provider: crate::ProviderType::Groq,
                    capabilities: caps,
                    owned_by: entry.owned_by,
                    context_window: None,
                    max_output_tokens: None,
                    created_at: entry.created,
                }
            })
            .collect();

        Ok(models)
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
