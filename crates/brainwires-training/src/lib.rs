#![deny(missing_docs)]
//! # Brainwires Training
//!
//! Model training and fine-tuning for the Brainwires Agent Framework.
//!
//! Supports cloud fine-tuning (OpenAI, Together, Fireworks, Anyscale, Bedrock, Vertex)
//! and local adapter training (LoRA, QLoRA, DoRA) via Burn framework.

// Re-export burn_core as `burn` so that Burn's derive macros (Module, Config) can resolve
// their internal `burn::` paths when using individual burn-* crates.
#[cfg(feature = "local")]
extern crate burn_core as burn;

/// Training configuration and hyperparameters.
pub mod config;
/// Training error types.
pub mod error;
/// Training job types and status.
pub mod types;

/// Dataset pipelines (absorbed from brainwires-datasets).
pub mod datasets;

/// Cloud fine-tuning providers.
#[cfg(feature = "cloud")]
pub mod cloud;

/// Local adapter training (LoRA/QLoRA/DoRA).
#[cfg(feature = "local")]
pub mod local;

/// Training job management.
pub mod manager;

// Re-export core types (always available)
pub use config::{AdapterMethod, AlignmentMethod, LoraConfig, LrScheduler, TrainingHyperparams};
pub use error::TrainingError;
pub use types::{
    DatasetId, TrainingJobId, TrainingJobStatus, TrainingJobSummary, TrainingMetrics,
    TrainingProgress,
};

#[cfg(feature = "cloud")]
pub use cloud::{CloudFineTuneConfig, FineTuneProvider, FineTuneProviderFactory};

#[cfg(feature = "local")]
pub use local::{ComputeDevice, LocalTrainingConfig, TrainedModelArtifact, TrainingBackend};

pub use manager::TrainingManager;
