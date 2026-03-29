//! BrainClaw application — wires everything together and runs the daemon.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Result, bail};

use brainwires_core::{ChatOptions, ToolContext};
use brainwires_gateway::agent_handler::AgentInboundHandler;
use brainwires_gateway::channel_registry::ChannelRegistry;
use brainwires_gateway::media::MediaProcessor;
use brainwires_gateway::server::Gateway;
use brainwires_gateway::session::SessionManager;
use brainwires_gateway::session_persistence::{JsonFileStore, expand_tilde};
use brainwires_gateway::middleware::rate_limit::RateLimiter;
use brainwires_gateway::middleware::sanitizer::MessageSanitizer;
use brainwires_providers::{ChatProviderFactory, ProviderConfig, ProviderType};
use brainwires_tool_system::BuiltinToolExecutor;

use crate::config::BrainClawConfig;
use crate::persona::Persona;
use crate::skill_handler::SkillHandler;
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

        // 3b. Build ToolContext with provider configs embedded as metadata
        let mut context = ToolContext::default();
        self.inject_tool_configs(&mut context);

        let executor = Arc::new(BuiltinToolExecutor::new(registry, context));

        tracing::info!(tools = tool_count, "Tool registry built");

        // 4. Build ChatOptions with system prompt from persona + context files
        let persona = Persona::from_config(&self.config.persona)?;
        let context_text = load_context_files(&self.config.persona.context_files);
        let system_prompt = if context_text.is_empty() {
            persona.system_prompt.clone()
        } else {
            format!("{}\n\n---\n\n{}", persona.system_prompt, context_text)
        };

        let options = ChatOptions {
            temperature: Some(self.config.provider.temperature),
            max_tokens: Some(self.config.provider.max_tokens),
            system: Some(system_prompt),
            ..Default::default()
        };

        tracing::info!(persona = %persona.name, "Persona loaded");

        // 5. Build GatewayConfig
        let gateway_config = self.config.to_gateway_config();

        // 6. Create session manager and channel registry
        let sessions = Arc::new(SessionManager::new());
        let channels = Arc::new(ChannelRegistry::new());

        // 7. Create AgentInboundHandler
        let mut handler = AgentInboundHandler::new(
            sessions,
            channels,
            provider,
            executor,
            options,
        )
        .with_max_tool_rounds(self.config.agent.max_tool_rounds);

        // 7b. Attach session persistence if configured
        if self.config.memory.persist_conversations {
            let storage_path = expand_tilde(&self.config.memory.storage_dir);
            match JsonFileStore::new(&storage_path) {
                Ok(store) => {
                    tracing::info!(
                        path = %storage_path.display(),
                        "Session persistence enabled (JsonFileStore)"
                    );
                    handler = handler.with_persistence(Arc::new(store));
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        path = %storage_path.display(),
                        "Failed to initialize session persistence; continuing without it"
                    );
                }
            }
        }

        // 7c. Attach message sanitizer from security config
        let sanitizer = Arc::new(MessageSanitizer::new(
            self.config.security.strip_system_spoofing,
            self.config.security.redact_secrets_in_output,
        ));
        handler = handler.with_sanitizer(sanitizer);

        // 7d. Attach rate limiter from security config
        let rate_limiter = Arc::new(RateLimiter::new(
            self.config.security.max_messages_per_minute,
            self.config.security.max_tool_calls_per_minute,
        ));
        handler = handler.with_rate_limiter(rate_limiter);

        // 7e. Attach skill handler if enabled
        if self.config.skills.enabled {
            let skill_dirs: Vec<PathBuf> = self
                .config
                .skills
                .directories
                .iter()
                .map(|d| PathBuf::from(expand_tilde_str(d)))
                .collect();

            match SkillHandler::new(&skill_dirs) {
                Ok(skill_handler) => {
                    let count = skill_handler.skill_count();
                    tracing::info!(skills = count, "Skill system enabled");
                    let sh = Arc::new(Mutex::new(skill_handler));
                    handler = handler.with_text_preprocessor(Arc::new(move |text: &str| {
                        if let Some((cmd, args)) = SkillHandler::parse_command(text) {
                            let guard = match sh.lock() {
                                Ok(g) => g,
                                Err(_) => return None,
                            };
                            match guard.resolve_command(cmd, args) {
                                Ok(Some(instructions)) => {
                                    // Prepend skill instructions; agent sees the full context
                                    Some(format!(
                                        "Execute the following skill instructions:\n\n\
                                         {instructions}\n\n\
                                         User input: {text}"
                                    ))
                                }
                                Ok(None) => {
                                    tracing::debug!(command = cmd, "No skill found for command");
                                    None
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, command = cmd, "Skill resolution error");
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    }));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to initialize skill system; continuing without it");
                }
            }
        }

        // 7f. Attach media processor (+ optional STT for voice)
        let mut media = MediaProcessor::new(10); // 10 MB attachment limit

        #[cfg(feature = "voice")]
        if let Some(ref voice_cfg) = self.config.voice {
            if let Some(stt) = build_stt_provider(voice_cfg) {
                tracing::info!(
                    provider = %voice_cfg.stt_provider,
                    "Speech-to-text enabled"
                );
                media = media.with_stt(stt);
            }
        }

        handler = handler.with_media(Arc::new(media));

        // 8. Create Gateway with handler
        let gateway = Gateway::with_handler(gateway_config.clone(), Arc::new(handler));

        tracing::info!(
            address = %gateway_config.bind_address(),
            "BrainClaw ready"
        );

        // 9. Run gateway (blocks until shutdown)
        gateway.run().await
    }

    /// Inject tool-specific configs into `ToolContext.metadata` as JSON strings.
    ///
    /// Tools read their config from metadata at call time; this avoids passing
    /// typed configs through the generic tool registry.
    fn inject_tool_configs(&self, #[cfg_attr(not(any(feature = "email", feature = "calendar")), allow(unused_variables))] context: &mut ToolContext) {
        #[cfg(feature = "email")]
        if let Some(result) = self.config.to_email_config() {
            match result {
                Ok(cfg) => match serde_json::to_string(&cfg) {
                    Ok(json) => {
                        context.metadata.insert("email_config".to_string(), json);
                        tracing::debug!("Email tool config injected into ToolContext");
                    }
                    Err(e) => tracing::warn!(error = %e, "Failed to serialize email config"),
                },
                Err(e) => tracing::warn!(
                    error = %e,
                    "Email config error; email tools will fail at call time"
                ),
            }
        }

        #[cfg(feature = "calendar")]
        if let Some(result) = self.config.to_calendar_config() {
            match result {
                Ok(cfg) => match serde_json::to_string(&cfg) {
                    Ok(json) => {
                        context.metadata.insert("calendar_config".to_string(), json);
                        tracing::debug!("Calendar tool config injected into ToolContext");
                    }
                    Err(e) => tracing::warn!(error = %e, "Failed to serialize calendar config"),
                },
                Err(e) => tracing::warn!(
                    error = %e,
                    "Calendar config error; calendar tools will fail at call time"
                ),
            }
        }

        // Inject browser config so BrowserTool can read thalora_binary / session_timeout_secs
        #[cfg(feature = "browser")]
        if let Some(ref browser_cfg) = self.config.browser {
            match serde_json::to_string(browser_cfg) {
                Ok(json) => {
                    context.metadata.insert("browser_config".to_string(), json);
                    tracing::debug!("Browser tool config injected into ToolContext");
                }
                Err(e) => tracing::warn!(error = %e, "Failed to serialize browser config"),
            }
        }
        #[cfg(not(feature = "browser"))]
        let _ = &self.config.browser;
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

/// Build an STT provider from the voice configuration.
///
/// Returns `None` with a warning if the provider is unknown or the API key is missing
/// for a provider that requires one.
#[cfg(feature = "voice")]
fn build_stt_provider(
    cfg: &crate::config::VoiceSection,
) -> Option<std::sync::Arc<dyn brainwires_hardware::SpeechToText>> {
    use brainwires_hardware::{
        AzureStt, DeepgramStt, ElevenLabsStt, FishStt, OpenAiStt, SpeechToText,
    };

    /// Resolve an API key: first from `api_key_env`, then from a named env var.
    fn resolve_key(cfg: &crate::config::VoiceSection, default_var: &str) -> Option<String> {
        let var_name = cfg
            .api_key_env
            .as_deref()
            .unwrap_or(default_var);
        std::env::var(var_name).ok().filter(|k| !k.is_empty())
    }

    match cfg.stt_provider.as_str() {
        "openai" | "openai-responses" => {
            let key = resolve_key(cfg, "OPENAI_API_KEY")?;
            Some(std::sync::Arc::new(OpenAiStt::new(key)) as Arc<dyn SpeechToText>)
        }
        "deepgram" => {
            let key = resolve_key(cfg, "DEEPGRAM_API_KEY")?;
            Some(std::sync::Arc::new(DeepgramStt::new(key)) as Arc<dyn SpeechToText>)
        }
        "elevenlabs" => {
            let key = resolve_key(cfg, "ELEVENLABS_API_KEY")?;
            Some(std::sync::Arc::new(ElevenLabsStt::new(key)) as Arc<dyn SpeechToText>)
        }
        "fish" => {
            let key = resolve_key(cfg, "FISH_API_KEY")?;
            Some(std::sync::Arc::new(FishStt::new(key)) as Arc<dyn SpeechToText>)
        }
        "azure" => {
            // Azure requires both subscription key and region.
            let key = resolve_key(cfg, "AZURE_SPEECH_KEY")?;
            let region = std::env::var("AZURE_SPEECH_REGION").ok().filter(|r| !r.is_empty())?;
            Some(std::sync::Arc::new(AzureStt::new(key, region)) as Arc<dyn SpeechToText>)
        }
        #[cfg(feature = "local-stt")]
        "whisper-local" | "whisper" => {
            Some(std::sync::Arc::new(brainwires_hardware::WhisperStt::new()) as Arc<dyn SpeechToText>)
        }
        other => {
            tracing::warn!(provider = %other, "Unknown STT provider; voice transcription disabled");
            None
        }
    }
}

/// Load context from the standard CONTEXT.md locations and any extra paths.
///
/// Checks in order:
/// 1. `~/.brainclaw/CONTEXT.md` (global user context)
/// 2. `.brainclaw/CONTEXT.md` (project-level context in the daemon's cwd)
/// 3. Any extra paths specified in `persona.context_files`
///
/// Returns all content concatenated with blank-line separators.
fn load_context_files(extra_paths: &[String]) -> String {
    let mut parts: Vec<String> = Vec::new();

    let mut candidates: Vec<PathBuf> = Vec::new();

    // Standard locations
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".brainclaw").join("CONTEXT.md"));
    }
    candidates.push(PathBuf::from(".brainclaw/CONTEXT.md"));

    // User-configured extra files
    for p in extra_paths {
        candidates.push(PathBuf::from(expand_tilde_str(p)));
    }

    for path in candidates {
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) if !content.trim().is_empty() => {
                    tracing::info!(path = %path.display(), "Loaded context file");
                    parts.push(content);
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "Failed to read context file");
                }
            }
        }
    }

    parts.join("\n\n")
}

/// Expand a leading `~` to the home directory.
fn expand_tilde_str(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    path.to_string()
}
