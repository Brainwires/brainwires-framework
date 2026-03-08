#![deny(missing_docs)]
//! # brainwires-a2a
//!
//! Agent-to-Agent (A2A) protocol implementation with JSON-RPC, REST, and gRPC bindings.
//!
//! ## Features
//!
//! - `client` — HTTP client for JSON-RPC and REST (reqwest)
//! - `server` — HTTP server for JSON-RPC and REST (hyper)
//! - `native` — Both client and server (default)
//! - `grpc` — gRPC types (prost + tonic)
//! - `grpc-client` — gRPC client transport
//! - `grpc-server` — gRPC server service
//! - `full` — Everything

// Core types (always available)
/// Core message types: Message, Part, Artifact, FileContent, Role.
pub mod types;
/// Task lifecycle types: Task, TaskStatus, TaskState.
pub mod task;
/// Agent card and capability types.
pub mod agent_card;
/// Streaming event types.
pub mod streaming;
/// Push notification configuration types.
pub mod push_notification;
/// Typed request parameter structs.
pub mod params;
/// JSON-RPC 2.0 envelopes and method constants.
pub mod jsonrpc;
/// Error types and JSON-RPC error codes.
pub mod error;
/// Type conversions between serde and proto types.
pub mod convert;
/// Generated proto types (gRPC feature).
pub mod proto;

// Client (feature-gated)
/// A2A client with transport selection.
#[cfg(feature = "client")]
pub mod client;

// Server (feature-gated)
/// A2A server serving all protocol bindings.
#[cfg(feature = "server")]
pub mod server;

// Re-exports for convenience
pub use error::A2aError;
pub use types::{Artifact, FileContent, Message, Part, Role};
pub use task::{Task, TaskState, TaskStatus};
pub use agent_card::{AgentCard, AgentCapabilities, AgentSkill, AgentProvider, AgentInterface, SecurityScheme, SecurityRequirement, AgentExtension, AgentCardSignature, OAuthFlows};
pub use streaming::{SendMessageResponse, StreamEvent, TaskArtifactUpdateEvent, TaskStatusUpdateEvent};
pub use push_notification::{AuthenticationInfo, TaskPushNotificationConfig};
pub use params::*;
pub use jsonrpc::{JsonRpcRequest, JsonRpcResponse, RequestId};

#[cfg(feature = "client")]
pub use client::{A2aClient, Transport};

#[cfg(feature = "server")]
pub use server::{A2aHandler, A2aServer};
