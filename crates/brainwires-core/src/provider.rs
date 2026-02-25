use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::message::{ChatResponse, Message, StreamChunk};
use crate::tool::Tool;

/// Base provider trait for AI providers
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the model's maximum output tokens (for setting appropriate limits)
    /// Returns None if the model doesn't have a specific limit
    fn max_output_tokens(&self) -> Option<u32> {
        None // Default implementation - providers can override
    }

    /// Chat completion (non-streaming)
    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> Result<ChatResponse>;

    /// Chat completion (streaming)
    fn stream_chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: Option<&'a [Tool]>,
        options: &'a ChatOptions,
    ) -> BoxStream<'a, Result<StreamChunk>>;
}

/// Chat completion options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatOptions {
    /// Temperature (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

impl Default for ChatOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            max_tokens: Some(4096),
            top_p: None,
            stop: None,
            system: None,
        }
    }
}

impl ChatOptions {
    /// Create new chat options with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set system prompt
    pub fn system<S: Into<String>>(mut self, system: S) -> Self {
        self.system = Some(system.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_options_default() {
        let opts = ChatOptions::default();
        assert_eq!(opts.temperature, Some(0.7));
        assert_eq!(opts.max_tokens, Some(4096));
    }

    #[test]
    fn test_chat_options_builder() {
        let opts = ChatOptions::new()
            .temperature(0.5)
            .max_tokens(2048)
            .system("Test");
        assert_eq!(opts.temperature, Some(0.5));
        assert_eq!(opts.max_tokens, Some(2048));
        assert_eq!(opts.system, Some("Test".to_string()));
    }
}
