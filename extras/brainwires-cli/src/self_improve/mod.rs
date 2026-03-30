pub mod comparator;
pub mod config;
pub mod controller;
pub mod feedback_loop;
pub mod metrics;
pub mod safety;
pub mod strategies;
pub mod task_generator;

pub use config::SelfImprovementConfig;
pub use controller::SelfImprovementController;
pub use feedback_loop::{AutonomousFeedbackLoop, FeedbackLoopConfig, FeedbackLoopReport};
pub use metrics::SessionReport;
pub use task_generator::TaskGenerator;
