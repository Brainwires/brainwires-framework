//! Self-improvement strategies and controller.
//!
//! Extracted from the CLI's `self_improve/` module and generalized
//! for use as a library. The controller accepts an `Arc<dyn Provider>`
//! instead of creating providers internally.

pub mod comparator;
pub mod controller;
pub mod strategies;
pub mod task_generator;

#[cfg(feature = "eval-driven")]
pub mod feedback_loop;

pub use comparator::{Comparator, ComparisonResult, PathResult};
pub use controller::{CycleResult, SelfImprovementController};
pub use strategies::{ImprovementCategory, ImprovementStrategy, ImprovementTask};
pub use task_generator::TaskGenerator;

#[cfg(feature = "eval-driven")]
pub use feedback_loop::{AutonomousFeedbackLoop, FeedbackLoopConfig, FeedbackLoopReport};
