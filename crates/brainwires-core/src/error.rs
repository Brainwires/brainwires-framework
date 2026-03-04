//! Framework error types

use thiserror::Error;

/// Core framework errors
#[derive(Error, Debug)]
pub enum FrameworkError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Provider authentication failed for {provider}: {message}")]
    ProviderAuth {
        provider: String,
        message: String,
    },

    #[error("Provider model error ({provider}/{model}): {message}")]
    ProviderModel {
        provider: String,
        model: String,
        message: String,
    },

    #[error("Embedding dimension mismatch: expected {expected}, got {got}")]
    EmbeddingDimension {
        expected: usize,
        got: usize,
    },

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Storage schema error in {store}: {message}")]
    StorageSchema {
        store: String,
        message: String,
    },

    #[error("Training configuration error for {parameter}: {message}")]
    TrainingConfig {
        parameter: String,
        message: String,
    },

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl FrameworkError {
    /// Create a provider authentication error
    pub fn provider_auth(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ProviderAuth { provider: provider.into(), message: message.into() }
    }

    /// Create a provider model error
    pub fn provider_model(provider: impl Into<String>, model: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ProviderModel { provider: provider.into(), model: model.into(), message: message.into() }
    }

    /// Create an embedding dimension mismatch error
    pub fn embedding_dimension(expected: usize, got: usize) -> Self {
        Self::EmbeddingDimension { expected, got }
    }

    /// Create a storage schema error
    pub fn storage_schema(store: impl Into<String>, message: impl Into<String>) -> Self {
        Self::StorageSchema { store: store.into(), message: message.into() }
    }

    /// Create a training configuration error
    pub fn training_config(parameter: impl Into<String>, message: impl Into<String>) -> Self {
        Self::TrainingConfig { parameter: parameter.into(), message: message.into() }
    }
}

/// Result type alias using FrameworkError
pub type FrameworkResult<T> = Result<T, FrameworkError>;
