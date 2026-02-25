//! AI provider implementations for the Brainwires Agent Framework.
//!
//! Contains concrete implementations of the `Provider` trait for various AI services.

// Re-export core traits for convenience
pub use brainwires_core::provider::{ChatOptions, Provider};

// Rate limiting and HTTP client
#[cfg(feature = "native")]
pub mod rate_limiter;
#[cfg(feature = "native")]
pub mod http_client;

#[cfg(feature = "native")]
pub use http_client::RateLimitedClient;
#[cfg(feature = "native")]
pub use rate_limiter::RateLimiter;

// Generic HTTP providers (feature-gated behind "native")
#[cfg(feature = "native")]
pub mod anthropic;
#[cfg(feature = "native")]
pub mod openai;
#[cfg(feature = "native")]
pub mod google;
#[cfg(feature = "native")]
pub mod ollama;

// Local LLM provider (always compiled, llama.cpp behind feature flag in CLI)
pub mod local_llm;

// Re-export provider implementations at crate root
#[cfg(feature = "native")]
pub use anthropic::AnthropicProvider;
#[cfg(feature = "native")]
pub use openai::OpenAIProvider;
#[cfg(feature = "native")]
pub use google::GoogleProvider;
#[cfg(feature = "native")]
pub use ollama::OllamaProvider;
pub use local_llm::*;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// AI provider types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Anthropic,
    OpenAI,
    Google,
    Ollama,
    Custom,
}

impl ProviderType {
    /// Get the default model for this provider
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Anthropic => "claude-3-5-sonnet-20241022",
            Self::OpenAI => "gpt-4o",
            Self::Google => "gemini-2.0-flash-exp",
            Self::Ollama => "llama3.1",
            Self::Custom => "claude-3-5-sonnet-20241022",
        }
    }

    /// Parse from string
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAI),
            "google" | "gemini" => Some(Self::Google),
            "ollama" => Some(Self::Ollama),
            "custom" | "brainwires" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Google => "google",
            Self::Ollama => "ollama",
            Self::Custom => "custom",
        }
    }
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for ProviderType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_opt(s)
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", s))
    }
}

/// Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type
    pub provider: ProviderType,
    /// Model name
    pub model: String,
    /// API key (if required)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Base URL (for custom endpoints)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Additional provider-specific options
    #[serde(flatten)]
    pub options: std::collections::HashMap<String, serde_json::Value>,
}

impl ProviderConfig {
    /// Create a new provider config
    pub fn new(provider: ProviderType, model: String) -> Self {
        Self {
            provider,
            model,
            api_key: None,
            base_url: None,
            options: std::collections::HashMap::new(),
        }
    }

    /// Set API key
    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set base URL
    pub fn with_base_url<S: Into<String>>(mut self, base_url: S) -> Self {
        self.base_url = Some(base_url.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_default_model() {
        assert_eq!(ProviderType::Anthropic.default_model(), "claude-3-5-sonnet-20241022");
        assert_eq!(ProviderType::OpenAI.default_model(), "gpt-4o");
        assert_eq!(ProviderType::Google.default_model(), "gemini-2.0-flash-exp");
        assert_eq!(ProviderType::Ollama.default_model(), "llama3.1");
    }

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(ProviderType::from_str_opt("anthropic"), Some(ProviderType::Anthropic));
        assert_eq!(ProviderType::from_str_opt("openai"), Some(ProviderType::OpenAI));
        assert_eq!(ProviderType::from_str_opt("google"), Some(ProviderType::Google));
        assert_eq!(ProviderType::from_str_opt("gemini"), Some(ProviderType::Google));
        assert_eq!(ProviderType::from_str_opt("ollama"), Some(ProviderType::Ollama));
        assert_eq!(ProviderType::from_str_opt("brainwires"), Some(ProviderType::Custom));
        assert_eq!(ProviderType::from_str_opt("unknown"), None);
    }

    #[test]
    fn test_provider_config() {
        let config = ProviderConfig::new(ProviderType::Anthropic, "claude-3".to_string())
            .with_api_key("sk-test")
            .with_base_url("https://api.example.com");
        assert_eq!(config.provider, ProviderType::Anthropic);
        assert_eq!(config.api_key, Some("sk-test".to_string()));
        assert_eq!(config.base_url, Some("https://api.example.com".to_string()));
    }
}
