use anyhow::{anyhow, Result};
use std::sync::Arc;

use brainwires_core::provider::Provider;
use super::{ProviderConfig, ProviderType};

/// Pure provider factory — creates provider instances from config.
///
/// No CLI dependencies (no SessionManager, no keyring, no file I/O).
/// The caller is responsible for resolving API keys and base URLs
/// before calling `create()`.
pub struct ProviderFactory;

impl ProviderFactory {
    /// Create a provider from a fully-resolved config.
    ///
    /// All fields (api_key, base_url, model) must already be populated.
    pub fn create(config: &ProviderConfig) -> Result<Arc<dyn Provider>> {
        match config.provider {
            ProviderType::Anthropic => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Anthropic provider requires an API key"))?;
                Ok(Arc::new(super::AnthropicProvider::new(api_key, config.model.clone())))
            }
            ProviderType::OpenAI => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("OpenAI provider requires an API key"))?;
                let mut provider = super::OpenAIProvider::new(api_key, config.model.clone());
                if let Some(ref url) = config.base_url {
                    provider = provider.with_base_url(url.clone());
                }
                Ok(Arc::new(provider))
            }
            ProviderType::Google => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Google provider requires an API key"))?;
                Ok(Arc::new(super::GoogleProvider::new(api_key, config.model.clone())))
            }
            ProviderType::Groq => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Groq provider requires an API key"))?;
                Ok(Arc::new(super::GroqProvider::new(api_key, config.model.clone())))
            }
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
            ProviderType::Together => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Together provider requires an API key"))?;
                Ok(Arc::new(super::TogetherProvider::new(api_key, config.model.clone())))
            }
            ProviderType::Fireworks => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Fireworks provider requires an API key"))?;
                Ok(Arc::new(super::FireworksProvider::new(api_key, config.model.clone())))
            }
            ProviderType::Anyscale => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Anyscale provider requires an API key"))?;
                Ok(Arc::new(super::AnyscaleProvider::new(api_key, config.model.clone())))
            }
            ProviderType::Custom => {
                Err(anyhow!("Custom provider type requires a custom factory implementation"))
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
    fn test_create_anthropic_requires_key() {
        let config = ProviderConfig::new(ProviderType::Anthropic, "claude-3".to_string());
        let result = ProviderFactory::create(&config);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("requires an API key"));
    }

    #[test]
    fn test_create_groq_with_key() {
        let config = ProviderConfig::new(ProviderType::Groq, "llama-3.3-70b-versatile".to_string())
            .with_api_key("gsk_test");
        let result = ProviderFactory::create(&config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "groq");
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
    fn test_create_custom_unsupported() {
        let config = ProviderConfig::new(ProviderType::Custom, "model".to_string());
        let result = ProviderFactory::create(&config);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("Custom provider"));
    }
}
