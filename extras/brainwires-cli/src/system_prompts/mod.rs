//! CLI System Prompt Registry
//!
//! One-stop module for every system prompt used by the brainwires CLI.
//!
//! - **Agent prompts** come from the framework: [`brainwires::agents::AgentPromptKind`] /
//!   [`brainwires::agents::build_agent_prompt`].
//! - **Mode prompts** (Edit, Ask, Plan, Batch) are CLI-specific and live in [`modes`].
//!
//! When adding a new UI mode, add the prompt function to `modes.rs` — that file
//! is the authoritative list of every interactive-mode prompt in the CLI.

pub mod modes;

// Re-export framework agent registry so callers can reach everything from one path.
pub use brainwires::agents::{AgentPromptKind, build_agent_prompt};

// Re-export all mode prompts.
pub use modes::{
    build_ask_mode_system_prompt, build_ask_mode_system_prompt_with_knowledge,
    build_batch_mode_system_prompt, build_plan_mode_system_prompt, build_system_prompt,
    build_system_prompt_with_context, build_system_prompt_with_knowledge,
    planning_agent_system_prompt,
};
