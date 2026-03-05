#![warn(missing_docs)]
//! Chat provider implementations for the Brainwires Agent Framework.
//!
//! Each sub-module wraps a low-level API client from `brainwires-providers`
//! and implements the `brainwires_core::Provider` trait, handling all
//! conversions between core domain types and provider-specific wire types.

// Re-export core traits for convenience
pub use brainwires_core::provider::{ChatOptions, Provider};

// Re-export provider config types for factory use
pub use brainwires_providers::{ProviderConfig, ProviderType};

/// OpenAI chat provider.
#[cfg(feature = "native")]
pub mod openai;
/// Anthropic chat provider.
#[cfg(feature = "native")]
pub mod anthropic;
/// Google Gemini chat provider.
#[cfg(feature = "native")]
pub mod google;
/// Groq chat provider (wraps OpenAI chat provider).
#[cfg(feature = "native")]
pub mod groq;
/// Ollama chat provider.
#[cfg(feature = "native")]
pub mod ollama;
/// Brainwires HTTP relay chat provider.
#[cfg(feature = "native")]
pub mod brainwires_http;
/// Together AI chat provider (wraps OpenAI chat provider).
#[cfg(feature = "native")]
pub mod together;
/// Fireworks AI chat provider (wraps OpenAI chat provider).
#[cfg(feature = "native")]
pub mod fireworks;
/// Anyscale chat provider (wraps OpenAI chat provider).
#[cfg(feature = "native")]
pub mod anyscale;

/// Provider factory for constructing chat providers from configuration.
#[cfg(feature = "native")]
pub mod factory;

/// Model listing — query available models from provider APIs.
#[cfg(feature = "native")]
pub mod model_listing;

// Re-exports
#[cfg(feature = "native")]
pub use openai::OpenAiChatProvider;
#[cfg(feature = "native")]
pub use anthropic::AnthropicChatProvider;
#[cfg(feature = "native")]
pub use google::GoogleChatProvider;
#[cfg(feature = "native")]
pub use groq::GroqChatProvider;
#[cfg(feature = "native")]
pub use ollama::OllamaChatProvider;
#[cfg(feature = "native")]
pub use brainwires_http::BrainwiresHttpChatProvider;
#[cfg(feature = "native")]
pub use together::TogetherChatProvider;
#[cfg(feature = "native")]
pub use fireworks::FireworksChatProvider;
#[cfg(feature = "native")]
pub use anyscale::AnyscaleChatProvider;
#[cfg(feature = "native")]
pub use factory::ChatProviderFactory;
#[cfg(feature = "native")]
pub use model_listing::{AvailableModel, ModelCapability, ModelLister, create_model_lister};
