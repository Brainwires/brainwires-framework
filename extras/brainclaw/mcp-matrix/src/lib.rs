//! Brainwires Matrix channel adapter.
//!
//! Connects Matrix rooms to the Brainwires gateway using the `matrix-sdk` crate.

pub mod config;
pub mod event_handler;
pub mod gateway_client;
pub mod matrix;
pub mod mcp_server;
