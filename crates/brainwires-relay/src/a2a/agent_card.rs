//! Agent Card — discovery metadata for A2A agents.
//!
//! An [`AgentCard`] is a JSON document published at a well-known URL
//! (`/.well-known/agent.json`) that describes an agent's identity,
//! capabilities, and skills so that other agents (or clients) can
//! discover and interact with it.

use serde::{Deserialize, Serialize};

/// Describes an agent's identity, endpoint, capabilities, and skills.
///
/// Published at `/.well-known/agent.json` for discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    /// Human-readable display name of the agent.
    pub name: String,

    /// Human-readable description of what the agent does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The base URL where this agent's A2A endpoint is hosted.
    pub url: String,

    /// Version string for this agent card (e.g. `"1.0.0"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Protocol-level capabilities supported by this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<AgentCapabilities>,

    /// The skills this agent can perform.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<AgentSkill>,

    /// Default MIME types this agent accepts as input (e.g. `["text/plain"]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_input_modes: Vec<String>,

    /// Default MIME types this agent produces as output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_output_modes: Vec<String>,

    /// Information about the organization or individual providing this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<AgentProvider>,
}

/// Protocol-level capabilities advertised by an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    /// Whether the agent supports streaming responses via SSE.
    #[serde(default)]
    pub streaming: bool,

    /// Whether the agent supports push notifications for task updates.
    #[serde(default)]
    pub push_notifications: bool,

    /// Whether the agent supports returning state transition history on tasks.
    #[serde(default)]
    pub state_transition_history: bool,
}

/// A discrete skill or capability that an agent can perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    /// Unique identifier for this skill.
    pub id: String,

    /// Human-readable name of the skill.
    pub name: String,

    /// Human-readable description of what this skill does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Tags for categorization and discovery.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Example prompts or inputs that demonstrate this skill.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,

    /// MIME types this skill accepts as input (overrides agent defaults).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_modes: Vec<String>,

    /// MIME types this skill produces as output (overrides agent defaults).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_modes: Vec<String>,
}

/// Information about the provider (organization or individual) behind an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProvider {
    /// Name of the provider organization or individual.
    pub organization: String,

    /// URL for the provider's website or documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}
