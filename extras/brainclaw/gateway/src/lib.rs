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
/// Cron job data types and persistent store.
pub mod cron;
/// Cross-channel user identity mapping.
pub mod identity;
/// Interactive tool approval via chat (ask user yes/no before executing tools).
pub mod approval;
/// OpenAI-compatible API endpoint (/v1/chat/completions, /v1/models, /v1/embeddings).
pub mod openai_compat;
/// Audit logging for security-relevant events.
pub mod audit;
/// Agent-backed inbound handler that bridges gateway events to ChatAgent.
pub mod agent_handler;
/// Channel registry for tracking connected channel adapters.
pub mod channel_registry;
/// Gateway configuration.
pub mod config;
/// Security middleware (sanitizer, origin validation, rate limiting).
pub mod middleware;
/// Message routing logic.
pub mod router;

// Re-export key types for external consumers.
pub use agent_handler::AgentInboundHandler;
pub use router::InboundHandler;
/// Media processing pipeline for attachments.
pub mod media;
/// In-memory metrics collection.
pub mod metrics;
/// Axum server setup and route definitions.
pub mod server;
/// Session management (user-to-agent session mapping).
pub mod session;
/// Session persistence — save/restore conversation history across restarts.
pub mod session_persistence;
/// Shared application state.
pub mod state;
/// Built-in WebChat channel (browser-based chat UI).
pub mod webchat;
/// TTS response processor (requires `voice` feature).
pub mod tts;
/// Webhook handler for HTTP-based channel integrations.
pub mod webhook;
/// WebSocket connection handler for channel adapters.
pub mod ws_handler;
