//! # Identity Layer
//!
//! Foundational identity types for the agent networking stack.
//!
//! Every agent on the network is represented by an [`AgentIdentity`] which
//! includes a unique identifier, human-readable name, and an [`AgentCard`]
//! that advertises the agent's capabilities, supported protocols, and
//! reachable endpoint.

mod agent_identity;
mod credentials;

pub use agent_identity::{AgentCard, AgentIdentity, ProtocolId};
pub use credentials::{SigningKey, VerifyingKey};
