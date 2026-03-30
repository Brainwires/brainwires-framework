//! Slash Command System
//!
//! Supports custom commands from .brainwires/commands/*.md files
//! and built-in commands like /clear, /status, /model

mod builtin;
pub mod executor;
mod parser;
mod registry;

pub use executor::CommandExecutor;
pub use registry::{Command, CommandRegistry};
