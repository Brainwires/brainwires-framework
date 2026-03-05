#![warn(missing_docs)]
//! Pure API connection layer for the Brainwires Agent Framework.
//!
//! Each provider module contains a low-level API client struct that handles
//! HTTP transport, authentication, rate limiting, and (de)serialisation of
//! provider-specific wire types.  Domain-level abstractions (the `Provider`
//! trait, message conversion, etc.) live in `brainwires-chat`.

// Re-export core traits for convenience (kept for backward compat)
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

// ── Chat-capable API clients ────────────────────────────────────────────

/// Anthropic (Claude) API client.
#[cfg(feature = "native")]
pub mod anthropic;
/// OpenAI API client.
#[cfg(feature = "native")]
pub mod openai;
/// Google (Gemini) API client.
#[cfg(feature = "native")]
pub mod google;
/// Groq constants and model listing.
#[cfg(feature = "native")]
pub mod groq;
/// Ollama local model API client.
#[cfg(feature = "native")]
pub mod ollama;
/// Brainwires HTTP relay API client.
#[cfg(feature = "native")]
pub mod brainwires_http;
/// Together AI constants.
#[cfg(feature = "native")]
pub mod together;
/// Fireworks AI constants.
#[cfg(feature = "native")]
pub mod fireworks;
/// Anyscale constants.
#[cfg(feature = "native")]
pub mod anyscale;

// ── Audio/speech API clients ────────────────────────────────────────────

/// ElevenLabs TTS/STT API client.
#[cfg(feature = "native")]
pub mod elevenlabs;
/// Deepgram TTS/STT API client.
#[cfg(feature = "native")]
pub mod deepgram;
/// Google Cloud Text-to-Speech API client.
#[cfg(feature = "native")]
pub mod google_tts;
/// Azure Cognitive Services Speech API client.
#[cfg(feature = "native")]
pub mod azure_speech;
/// Fish Audio TTS/ASR API client.
#[cfg(feature = "native")]
pub mod fish;
/// Cartesia TTS API client.
#[cfg(feature = "native")]
pub mod cartesia;
/// Murf AI TTS API client.
#[cfg(feature = "native")]
pub mod murf;

// ── Model listing ───────────────────────────────────────────────────────

/// Model listing — query available models from provider APIs.
#[cfg(feature = "native")]
pub mod model_listing;

/// Provider factory (deprecated — use `brainwires-chat::ChatProviderFactory`).
#[cfg(feature = "native")]
pub mod factory;

// ── Local LLM ───────────────────────────────────────────────────────────

/// Local LLM inference (always compiled, llama.cpp behind feature flag).
pub mod local_llm;

// ── Re-exports ──────────────────────────────────────────────────────────

// Chat-capable API clients
#[cfg(feature = "native")]
pub use anthropic::AnthropicClient;
#[cfg(feature = "native")]
pub use openai::OpenAiClient;
#[cfg(feature = "native")]
pub use google::GoogleClient;
#[cfg(feature = "native")]
pub use ollama::OllamaProvider;
#[cfg(feature = "native")]
pub use brainwires_http::BrainwiresHttpProvider;

// Groq model listing
#[cfg(feature = "native")]
pub use groq::GroqModelLister;

// Audio API clients
#[cfg(feature = "native")]
pub use elevenlabs::ElevenLabsClient;
#[cfg(feature = "native")]
pub use deepgram::DeepgramClient;
#[cfg(feature = "native")]
pub use google_tts::GoogleTtsClient;
#[cfg(feature = "native")]
pub use azure_speech::AzureSpeechClient;
#[cfg(feature = "native")]
pub use fish::FishClient;
#[cfg(feature = "native")]
pub use cartesia::CartesiaClient;
#[cfg(feature = "native")]
pub use murf::MurfClient;

// Factory + model listing
#[cfg(feature = "native")]
pub use factory::ProviderFactory;
#[cfg(feature = "native")]
pub use model_listing::{AvailableModel, ModelCapability, ModelLister, create_model_lister};

// Local LLM
pub use local_llm::*;

// ── Backward-compat type aliases ────────────────────────────────────────
// The old names pointed to structs that have been renamed. Downstream code
// that hasn't migrated to `brainwires-chat` yet can still compile.
#[cfg(feature = "native")]
/// Backward-compat alias for [`AnthropicClient`].
pub type AnthropicProvider = AnthropicClient;
#[cfg(feature = "native")]
/// Backward-compat alias for [`OpenAiClient`].
pub type OpenAIProvider = OpenAiClient;
#[cfg(feature = "native")]
/// Backward-compat alias for [`GoogleClient`].
pub type GoogleProvider = GoogleClient;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// AI provider types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    /// Anthropic (Claude).
    Anthropic,
    /// OpenAI (GPT).
    OpenAI,
    /// Google (Gemini).
    Google,
    /// Groq inference.
    Groq,
    /// Ollama local models.
    Ollama,
    /// Brainwires HTTP relay.
    Brainwires,
    /// Together AI.
    Together,
    /// Fireworks AI.
    Fireworks,
    /// Anyscale.
    Anyscale,
    /// ElevenLabs.
    ElevenLabs,
    /// Deepgram.
    Deepgram,
    /// Azure Speech.
    Azure,
    /// Fish Audio.
    Fish,
    /// Cartesia.
    Cartesia,
    /// Murf AI.
    Murf,
    /// Custom / user-defined provider.
    Custom,
}

impl ProviderType {
    /// Get the default model for this provider
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::Anthropic => "claude-3-5-sonnet-20241022",
            Self::OpenAI => "gpt-4o",
            Self::Google => "gemini-2.0-flash-exp",
            Self::Groq => "llama-3.3-70b-versatile",
            Self::Ollama => "llama3.1",
            Self::Brainwires => "gpt-5-mini",
            Self::Together => "meta-llama/Llama-3.1-8B-Instruct",
            Self::Fireworks => "accounts/fireworks/models/llama-v3p1-8b-instruct",
            Self::Anyscale => "meta-llama/Meta-Llama-3.1-8B-Instruct",
            Self::ElevenLabs => "eleven_multilingual_v2",
            Self::Deepgram => "nova-2",
            Self::Azure => "en-US-JennyNeural",
            Self::Fish => "default",
            Self::Cartesia => "sonic-english",
            Self::Murf => "en-US-natalie",
            Self::Custom => "claude-3-5-sonnet-20241022",
        }
    }

    /// Parse from string
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAI),
            "google" | "gemini" => Some(Self::Google),
            "groq" => Some(Self::Groq),
            "ollama" => Some(Self::Ollama),
            "brainwires" => Some(Self::Brainwires),
            "together" => Some(Self::Together),
            "fireworks" => Some(Self::Fireworks),
            "anyscale" => Some(Self::Anyscale),
            "elevenlabs" => Some(Self::ElevenLabs),
            "deepgram" => Some(Self::Deepgram),
            "azure" => Some(Self::Azure),
            "fish" => Some(Self::Fish),
            "cartesia" => Some(Self::Cartesia),
            "murf" => Some(Self::Murf),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Google => "google",
            Self::Groq => "groq",
            Self::Ollama => "ollama",
            Self::Brainwires => "brainwires",
            Self::Together => "together",
            Self::Fireworks => "fireworks",
            Self::Anyscale => "anyscale",
            Self::ElevenLabs => "elevenlabs",
            Self::Deepgram => "deepgram",
            Self::Azure => "azure",
            Self::Fish => "fish",
            Self::Cartesia => "cartesia",
            Self::Murf => "murf",
            Self::Custom => "custom",
        }
    }

    /// Whether this provider requires an API key
    pub fn requires_api_key(&self) -> bool {
        !matches!(self, Self::Ollama)
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
        assert_eq!(ProviderType::Groq.default_model(), "llama-3.3-70b-versatile");
        assert_eq!(ProviderType::Ollama.default_model(), "llama3.1");
        assert_eq!(ProviderType::Brainwires.default_model(), "gpt-5-mini");
    }

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(ProviderType::from_str_opt("anthropic"), Some(ProviderType::Anthropic));
        assert_eq!(ProviderType::from_str_opt("openai"), Some(ProviderType::OpenAI));
        assert_eq!(ProviderType::from_str_opt("google"), Some(ProviderType::Google));
        assert_eq!(ProviderType::from_str_opt("gemini"), Some(ProviderType::Google));
        assert_eq!(ProviderType::from_str_opt("groq"), Some(ProviderType::Groq));
        assert_eq!(ProviderType::from_str_opt("ollama"), Some(ProviderType::Ollama));
        assert_eq!(ProviderType::from_str_opt("brainwires"), Some(ProviderType::Brainwires));
        assert_eq!(ProviderType::from_str_opt("together"), Some(ProviderType::Together));
        assert_eq!(ProviderType::from_str_opt("fireworks"), Some(ProviderType::Fireworks));
        assert_eq!(ProviderType::from_str_opt("anyscale"), Some(ProviderType::Anyscale));
        assert_eq!(ProviderType::from_str_opt("elevenlabs"), Some(ProviderType::ElevenLabs));
        assert_eq!(ProviderType::from_str_opt("deepgram"), Some(ProviderType::Deepgram));
        assert_eq!(ProviderType::from_str_opt("custom"), Some(ProviderType::Custom));
        assert_eq!(ProviderType::from_str_opt("unknown"), None);
    }

    #[test]
    fn test_provider_type_requires_api_key() {
        assert!(ProviderType::Anthropic.requires_api_key());
        assert!(ProviderType::OpenAI.requires_api_key());
        assert!(!ProviderType::Ollama.requires_api_key());
        assert!(ProviderType::ElevenLabs.requires_api_key());
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
