//! # Network Core Types
//!
//! Shared types used across all networking layers: message envelopes,
//! network events, error types, and the application-layer
//! [`NetworkManager`].

pub(crate) mod envelope;
mod error;
pub(crate) mod event;
mod manager;

pub use envelope::{MessageEnvelope, MessageTarget, Payload};
pub use error::NetworkError;
pub use event::{ConnectionState, NetworkEvent, TransportType};
pub use manager::{NetworkManager, NetworkManagerBuilder};
