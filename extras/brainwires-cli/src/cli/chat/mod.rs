//! Chat Module
//!
//! Handles chat functionality including conversation management,
//! streaming responses, and tool execution.

pub mod continuation;
mod conversation;
mod handler;
mod streaming;

// Re-export public API
pub use conversation::{handle_batch_mode, handle_chat_with_conversation, handle_prompt_mode};
pub use handler::handle_chat;
