//! Provider factory — DEPRECATED.
//!
//! The canonical factory now lives in `brainwires-chat::ChatProviderFactory`.
//! This stub is kept so that `brainwires_providers::ProviderFactory` still resolves,
//! but it only supports providers that still implement `Provider` directly
//! (Ollama, BrainwiresHttp).  For the full set, use `brainwires-chat`.

use anyhow::{anyhow, Result};
use std::sync::Arc;

use brainwires_core::provider::Provider;
use super::{ProviderConfig, ProviderType};

/// Deprecated — prefer [`brainwires_chat::ChatProviderFactory`].
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider from config.
    ///
    /// Only Ollama and Brainwires are supported here.
    /// For all providers use `brainwires_chat::ChatProviderFactory::create()`.
    pub fn create(config: &ProviderConfig) -> Result<Arc<dyn Provider>> {
        match config.provider {
            ProviderType::Ollama => {
                Ok(Arc::new(super::OllamaProvider::new(
                    config.model.clone(),
                    config.base_url.clone(),
                )))
            }
            ProviderType::Brainwires => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Brainwires provider requires an API key"))?;
                let backend_url = config.base_url.clone()
                    .unwrap_or_else(|| "https://brainwires.studio".to_string());
                Ok(Arc::new(super::BrainwiresHttpProvider::new(
                    api_key,
                    backend_url,
                    config.model.clone(),
                )))
            }
            other => {
                Err(anyhow!(
                    "Provider '{}' no longer supported in brainwires-providers::ProviderFactory. \
                     Use brainwires_chat::ChatProviderFactory instead.",
                    other
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ollama_no_key_required() {
        let config = ProviderConfig::new(ProviderType::Ollama, "llama3.1".to_string());
        let result = ProviderFactory::create(&config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "ollama");
    }

    #[test]
    fn test_create_brainwires_with_key() {
        let config = ProviderConfig::new(ProviderType::Brainwires, "gpt-5-mini".to_string())
            .with_api_key("bw_test_key")
            .with_base_url("https://brainwires.studio");
        let result = ProviderFactory::create(&config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "brainwires");
    }

    #[test]
    fn test_create_anthropic_redirects() {
        let config = ProviderConfig::new(ProviderType::Anthropic, "claude-3".to_string())
            .with_api_key("sk-test");
        let result = ProviderFactory::create(&config);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("ChatProviderFactory"));
    }
}
