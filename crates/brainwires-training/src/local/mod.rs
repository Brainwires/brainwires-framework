pub mod burn_backend;
pub mod burn_modules;
pub mod adapters;
pub mod alignment;
pub mod architectures;
pub mod checkpointing;
pub mod quantization;
pub mod export;

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use crate::config::{TrainingHyperparams, LoraConfig, AlignmentMethod};
use crate::error::TrainingError;
use crate::types::TrainingProgress;

/// Available compute devices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComputeDevice {
    Cpu,
    Gpu { index: usize, name: String, vram_mb: u64 },
    Mps, // Apple Metal
}

impl std::fmt::Display for ComputeDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "CPU"),
            Self::Gpu { index, name, vram_mb } => {
                write!(f, "GPU:{} ({}, {}MB VRAM)", index, name, vram_mb)
            }
            Self::Mps => write!(f, "MPS (Apple Metal)"),
        }
    }
}

/// Configuration for local training.
#[derive(Debug, Clone)]
pub struct LocalTrainingConfig {
    /// Path to base model (GGUF or safetensors).
    pub model_path: PathBuf,
    /// Path to training dataset (JSONL).
    pub dataset_path: PathBuf,
    /// Optional validation dataset.
    pub validation_path: Option<PathBuf>,
    /// Output directory for checkpoints and final model.
    pub output_dir: PathBuf,
    /// Training hyperparameters.
    pub hyperparams: TrainingHyperparams,
    /// LoRA adapter configuration.
    pub lora: LoraConfig,
    /// Alignment method.
    pub alignment: AlignmentMethod,
    /// Device to train on.
    pub device: ComputeDevice,
    /// Enable gradient checkpointing (saves memory).
    pub gradient_checkpointing: bool,
    /// Enable mixed precision training (BF16).
    pub mixed_precision: bool,
}

impl LocalTrainingConfig {
    pub fn new(model_path: impl Into<PathBuf>, dataset_path: impl Into<PathBuf>, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            model_path: model_path.into(),
            dataset_path: dataset_path.into(),
            validation_path: None,
            output_dir: output_dir.into(),
            hyperparams: TrainingHyperparams::default(),
            lora: LoraConfig::default(),
            alignment: AlignmentMethod::None,
            device: ComputeDevice::Cpu,
            gradient_checkpointing: true,
            mixed_precision: false,
        }
    }

    pub fn with_device(mut self, device: ComputeDevice) -> Self {
        self.device = device;
        self
    }

    pub fn with_validation(mut self, path: impl Into<PathBuf>) -> Self {
        self.validation_path = Some(path.into());
        self
    }
}

/// Artifact produced by a completed local training run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainedModelArtifact {
    /// Path to the output model file (GGUF or adapter weights).
    pub model_path: PathBuf,
    /// Format of the output (gguf, safetensors, adapter_only).
    pub format: String,
    /// Base model used for training.
    pub base_model: String,
    /// Final training metrics.
    pub metrics: crate::types::TrainingMetrics,
    /// LoRA config used (if adapter training).
    pub lora_config: Option<LoraConfig>,
}

/// Trait for local training backends.
pub trait TrainingBackend: Send + Sync {
    /// Backend name.
    fn name(&self) -> &str;

    /// List available compute devices.
    fn available_devices(&self) -> Vec<ComputeDevice>;

    /// Run training with progress callback.
    fn train(
        &self,
        config: LocalTrainingConfig,
        callback: Box<dyn Fn(TrainingProgress) + Send>,
    ) -> Result<TrainedModelArtifact, TrainingError>;
}

pub use burn_backend::BurnBackend;
