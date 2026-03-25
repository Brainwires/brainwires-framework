//! # Brainwires Gateway
//!
//! Always-on WebSocket server that routes messages between channel MCP servers
//! and agent sessions. This is the hub of the personal AI assistant architecture.
//!
//! Channel adapters (Discord, Telegram, Slack, etc.) connect to the gateway via
//! WebSocket, perform a handshake, and then exchange `ChannelEvent` messages.
//! The gateway manages session mapping and routes messages to/from agent processes.

/// Admin API handlers (health check, channel listing, session listing, broadcast).
pub mod admin;
/// Channel registry for tracking connected channel adapters.
pub mod channel_registry;
/// Gateway configuration.
pub mod config;
/// Message routing logic.
pub mod router;
/// Axum server setup and route definitions.
pub mod server;
/// Session management (user-to-agent session mapping).
pub mod session;
/// Shared application state.
pub mod state;
/// Webhook handler for HTTP-based channel integrations.
pub mod webhook;
/// WebSocket connection handler for channel adapters.
pub mod ws_handler;
