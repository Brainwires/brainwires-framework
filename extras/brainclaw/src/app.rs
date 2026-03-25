//! BrainClaw application — wires everything together and runs the daemon.

use std::sync::Arc;

use anyhow::{Result, bail};

use brainwires_core::{ChatOptions, ToolContext};
use brainwires_gateway::agent_handler::AgentInboundHandler;
use brainwires_gateway::channel_registry::ChannelRegistry;
use brainwires_gateway::server::Gateway;
use brainwires_gateway::session::SessionManager;
use brainwires_providers::{ChatProviderFactory, ProviderConfig, ProviderType};
use brainwires_tool_system::BuiltinToolExecutor;

use crate::config::BrainClawConfig;
use crate::persona::Persona;
use crate::tools::build_tool_registry;

/// The BrainClaw daemon.
pub struct BrainClaw {
    config: BrainClawConfig,
}

impl BrainClaw {
    /// Create a new BrainClaw instance with the given configuration.
    pub fn new(config: BrainClawConfig) -> Self {
        Self { config }
    }

    /// Run the BrainClaw daemon.
    ///
    /// This method blocks until the server is shut down (via SIGINT/SIGTERM).
    pub async fn run(&self) -> Result<()> {
        tracing::info!(
            provider = %self.config.provider.default_provider,
            persona = %self.config.persona.name,
            "Starting BrainClaw daemon"
        );

        // 1. Resolve API key
        let api_key = self.resolve_api_key()?;

        // 2. Create provider
        let provider_type: ProviderType = self.config.provider.default_provider.parse()?;
        let model = self
            .config
            .provider
            .default_model
            .clone()
            .unwrap_or_else(|| provider_type.default_model().to_string());

        let mut prov_config = ProviderConfig::new(provider_type, model.clone());
        if let Some(key) = api_key {
            prov_config = prov_config.with_api_key(key);
        } else if provider_type.requires_api_key() {
            bail!(
                "No API key found for provider '{}'.\n\
                 Set it via:\n  \
                 - Config file: provider.api_key or provider.api_key_env\n  \
                 - Environment variable (e.g. ANTHROPIC_API_KEY)\n  \
                 - CLI flag: --api-key",
                self.config.provider.default_provider
            );
        }

        let provider = ChatProviderFactory::create(&prov_config)?;

        tracing::info!(
            provider = %provider_type,
            model = %model,
            "Provider initialized"
        );

        // 3. Build tool registry
        let registry = build_tool_registry(&self.config.tools);
        let tool_count = registry.len();
        let context = ToolContext::default();
        let executor = Arc::new(BuiltinToolExecutor::new(registry, context));

        tracing::info!(tools = tool_count, "Tool registry built");

        // 4. Build ChatOptions with system prompt from persona
        let persona = Persona::from_config(&self.config.persona)?;
        let options = ChatOptions {
            temperature: Some(self.config.provider.temperature),
            max_tokens: Some(self.config.provider.max_tokens),
            system: Some(persona.system_prompt.clone()),
            ..Default::default()
        };

        tracing::info!(persona = %persona.name, "Persona loaded");

        // 5. Build GatewayConfig
        let gateway_config = self.config.to_gateway_config();

        // 6. Create session manager and channel registry
        let sessions = Arc::new(SessionManager::new());
        let channels = Arc::new(ChannelRegistry::new());

        // 7. Create AgentInboundHandler
        let handler = AgentInboundHandler::new(
            sessions,
            channels,
            provider,
            executor,
            options,
        )
        .with_max_tool_rounds(self.config.agent.max_tool_rounds);

        // 8. Create Gateway with handler
        let gateway = Gateway::with_handler(gateway_config.clone(), Arc::new(handler));

        tracing::info!(
            address = %gateway_config.bind_address(),
            "BrainClaw ready"
        );

        // 9. Run gateway (blocks until shutdown)
        gateway.run().await
    }

    /// Resolve the API key from config, environment variable, or standard env vars.
    fn resolve_api_key(&self) -> Result<Option<String>> {
        // 1. Direct config value
        if let Some(ref key) = self.config.provider.api_key {
            if !key.is_empty() {
                return Ok(Some(key.clone()));
            }
        }

        // 2. Custom env var name from config
        if let Some(ref env_name) = self.config.provider.api_key_env {
            if let Ok(key) = std::env::var(env_name) {
                if !key.is_empty() {
                    return Ok(Some(key));
                }
            }
        }

        // 3. Standard env vars based on provider
        let env_var = match self.config.provider.default_provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "openai" | "openai-responses" | "openai_responses" => "OPENAI_API_KEY",
            "google" | "gemini" => "GOOGLE_API_KEY",
            "groq" => "GROQ_API_KEY",
            "together" => "TOGETHER_API_KEY",
            "fireworks" => "FIREWORKS_API_KEY",
            "anyscale" => "ANYSCALE_API_KEY",
            "brainwires" => "BRAINWIRES_API_KEY",
            "elevenlabs" => "ELEVENLABS_API_KEY",
            "deepgram" => "DEEPGRAM_API_KEY",
            _ => "",
        };

        if !env_var.is_empty() {
            if let Ok(key) = std::env::var(env_var) {
                if !key.is_empty() {
                    return Ok(Some(key));
                }
            }
        }

        Ok(None)
    }
}
