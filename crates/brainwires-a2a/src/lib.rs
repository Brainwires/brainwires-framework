//! # Brainwires A2A
//!
//! Implementation of Google's [Agent-to-Agent (A2A) protocol](https://google.github.io/A2A/)
//! for the Brainwires Agent Framework.
//!
//! A2A enables interoperable communication between AI agents regardless of
//! their underlying framework or vendor. This crate provides:
//!
//! - **Agent Cards**: Discovery metadata describing an agent's capabilities and skills
//! - **Task lifecycle**: Submission, execution tracking, and artifact delivery
//! - **Message types**: Text, file, and structured data parts
//! - **Authentication**: Pluggable auth schemes (API key, OAuth2, JWT, Bearer)
//! - **Transport** (planned): HTTP + SSE transport with JSON-RPC 2.0 envelopes
//!
//! ## Feature flags
//!
//! | Flag     | Default | Description                              |
//! |----------|---------|------------------------------------------|
//! | `server` | yes     | A2A server (agent host) support          |
//! | `client` | yes     | A2A client (agent consumer) with reqwest |

pub mod agent_card;
pub mod auth;
pub mod error;
pub mod task;
pub mod transport;
pub mod types;

// Re-export core types at crate root for convenience.
pub use agent_card::{AgentCapabilities, AgentCard, AgentProvider, AgentSkill};
pub use auth::{AuthConfig, AuthScheme};
pub use error::A2aError;
pub use task::{Task, TaskQueryParams, TaskSendParams, TaskState};
pub use types::{Artifact, Message, Part};
