//! BrainClaw configuration — TOML-based with sensible defaults.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use brainwires_gateway::config::GatewayConfig;
use serde::{Deserialize, Serialize};

/// Top-level BrainClaw configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrainClawConfig {
    /// Gateway (WebSocket server) settings.
    pub gateway: GatewaySection,
    /// AI provider settings.
    pub provider: ProviderSection,
    /// Agent behaviour settings.
    pub agent: AgentSection,
    /// Tool availability settings.
    pub tools: ToolsSection,
    /// Persona / system prompt settings.
    pub persona: PersonaSection,
    /// Conversation memory settings.
    pub memory: MemorySection,
    /// Skill system settings.
    pub skills: SkillsSection,
    /// Security settings.
    pub security: SecuritySection,
}

// ── Section structs ─────────────────────────────────────────────────────

/// Gateway (WebSocket server) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GatewaySection {
    /// Host address to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// Maximum number of concurrent channel connections.
    pub max_connections: usize,
    /// Session inactivity timeout in seconds.
    pub session_timeout_secs: u64,
    /// Allowed API tokens for channel connections (empty = open mode).
    pub auth_tokens: Vec<String>,
}

/// AI provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ProviderSection {
    /// Default provider name (anthropic, openai, google, groq, ollama, etc.).
    pub default_provider: String,
    /// Default model name (None = use provider default).
    pub default_model: Option<String>,
    /// API key (if set directly in config).
    pub api_key: Option<String>,
    /// Environment variable name to read the API key from.
    pub api_key_env: Option<String>,
    /// Sampling temperature (0.0 - 1.0).
    pub temperature: f32,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
}

/// Agent behaviour configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentSection {
    /// Maximum tool-call rounds per message.
    pub max_tool_rounds: usize,
    /// Maximum concurrent agent sessions.
    pub max_concurrent_sessions: usize,
    /// Session idle timeout in seconds.
    pub session_idle_timeout_secs: u64,
}

/// Tool availability configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolsSection {
    /// List of enabled tool groups.
    pub enabled: Vec<String>,
    /// List of explicitly disabled tool groups (overrides enabled).
    pub disabled: Vec<String>,
    /// Whether the bash/shell tool is allowed.
    pub bash_allowed: bool,
}

/// Persona / system prompt configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PersonaSection {
    /// Name of the assistant persona.
    pub name: String,
    /// Inline system prompt.
    pub system_prompt: Option<String>,
    /// Path to a file containing the system prompt.
    pub system_prompt_file: Option<String>,
}

/// Conversation memory configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemorySection {
    /// Whether conversation memory is enabled.
    pub enabled: bool,
    /// Directory for memory storage.
    pub storage_dir: String,
    /// Maximum history messages to keep per session.
    pub max_history_messages: usize,
    /// Whether to persist conversations across restarts (JSON file store).
    pub persist_conversations: bool,
}

/// Skill system configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SkillsSection {
    /// Whether the skill system is enabled.
    pub enabled: bool,
    /// Directories to scan for SKILL.md files.
    pub directories: Vec<String>,
}

/// Security configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecuritySection {
    /// Allowed WebSocket origins (empty = allow all).
    pub allowed_origins: Vec<String>,
    /// Strip system-message spoofing from inbound messages.
    pub strip_system_spoofing: bool,
    /// Redact secret patterns in outbound messages.
    pub redact_secrets_in_output: bool,
    /// Maximum messages per minute per user.
    pub max_messages_per_minute: u32,
    /// Maximum tool calls per minute per user.
    pub max_tool_calls_per_minute: u32,
    /// Require cryptographic signatures on skill packages.
    pub require_signed_skills: bool,
}

// ── Defaults ────────────────────────────────────────────────────────────

impl Default for BrainClawConfig {
    fn default() -> Self {
        Self {
            gateway: GatewaySection::default(),
            provider: ProviderSection::default(),
            agent: AgentSection::default(),
            tools: ToolsSection::default(),
            persona: PersonaSection::default(),
            memory: MemorySection::default(),
            skills: SkillsSection::default(),
            security: SecuritySection::default(),
        }
    }
}

impl Default for GatewaySection {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 18789,
            max_connections: 256,
            session_timeout_secs: 3600,
            auth_tokens: Vec::new(),
        }
    }
}

impl Default for ProviderSection {
    fn default() -> Self {
        Self {
            default_provider: "anthropic".to_string(),
            default_model: None,
            api_key: None,
            api_key_env: None,
            temperature: 0.7,
            max_tokens: 16384,
        }
    }
}

impl Default for AgentSection {
    fn default() -> Self {
        Self {
            max_tool_rounds: 10,
            max_concurrent_sessions: 50,
            session_idle_timeout_secs: 1800,
        }
    }
}

impl Default for ToolsSection {
    fn default() -> Self {
        Self {
            enabled: vec![
                "bash".to_string(),
                "files".to_string(),
                "git".to_string(),
                "search".to_string(),
                "web".to_string(),
                "validation".to_string(),
            ],
            disabled: Vec::new(),
            bash_allowed: true,
        }
    }
}

impl Default for PersonaSection {
    fn default() -> Self {
        Self {
            name: "BrainClaw".to_string(),
            system_prompt: None,
            system_prompt_file: None,
        }
    }
}

impl Default for MemorySection {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_dir: "~/.brainclaw/memory".to_string(),
            max_history_messages: 100,
            persist_conversations: true,
        }
    }
}

impl Default for SkillsSection {
    fn default() -> Self {
        Self {
            enabled: false,
            directories: Vec::new(),
        }
    }
}

impl Default for SecuritySection {
    fn default() -> Self {
        Self {
            allowed_origins: Vec::new(),
            strip_system_spoofing: true,
            redact_secrets_in_output: true,
            max_messages_per_minute: 20,
            max_tool_calls_per_minute: 30,
            require_signed_skills: false,
        }
    }
}

// ── Methods ─────────────────────────────────────────────────────────────

impl BrainClawConfig {
    /// Load configuration from a TOML file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        Self::from_toml_str(&content)
    }

    /// Parse configuration from a TOML string.
    pub fn from_toml_str(s: &str) -> Result<Self> {
        toml::from_str(s).context("Failed to parse BrainClaw config")
    }

    /// Try to load configuration from default locations, falling back to defaults.
    ///
    /// Search order:
    /// 1. `~/.brainclaw/brainclaw.toml`
    /// 2. `./brainclaw.toml`
    /// 3. Built-in defaults
    pub fn load_or_default() -> Result<Self> {
        // Try ~/.brainclaw/brainclaw.toml
        if let Some(home_dir) = dirs::home_dir() {
            let home_config = home_dir.join(".brainclaw").join("brainclaw.toml");
            if home_config.exists() {
                tracing::info!(path = %home_config.display(), "Loading config from home directory");
                return Self::load(&home_config);
            }
        }

        // Try ./brainclaw.toml
        let local_config = PathBuf::from("brainclaw.toml");
        if local_config.exists() {
            tracing::info!("Loading config from ./brainclaw.toml");
            return Self::load(&local_config);
        }

        tracing::info!("No config file found, using defaults");
        Ok(Self::default())
    }

    /// Validate the configuration for internal consistency.
    pub fn validate(&self) -> Result<()> {
        // Validate provider name is recognized
        use brainwires_providers::ProviderType;
        let _provider_type: ProviderType = self
            .provider
            .default_provider
            .parse()
            .map_err(|_| anyhow::anyhow!(
                "Unknown provider: '{}'. Valid providers: anthropic, openai, google, groq, ollama, \
                 brainwires, together, fireworks, anyscale, bedrock, vertex-ai",
                self.provider.default_provider
            ))?;

        // Validate temperature range
        if !(0.0..=2.0).contains(&self.provider.temperature) {
            bail!(
                "Temperature must be between 0.0 and 2.0, got {}",
                self.provider.temperature
            );
        }

        // Validate max_tokens is reasonable
        if self.provider.max_tokens == 0 {
            bail!("max_tokens must be greater than 0");
        }

        // Validate port
        if self.gateway.port == 0 {
            bail!("Gateway port must be greater than 0");
        }

        // Validate max_tool_rounds
        if self.agent.max_tool_rounds == 0 {
            bail!("max_tool_rounds must be greater than 0");
        }

        Ok(())
    }

    /// Convert to a [`GatewayConfig`] for the gateway server.
    pub fn to_gateway_config(&self) -> GatewayConfig {
        GatewayConfig {
            host: self.gateway.host.clone(),
            port: self.gateway.port,
            max_connections: self.gateway.max_connections,
            session_timeout: Duration::from_secs(self.gateway.session_timeout_secs),
            auth_tokens: self.gateway.auth_tokens.clone(),
            webhook_enabled: true,
            webhook_path: "/webhook".to_string(),
            admin_enabled: true,
            admin_path: "/admin".to_string(),
            allowed_origins: self.security.allowed_origins.clone(),
            strip_system_spoofing: self.security.strip_system_spoofing,
            redact_secrets_in_output: self.security.redact_secrets_in_output,
            max_messages_per_minute: self.security.max_messages_per_minute,
            max_tool_calls_per_minute: self.security.max_tool_calls_per_minute,
            webchat_enabled: true,
            max_attachment_size_mb: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_expected_values() {
        let config = BrainClawConfig::default();
        assert_eq!(config.gateway.host, "127.0.0.1");
        assert_eq!(config.gateway.port, 18789);
        assert_eq!(config.gateway.max_connections, 256);
        assert_eq!(config.provider.default_provider, "anthropic");
        assert!(config.provider.default_model.is_none());
        assert!(config.provider.api_key.is_none());
        assert_eq!(config.provider.temperature, 0.7);
        assert_eq!(config.provider.max_tokens, 16384);
        assert_eq!(config.agent.max_tool_rounds, 10);
        assert_eq!(config.agent.max_concurrent_sessions, 50);
        assert_eq!(config.tools.enabled.len(), 6);
        assert!(config.tools.bash_allowed);
        assert_eq!(config.persona.name, "BrainClaw");
        assert!(config.persona.system_prompt.is_none());
        assert!(config.memory.enabled);
        assert_eq!(config.memory.max_history_messages, 100);
        assert!(!config.skills.enabled);
        assert!(config.security.strip_system_spoofing);
        assert!(config.security.redact_secrets_in_output);
        assert_eq!(config.security.max_messages_per_minute, 20);
        assert_eq!(config.security.max_tool_calls_per_minute, 30);
        assert!(!config.security.require_signed_skills);
    }

    #[test]
    fn test_load_from_toml_string() {
        let toml_str = r#"
[gateway]
host = "0.0.0.0"
port = 9090

[provider]
default_provider = "openai"
default_model = "gpt-4o"
temperature = 0.5
max_tokens = 8192

[agent]
max_tool_rounds = 5

[tools]
enabled = ["bash", "files"]
bash_allowed = false

[persona]
name = "TestBot"
system_prompt = "You are a test bot."

[memory]
enabled = false
max_history_messages = 50

[skills]
enabled = true
directories = ["/home/user/skills"]

[security]
allowed_origins = ["https://example.com"]
max_messages_per_minute = 10
require_signed_skills = true
"#;

        let config = BrainClawConfig::from_toml_str(toml_str).unwrap();
        assert_eq!(config.gateway.host, "0.0.0.0");
        assert_eq!(config.gateway.port, 9090);
        assert_eq!(config.provider.default_provider, "openai");
        assert_eq!(config.provider.default_model.as_deref(), Some("gpt-4o"));
        assert_eq!(config.provider.temperature, 0.5);
        assert_eq!(config.provider.max_tokens, 8192);
        assert_eq!(config.agent.max_tool_rounds, 5);
        assert_eq!(config.tools.enabled, vec!["bash", "files"]);
        assert!(!config.tools.bash_allowed);
        assert_eq!(config.persona.name, "TestBot");
        assert_eq!(
            config.persona.system_prompt.as_deref(),
            Some("You are a test bot.")
        );
        assert!(!config.memory.enabled);
        assert_eq!(config.memory.max_history_messages, 50);
        assert!(config.skills.enabled);
        assert_eq!(config.skills.directories, vec!["/home/user/skills"]);
        assert_eq!(
            config.security.allowed_origins,
            vec!["https://example.com"]
        );
        assert_eq!(config.security.max_messages_per_minute, 10);
        assert!(config.security.require_signed_skills);
    }

    #[test]
    fn test_partial_toml_uses_defaults() {
        let toml_str = r#"
[provider]
default_provider = "groq"
"#;

        let config = BrainClawConfig::from_toml_str(toml_str).unwrap();
        assert_eq!(config.provider.default_provider, "groq");
        // Everything else should be defaults
        assert_eq!(config.gateway.host, "127.0.0.1");
        assert_eq!(config.gateway.port, 18789);
        assert_eq!(config.agent.max_tool_rounds, 10);
        assert_eq!(config.persona.name, "BrainClaw");
    }

    #[test]
    fn test_validate_valid_config() {
        let config = BrainClawConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_unknown_provider() {
        let mut config = BrainClawConfig::default();
        config.provider.default_provider = "nonexistent".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown provider"));
    }

    #[test]
    fn test_validate_bad_temperature() {
        let mut config = BrainClawConfig::default();
        config.provider.temperature = 3.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_max_tokens() {
        let mut config = BrainClawConfig::default();
        config.provider.max_tokens = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_port() {
        let mut config = BrainClawConfig::default();
        config.gateway.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_zero_tool_rounds() {
        let mut config = BrainClawConfig::default();
        config.agent.max_tool_rounds = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_to_gateway_config() {
        let mut config = BrainClawConfig::default();
        config.gateway.host = "0.0.0.0".to_string();
        config.gateway.port = 9999;
        config.gateway.max_connections = 128;
        config.gateway.session_timeout_secs = 7200;
        config.security.allowed_origins = vec!["https://example.com".to_string()];
        config.security.max_messages_per_minute = 15;

        let gw = config.to_gateway_config();
        assert_eq!(gw.host, "0.0.0.0");
        assert_eq!(gw.port, 9999);
        assert_eq!(gw.max_connections, 128);
        assert_eq!(gw.session_timeout, Duration::from_secs(7200));
        assert_eq!(gw.allowed_origins, vec!["https://example.com"]);
        assert_eq!(gw.max_messages_per_minute, 15);
        assert!(gw.strip_system_spoofing);
        assert!(gw.redact_secrets_in_output);
    }

    #[test]
    fn test_empty_toml_uses_all_defaults() {
        let config = BrainClawConfig::from_toml_str("").unwrap();
        let default = BrainClawConfig::default();
        assert_eq!(config.gateway.host, default.gateway.host);
        assert_eq!(config.gateway.port, default.gateway.port);
        assert_eq!(
            config.provider.default_provider,
            default.provider.default_provider
        );
        assert_eq!(config.persona.name, default.persona.name);
    }
}
