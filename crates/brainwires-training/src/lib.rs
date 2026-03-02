//! # Brainwires Training
//!
//! Model training and fine-tuning for the Brainwires Agent Framework.
//!
//! Supports cloud fine-tuning (OpenAI, Together, Fireworks, Anyscale, Bedrock, Vertex)
//! and local adapter training (LoRA, QLoRA, DoRA) via Burn framework.

pub mod error;
pub mod types;
pub mod config;

#[cfg(feature = "cloud")]
pub mod cloud;

#[cfg(feature = "local")]
pub mod local;

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
