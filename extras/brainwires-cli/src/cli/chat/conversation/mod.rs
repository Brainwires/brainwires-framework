//! Conversation Loop
//!
//! Main conversation loop for CLI-based chat interactions.

mod ai_processing;
mod batch_mode;
mod chat_loop;
mod checkpoint_commands;
mod command_dispatch;
mod context_commands;
mod history_commands;
mod misc_commands;
mod plan_commands;
mod prompt_mode;

// Re-export public functions
pub use batch_mode::handle_batch_mode;
pub use chat_loop::handle_chat_with_conversation;
pub use prompt_mode::{handle_prompt_mode, handle_prompt_mode_mdap};
