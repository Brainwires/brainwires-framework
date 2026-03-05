//! Chat provider factory — creates `Arc<dyn Provider>` from configuration.

use std::sync::Arc;

use anyhow::{anyhow, Result};

use brainwires_core::Provider;
use brainwires_providers::{ProviderConfig, ProviderType};

/// Pure chat provider factory — creates provider instances from config.
///
/// No CLI dependencies (no SessionManager, no keyring, no file I/O).
/// The caller is responsible for resolving API keys and base URLs
/// before calling `create()`.
pub struct ChatProviderFactory;

impl ChatProviderFactory {
    /// Create a chat provider from a fully-resolved config.
    ///
    /// All fields (api_key, base_url, model) must already be populated.
    pub fn create(config: &ProviderConfig) -> Result<Arc<dyn Provider>> {
        match config.provider {
            ProviderType::Anthropic => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Anthropic provider requires an API key"))?;
                let client = Arc::new(brainwires_providers::AnthropicClient::new(
                    api_key, config.model.clone(),
                ));
                Ok(Arc::new(super::AnthropicChatProvider::new(client, config.model.clone())))
            }
            ProviderType::OpenAI => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("OpenAI provider requires an API key"))?;
                let mut client = brainwires_providers::OpenAiClient::new(
                    api_key, config.model.clone(),
                );
                if let Some(ref url) = config.base_url {
                    client = client.with_base_url(url.clone());
                }
                let client = Arc::new(client);
                Ok(Arc::new(super::OpenAiChatProvider::new(client, config.model.clone())))
            }
            ProviderType::Google => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Google provider requires an API key"))?;
                let client = Arc::new(brainwires_providers::GoogleClient::new(
                    api_key, config.model.clone(),
                ));
                Ok(Arc::new(super::GoogleChatProvider::new(client, config.model.clone())))
            }
            ProviderType::Groq => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Groq provider requires an API key"))?;
                Ok(Arc::new(super::GroqChatProvider::new(api_key, config.model.clone())))
            }
            ProviderType::Ollama => {
                Ok(Arc::new(super::OllamaChatProvider::new(
                    config.model.clone(),
                    config.base_url.clone(),
                )))
            }
            ProviderType::Brainwires => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Brainwires provider requires an API key"))?;
                let backend_url = config.base_url.clone()
                    .unwrap_or_else(|| "https://brainwires.studio".to_string());
                Ok(Arc::new(super::BrainwiresHttpChatProvider::new(
                    api_key,
                    backend_url,
                    config.model.clone(),
                )))
            }
            ProviderType::Together => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Together provider requires an API key"))?;
                Ok(Arc::new(super::TogetherChatProvider::new(api_key, config.model.clone())))
            }
            ProviderType::Fireworks => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Fireworks provider requires an API key"))?;
                Ok(Arc::new(super::FireworksChatProvider::new(api_key, config.model.clone())))
            }
            ProviderType::Anyscale => {
                let api_key = config.api_key.clone()
                    .ok_or_else(|| anyhow!("Anyscale provider requires an API key"))?;
                Ok(Arc::new(super::AnyscaleChatProvider::new(api_key, config.model.clone())))
            }
            _ => {
                Err(anyhow!("Provider type '{}' is not a chat provider", config.provider))
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
        let result = ChatProviderFactory::create(&config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "ollama");
    }

    #[test]
    fn test_create_anthropic_requires_key() {
        let config = ProviderConfig::new(ProviderType::Anthropic, "claude-3".to_string());
        let result = ChatProviderFactory::create(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_groq_with_key() {
        let config = ProviderConfig::new(ProviderType::Groq, "llama-3.3-70b-versatile".to_string())
            .with_api_key("gsk_test");
        let result = ChatProviderFactory::create(&config);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name(), "groq");
    }
}
