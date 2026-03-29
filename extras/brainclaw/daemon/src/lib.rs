//! # BrainClaw
//!
//! Personal AI assistant daemon built on the Brainwires Framework.
//!
//! BrainClaw is a thin orchestration layer that ties together the gateway,
//! agent handler, provider, tools, and skills into a single daemon binary.
//! All the heavy lifting is done by framework crates — BrainClaw is just
//! config + startup.

pub mod app;
pub mod config;
pub mod cron;
pub mod persona;
pub mod shell_hooks;
pub mod skill_handler;
pub mod tools;

pub use app::BrainClaw;
pub use config::BrainClawConfig;
pub use persona::Persona;
pub use skill_handler::SkillHandler;
