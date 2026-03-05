//! Groq API — constants and model listing.
//!
//! The chat-level `Provider` impl lives in `brainwires-chat::GroqChatProvider`.

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;

use crate::model_listing::{
    AvailableModel, ModelCapability, ModelLister, OpenAIListResponse,
    infer_openai_capabilities,
};

/// Groq chat completions endpoint (OpenAI-compatible).
pub const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";

/// Groq models endpoint.
pub const GROQ_MODELS_URL: &str = "https://api.groq.com/openai/v1/models";

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
