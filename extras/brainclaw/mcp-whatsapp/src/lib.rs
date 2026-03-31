//! WhatsApp Business channel adapter for BrainClaw.
//!
//! Connects WhatsApp Business accounts to the brainwires-gateway via the
//! Meta Graph API. Inbound messages arrive through a webhook Axum server;
//! outbound messages are sent via REST.

pub mod config;
pub mod event_handler;
pub mod gateway_client;
pub mod mcp_server;
pub mod whatsapp;
