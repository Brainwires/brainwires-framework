#![warn(missing_docs)]
//! # brainwires-autonomy
//!
//! Autonomous agent operations — self-improvement, Git workflows, and
//! human-out-of-loop execution for the Brainwires Agent Framework.
//!
//! ## Feature flags
//!
//! | Feature | Description |
//! |---------|-------------|
//! | `self-improve` | Self-improvement controller and strategies |
//! | `eval-driven` | Eval-driven feedback loop (requires `brainwires-eval`) |
//! | `supervisor` | Agent supervisor with health monitoring |
//! | `attention` | Attention mechanism with RAG integration |
//! | `parallel` | Parallel coordinator with optional MDAP |
//! | `training` | Autonomous training loop |
//! | `git-workflow` | Automated Git workflow pipeline |
//! | `webhook` | Webhook server for Git forge events |
//! | `full` | All features enabled |

pub mod error;
pub mod config;
pub mod safety;
pub mod metrics;

#[cfg(feature = "self-improve")]
pub mod self_improve;

pub mod agent_ops;

#[cfg(feature = "git-workflow")]
pub mod git_workflow;

pub use error::AutonomyError;
pub use config::AutonomyConfig;
pub use safety::{ApprovalPolicy, AutonomousOperation, SafetyGuard};
pub use metrics::{SessionMetrics, SessionReport};
