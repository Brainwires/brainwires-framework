use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::sync::Arc;

use super::local_llm::{LocalLlmConfig, LocalLlmProvider, LocalModelRegistry};
use super::{Provider, ProviderType};
use crate::auth::SessionManager;
use crate::config::ConfigManager;
use brainwires::providers::ChatProviderFactory;
use brainwires::providers::ProviderConfig;

/// CLI-specific provider factory.
///
/// Wraps the framework's `ProviderFactory` with session-aware key resolution
/// and config-aware provider dispatch.
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn new() -> Self {
        Self
    }

    /// Create a provider based on the active config.
    ///
    /// - `Brainwires` → uses SessionManager for API key
    /// - `Ollama` → no key needed, uses config base URL
    /// - Others → reads API key from system keyring
    pub async fn create(&self, model: String) -> Result<Arc<dyn Provider>> {
        self.create_with_backend(model, None).await
    }

    /// Create a provider with an optional backend URL override.
    pub async fn create_with_backend(
        &self,
        model: String,
        backend_url_override: Option<String>,
    ) -> Result<Arc<dyn Provider>> {
        let config_manager = ConfigManager::new()?;
        let config = config_manager.get();

        match config.provider_type {
            ProviderType::Brainwires => {
                self.create_brainwires_provider(model, backend_url_override)
                    .await
            }
            ProviderType::Ollama => {
                let base_url = backend_url_override.or_else(|| config.provider_base_url.clone());
                let mut provider_config = ProviderConfig::new(ProviderType::Ollama, model)
                    .with_base_url(
                        base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
                    );
                Self::attach_analytics(&mut provider_config);
                ChatProviderFactory::create(&provider_config)
            }
            ProviderType::Bedrock => {
                let mut provider_config = ProviderConfig::new(ProviderType::Bedrock, model);

                // Load provider options from config
                if let Some(opts) = config.extra.get("provider_options")
                    && let Some(region) = opts.get("region").and_then(|v| v.as_str())
                {
                    provider_config = provider_config.with_region(region);
                }

                Self::attach_analytics(&mut provider_config);
                ChatProviderFactory::create(&provider_config)
            }
            ProviderType::VertexAI => {
                let mut provider_config = ProviderConfig::new(ProviderType::VertexAI, model);

                // Load provider options from config
                if let Some(opts) = config.extra.get("provider_options") {
                    if let Some(project_id) = opts.get("project_id").and_then(|v| v.as_str()) {
                        provider_config = provider_config.with_project_id(project_id);
                    }
                    if let Some(region) = opts.get("region").and_then(|v| v.as_str()) {
                        provider_config = provider_config.with_region(region);
                    }
                }

                Self::attach_analytics(&mut provider_config);
                ChatProviderFactory::create(&provider_config)
            }
            provider_type => {
                // Direct providers: Anthropic, OpenAI, Google, Groq
                let api_key = config_manager.get_provider_api_key()?.ok_or_else(|| {
                    anyhow!(
                        "No API key configured for {}. Run: brainwires auth login --provider {}",
                        provider_type.as_str(),
                        provider_type.as_str()
                    )
                })?;

                let mut provider_config =
                    ProviderConfig::new(provider_type, model).with_api_key(api_key.to_string());

                if let Some(url) = backend_url_override.or_else(|| config.provider_base_url.clone())
                {
                    provider_config = provider_config.with_base_url(url);
                }

                Self::attach_analytics(&mut provider_config);
                ChatProviderFactory::create(&provider_config)
            }
        }
    }

    /// Attach the global analytics collector to a ProviderConfig.
    ///
    /// brainwires-analytics is a direct dep of brainwires-cli and brainwires-providers
    /// is built with the `analytics` feature via brainwires/full, so this is always available.
    fn attach_analytics(config: &mut ProviderConfig) {
        if let Some(collector) = crate::utils::logger::analytics_collector() {
            config.analytics_collector = Some(std::sync::Arc::new(collector));
        }
    }

    /// Create a Brainwires SaaS provider (existing flow).
    async fn create_brainwires_provider(
        &self,
        model: String,
        backend_url_override: Option<String>,
    ) -> Result<Arc<dyn Provider>> {
        if let Ok(Some(session)) = SessionManager::get_session() {
            let api_key = SessionManager::get_api_key()?.ok_or_else(|| {
                anyhow!("No API key found. Please re-authenticate with: brainwires auth")
            })?;

            let backend_url = backend_url_override.unwrap_or_else(|| session.backend.clone());

            tracing::info!(
                "Active Brainwires session found, using HTTP provider (backend: {})",
                backend_url
            );

            let provider_config = ProviderConfig::new(ProviderType::Brainwires, model)
                .with_api_key(api_key.to_string())
                .with_base_url(backend_url);

            return ChatProviderFactory::create(&provider_config);
        }

        Err(anyhow!("No active session. Run: brainwires auth login"))
    }

    /// Create a provider from session (alias for create)
    pub async fn create_from_session(&self, model: String) -> Result<Arc<dyn Provider>> {
        self.create(model).await
    }

    /// Create a local LLM provider from a model ID in the registry.
    ///
    /// Does not require an active session — runs entirely locally.
    pub fn create_local(&self, model_id: &str) -> Result<Arc<dyn Provider>> {
        let registry = LocalModelRegistry::load()?;

        let config = registry
            .get(model_id)
            .ok_or_else(|| anyhow!("Local model '{}' not found in registry", model_id))?
            .clone();

        let provider = LocalLlmProvider::new(config)?;
        Ok(Arc::new(provider))
    }

    /// Create a local LLM provider from a config directly.
    pub fn create_local_from_config(&self, config: LocalLlmConfig) -> Result<Arc<dyn Provider>> {
        let provider = LocalLlmProvider::new(config)?;
        Ok(Arc::new(provider))
    }

    /// Create a local LLM provider from a model path.
    ///
    /// Auto-detects model type from the filename.
    pub fn create_local_from_path(&self, model_path: PathBuf) -> Result<Arc<dyn Provider>> {
        let filename = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let lower = filename.to_lowercase();
        let config = if lower.contains("lfm2") || lower.contains("liquid") {
            if lower.contains("350m") || lower.contains("0.3b") {
                LocalLlmConfig::lfm2_350m(model_path)
            } else if lower.contains("1.2b") || lower.contains("1b") {
                LocalLlmConfig::lfm2_1_2b(model_path)
            } else if lower.contains("2.6b") || lower.contains("exp") {
                LocalLlmConfig::lfm2_2_6b_exp(model_path)
            } else {
                LocalLlmConfig::lfm2_1_2b(model_path)
            }
        } else if lower.contains("granite") {
            if lower.contains("350m") {
                LocalLlmConfig::granite_nano_350m(model_path)
            } else {
                LocalLlmConfig::granite_nano_1_5b(model_path)
            }
        } else {
            LocalLlmConfig {
                id: filename.to_string(),
                name: filename.to_string(),
                model_path,
                ..Default::default()
            }
        };

        self.create_local_from_config(config)
    }

    /// Get the default local provider if configured.
    pub fn get_default_local(&self) -> Result<Option<Arc<dyn Provider>>> {
        let registry = LocalModelRegistry::load()?;

        if let Some(default) = registry.get_default() {
            Ok(Some(Arc::new(LocalLlmProvider::new(default.clone())?)))
        } else {
            Ok(None)
        }
    }
}

impl Default for ProviderFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to prevent parallel tests from interfering with each other's env vars
    static TEST_ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper struct that sets up isolated session storage for the test duration.
    struct TestEnv {
        _temp_dir: TempDir,
        _guard: std::sync::MutexGuard<'static, ()>,
        original_xdg: Option<String>,
    }

    impl TestEnv {
        fn new() -> Self {
            let guard = TEST_ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let data_dir = temp_dir.path().join(".local/share");
            std::fs::create_dir_all(&data_dir).expect("Failed to create data dir");

            let original_xdg = std::env::var("XDG_DATA_HOME").ok();

            // SAFETY: We hold a mutex to prevent concurrent env var modifications in tests
            unsafe {
                std::env::set_var("XDG_DATA_HOME", &data_dir);
            }

            Self {
                _temp_dir: temp_dir,
                _guard: guard,
                original_xdg,
            }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            // SAFETY: We hold a mutex to prevent concurrent env var modifications in tests
            unsafe {
                match &self.original_xdg {
                    Some(val) => std::env::set_var("XDG_DATA_HOME", val),
                    None => std::env::remove_var("XDG_DATA_HOME"),
                }
            }
        }
    }

    #[test]
    fn test_factory_creation() {
        let factory = ProviderFactory::new();
        let _factory = factory;
    }

    #[test]
    fn test_factory_default() {
        let factory = ProviderFactory::default();
        let _factory = factory;
    }

    #[tokio::test]
    async fn test_create_without_session() {
        let _env = TestEnv::new();
        let factory = ProviderFactory::new();
        let result = factory
            .create("claude-3-5-sonnet-20241022".to_string())
            .await;

        // Should fail when no session exists (default provider is Brainwires)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_from_session_without_session() {
        let _env = TestEnv::new();
        let factory = ProviderFactory::new();
        let result = factory
            .create_from_session("claude-3-5-sonnet-20241022".to_string())
            .await;

        // Should fail when no session exists
        assert!(result.is_err());
    }
}
