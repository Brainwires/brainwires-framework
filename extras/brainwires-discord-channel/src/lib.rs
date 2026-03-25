//! # Brainwires Discord Channel
//!
//! Discord channel adapter for the Brainwires Agent Framework.
//!
//! This crate implements the `Channel` trait from `brainwires-channels` for Discord,
//! using the serenity library. It connects to the brainwires-gateway over WebSocket
//! and can also serve as a standalone MCP tool server.

/// Configuration types for the Discord adapter.
pub mod config;
/// Discord bot implementation of the `Channel` trait.
pub mod discord;
/// Serenity `EventHandler` implementation that converts Discord events to `ChannelEvent`.
pub mod event_handler;
/// WebSocket client for connecting to the brainwires-gateway.
pub mod gateway_client;
/// MCP server exposing Discord operations as tools.
pub mod mcp_server;
