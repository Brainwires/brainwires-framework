//! Model listing and validation service for multi-provider support.
//!
//! Wraps the framework [`ModelLister`] trait with per-provider JSON caching
//! and error-resilient fallback behaviour.

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::providers::{create_model_lister, AvailableModel, ModelCapability, ProviderType};
use crate::utils::paths::PlatformPaths;

/// Cached model list with timestamp.
#[derive(Debug, Serialize, Deserialize)]
struct ProviderModelCache {
    models: Vec<AvailableModel>,
    cached_at: DateTime<Utc>,
}

impl ProviderModelCache {
    /// Whether the cache is still fresh for the given provider.
    fn is_valid(&self, provider: ProviderType) -> bool {
        let ttl = cache_ttl(provider);
        let age = Utc::now().signed_duration_since(self.cached_at);
        age < ttl
    }
}

/// Return the cache time-to-live for a provider.
fn cache_ttl(provider: ProviderType) -> Duration {
    match provider {
        ProviderType::Ollama => Duration::minutes(5),
        ProviderType::Brainwires => Duration::hours(24),
        _ => Duration::hours(12), // cloud providers
    }
}

/// Directory for per-provider cache files.
fn cache_dir() -> Result<PathBuf> {
    let dir = PlatformPaths::brainwires_data_dir()?.join("provider_models_cache");
    if !dir.exists() {
        fs::create_dir_all(&dir).context("Failed to create provider model cache directory")?;
    }
    Ok(dir)
}

/// Cache file path for a specific provider.
fn cache_path(provider: ProviderType) -> Result<PathBuf> {
    Ok(cache_dir()?.join(format!("{}.json", provider.as_str())))
}

/// Load cached models for a provider, if present and fresh.
fn load_cache(provider: ProviderType) -> Option<Vec<AvailableModel>> {
    let path = cache_path(provider).ok()?;
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    let cache: ProviderModelCache = serde_json::from_str(&content).ok()?;
    if cache.is_valid(provider) {
        Some(cache.models)
    } else {
        None
    }
}

/// Load expired cache as a fallback (network failure).
fn load_expired_cache(provider: ProviderType) -> Option<Vec<AvailableModel>> {
    let path = cache_path(provider).ok()?;
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    let cache: ProviderModelCache = serde_json::from_str(&content).ok()?;
    Some(cache.models)
}

/// Save models to cache.
fn save_cache(provider: ProviderType, models: &[AvailableModel]) -> Result<()> {
    let cache = ProviderModelCache {
        models: models.to_vec(),
        cached_at: Utc::now(),
    };
    let path = cache_path(provider)?;
    let content = serde_json::to_string_pretty(&cache)?;
    fs::write(&path, content).context("Failed to write provider model cache")?;
    Ok(())
}

/// Service for listing and validating models across providers.
pub struct ModelService;

impl ModelService {
    /// List models for a specific provider, given explicit credentials.
    ///
    /// Used during `auth login --provider` and by `models list --provider`.
    pub async fn list_models_for_provider(
        provider_type: ProviderType,
        api_key: Option<&str>,
        base_url: Option<&str>,
        use_cache: bool,
    ) -> Result<Vec<AvailableModel>> {
        // Try cache first
        if use_cache {
            if let Some(cached) = load_cache(provider_type) {
                return Ok(cached);
            }
        }

        // Fetch from provider API
        let lister = create_model_lister(provider_type, api_key, base_url)?;
        match lister.list_models().await {
            Ok(models) => {
                let _ = save_cache(provider_type, &models);
                Ok(models)
            }
            Err(e) => {
                // On network failure, fall back to expired cache
                if let Some(stale) = load_expired_cache(provider_type) {
                    tracing::warn!(
                        "Failed to fetch {} models ({}), using cached data",
                        provider_type,
                        e
                    );
                    Ok(stale)
                } else {
                    Err(e).context(format!(
                        "Failed to list {} models (no cache available)",
                        provider_type
                    ))
                }
            }
        }
    }

    /// List models for the currently configured provider.
    pub async fn list_models(use_cache: bool) -> Result<Vec<AvailableModel>> {
        let config_manager = crate::config::ConfigManager::new()?;
        let config = config_manager.get();
        let provider_type = config.provider_type;

        if provider_type == ProviderType::Brainwires {
            return Err(anyhow::anyhow!(
                "Use 'brainwires models list' without --provider for Brainwires SaaS models"
            ));
        }

        let api_key = config_manager.get_provider_api_key()?.map(|z| z.to_string());
        let base_url = config.provider_base_url.clone();

        Self::list_models_for_provider(
            provider_type,
            api_key.as_deref(),
            base_url.as_deref(),
            use_cache,
        )
        .await
    }

    /// List only chat-capable models for the active provider.
    pub async fn list_chat_models(use_cache: bool) -> Result<Vec<AvailableModel>> {
        let models = Self::list_models(use_cache).await?;
        Ok(models
            .into_iter()
            .filter(|m| m.is_chat_capable())
            .collect())
    }

    /// List chat-capable models for a specific provider.
    pub async fn list_chat_models_for_provider(
        provider_type: ProviderType,
        api_key: Option<&str>,
        base_url: Option<&str>,
        use_cache: bool,
    ) -> Result<Vec<AvailableModel>> {
        let models =
            Self::list_models_for_provider(provider_type, api_key, base_url, use_cache).await?;
        Ok(models
            .into_iter()
            .filter(|m| m.is_chat_capable())
            .collect())
    }

    /// Validate that a model ID exists at the given provider.
    ///
    /// Returns the model on success, or an error with suggestions on failure.
    pub async fn validate_model(
        provider_type: ProviderType,
        model_id: &str,
        api_key: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<AvailableModel> {
        let models =
            Self::list_models_for_provider(provider_type, api_key, base_url, true).await?;

        // Exact match
        if let Some(model) = models.iter().find(|m| m.id == model_id) {
            return Ok(model.clone());
        }

        // Find close matches for suggestions
        let suggestions: Vec<&str> = models
            .iter()
            .filter(|m| {
                m.id.contains(model_id)
                    || model_id.contains(&m.id)
                    || levenshtein_close(&m.id, model_id)
            })
            .take(5)
            .map(|m| m.id.as_str())
            .collect();

        let mut msg = format!(
            "Model '{}' not found at {} provider",
            model_id, provider_type
        );
        if !suggestions.is_empty() {
            msg.push_str(&format!("\n\nDid you mean one of these?\n"));
            for s in &suggestions {
                msg.push_str(&format!("  - {}\n", s));
            }
        }

        Err(anyhow::anyhow!(msg))
    }
}

/// Simple distance check: true if strings differ by at most 3 characters.
fn levenshtein_close(a: &str, b: &str) -> bool {
    if a.len().abs_diff(b.len()) > 3 {
        return false;
    }
    let (short, long) = if a.len() <= b.len() {
        (a.as_bytes(), b.as_bytes())
    } else {
        (b.as_bytes(), a.as_bytes())
    };

    // Simple bounded edit distance
    let mut prev: Vec<usize> = (0..=short.len()).collect();
    let mut curr = vec![0usize; short.len() + 1];

    for i in 1..=long.len() {
        curr[0] = i;
        for j in 1..=short.len() {
            let cost = if long[i - 1] == short[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[short.len()] <= 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_ttl() {
        assert_eq!(cache_ttl(ProviderType::Ollama), Duration::minutes(5));
        assert_eq!(cache_ttl(ProviderType::Anthropic), Duration::hours(12));
        assert_eq!(cache_ttl(ProviderType::OpenAI), Duration::hours(12));
        assert_eq!(cache_ttl(ProviderType::Brainwires), Duration::hours(24));
    }

    #[test]
    fn test_cache_validity() {
        let fresh = ProviderModelCache {
            models: vec![],
            cached_at: Utc::now(),
        };
        assert!(fresh.is_valid(ProviderType::Anthropic));
        assert!(fresh.is_valid(ProviderType::Ollama));

        let old = ProviderModelCache {
            models: vec![],
            cached_at: Utc::now() - Duration::hours(13),
        };
        assert!(!old.is_valid(ProviderType::Anthropic));
        assert!(!old.is_valid(ProviderType::Ollama));

        // 24h provider still valid at 13h
        let brainwires_cache = ProviderModelCache {
            models: vec![],
            cached_at: Utc::now() - Duration::hours(13),
        };
        assert!(brainwires_cache.is_valid(ProviderType::Brainwires));
    }

    #[test]
    fn test_levenshtein_close() {
        assert!(levenshtein_close("gpt-4o", "gpt-4o"));
        assert!(levenshtein_close("gpt-4o", "gpt-4"));
        assert!(levenshtein_close("claude-3", "claude-4"));
        assert!(!levenshtein_close("gpt-4o", "completely-different-model"));
    }

    #[test]
    fn test_model_cache_serialization() {
        let cache = ProviderModelCache {
            models: vec![AvailableModel {
                id: "test-model".to_string(),
                display_name: Some("Test".to_string()),
                provider: ProviderType::Anthropic,
                capabilities: vec![ModelCapability::Chat],
                owned_by: None,
                context_window: Some(200000),
                max_output_tokens: Some(4096),
                created_at: None,
            }],
            cached_at: Utc::now(),
        };

        let json = serde_json::to_string(&cache).unwrap();
        let parsed: ProviderModelCache = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.models[0].id, "test-model");
    }
}
