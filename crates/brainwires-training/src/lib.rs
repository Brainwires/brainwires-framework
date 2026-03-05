#![warn(missing_docs)]
//! # Brainwires Training
//!
//! Model training and fine-tuning for the Brainwires Agent Framework.
//!
//! Supports cloud fine-tuning (OpenAI, Together, Fireworks, Anyscale, Bedrock, Vertex)
//! and local adapter training (LoRA, QLoRA, DoRA) via Burn framework.

/// Training error types.
pub mod error;
/// Training job types and status.
pub mod types;
/// Training configuration and hyperparameters.
pub mod config;

/// Cloud fine-tuning providers.
#[cfg(feature = "cloud")]
pub mod cloud;

/// Local adapter training (LoRA/QLoRA/DoRA).
#[cfg(feature = "local")]
pub mod local;

/// Training job management.
pub mod manager;

// Re-export core types (always available)
pub use error::TrainingError;
pub use types::{
    TrainingJobId, TrainingJobStatus, TrainingProgress, TrainingMetrics,
    TrainingJobSummary, DatasetId,
};
pub use config::{
    TrainingHyperparams, LoraConfig, AdapterMethod, AlignmentMethod,
    LrScheduler,
};

#[cfg(feature = "cloud")]
pub use cloud::{
    FineTuneProvider, CloudFineTuneConfig, FineTuneProviderFactory,
};

#[cfg(feature = "local")]
pub use local::{
    TrainingBackend, LocalTrainingConfig, ComputeDevice, TrainedModelArtifact,
};

pub use manager::TrainingManager;
