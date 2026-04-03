//! System Prompt Builder — compatibility shim
//!
//! All prompt implementations have moved to [`crate::system_prompts::modes`].
//! This module re-exports them so existing call sites continue to compile.

pub use crate::system_prompts::modes::{
    build_ask_mode_system_prompt, build_ask_mode_system_prompt_with_knowledge,
    build_system_prompt, build_system_prompt_with_context, build_system_prompt_with_knowledge,
};
