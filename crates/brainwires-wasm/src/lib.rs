//! # Brainwires WASM
//!
//! WASM bindings for the Brainwires Agent Framework.
//!
//! Provides a JavaScript-friendly API for the WASM-compatible subset of the framework:
//! - Core types (messages, tools, tasks)
//! - MDAP types and configuration
//! - Code execution (with `interpreters` feature)

use wasm_bindgen::prelude::*;

// Re-export WASM-safe framework crates for Rust consumers
pub use brainwires_core;
pub use brainwires_mdap;

#[cfg(feature = "interpreters")]
pub use brainwires_code_interpreters;

// ── WASM Bindings ────────────────────────────────────────────────────────

/// Get the framework version.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Validate and normalize a JSON message.
///
/// Parses the JSON into a Message struct and re-serializes it,
/// ensuring it conforms to the expected schema.
#[wasm_bindgen]
pub fn validate_message(json: &str) -> Result<String, String> {
    let msg: brainwires_core::Message =
        serde_json::from_str(json).map_err(|e| format!("Invalid message JSON: {e}"))?;
    serde_json::to_string(&msg).map_err(|e| format!("Serialization error: {e}"))
}

/// Validate and normalize a JSON tool definition.
#[wasm_bindgen]
pub fn validate_tool(json: &str) -> Result<String, String> {
    let tool: brainwires_core::Tool =
        serde_json::from_str(json).map_err(|e| format!("Invalid tool JSON: {e}"))?;
    serde_json::to_string(&tool).map_err(|e| format!("Serialization error: {e}"))
}

/// Serialize a conversation history to the stateless protocol format.
///
/// Takes a JSON array of Messages and returns the stateless history format
/// suitable for API requests.
#[wasm_bindgen]
pub fn serialize_history(messages_json: &str) -> Result<String, String> {
    let messages: Vec<brainwires_core::Message> =
        serde_json::from_str(messages_json).map_err(|e| format!("Invalid messages JSON: {e}"))?;
    let history = brainwires_core::serialize_messages_to_stateless_history(&messages);
    serde_json::to_string(&history).map_err(|e| format!("Serialization error: {e}"))
}
