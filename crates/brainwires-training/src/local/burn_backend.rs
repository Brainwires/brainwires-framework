use tracing::{info, debug};

use crate::error::TrainingError;
use crate::types::TrainingProgress;
use super::{ComputeDevice, LocalTrainingConfig, TrainedModelArtifact, TrainingBackend};

/// Burn framework training backend with WGPU GPU support.
pub struct BurnBackend;

impl BurnBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BurnBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TrainingBackend for BurnBackend {
    fn name(&self) -> &str {
        "burn-wgpu"
    }

    fn available_devices(&self) -> Vec<ComputeDevice> {
        // In a real implementation, this would query WGPU for available adapters
        let mut devices = vec![ComputeDevice::Cpu];

        // Check for GPU via WGPU (placeholder — real implementation would enumerate adapters)
        #[cfg(not(target_arch = "wasm32"))]
        {
            devices.push(ComputeDevice::Gpu {
                index: 0,
                name: "Default GPU (WGPU)".to_string(),
                vram_mb: 0, // Would be detected at runtime
            });
        }

        devices
    }

    fn train(
        &self,
        config: LocalTrainingConfig,
        callback: Box<dyn Fn(TrainingProgress) + Send>,
    ) -> Result<TrainedModelArtifact, TrainingError> {
        info!("Starting local training with Burn backend");
        info!("Model: {:?}", config.model_path);
        info!("Dataset: {:?}", config.dataset_path);
        info!("Device: {}", config.device);
        info!("LoRA rank: {}, alpha: {}", config.lora.rank, config.lora.alpha);

        // Verify paths exist
        if !config.model_path.exists() {
            return Err(TrainingError::Config(format!(
                "Model file not found: {:?}",
                config.model_path
            )));
        }

        if !config.dataset_path.exists() {
            return Err(TrainingError::Config(format!(
                "Dataset file not found: {:?}",
                config.dataset_path
            )));
        }

        // Create output directory
        std::fs::create_dir_all(&config.output_dir).map_err(|e| {
            TrainingError::Config(format!("Failed to create output directory: {}", e))
        })?;

        // TODO: Full Burn training loop implementation
        // 1. Load base model weights into Burn tensors
        // 2. Initialize LoRA adapter layers
        // 3. Load and tokenize training dataset
        // 4. Training loop with gradient accumulation
        // 5. Evaluation on validation set
        // 6. Export final model

        let total_steps = config.hyperparams.epochs as u64 * 100; // placeholder

        // Report initial progress
        callback(TrainingProgress {
            epoch: 0,
            total_epochs: config.hyperparams.epochs,
            step: 0,
            total_steps,
            train_loss: None,
            eval_loss: None,
            learning_rate: Some(config.hyperparams.learning_rate),
            elapsed_secs: 0,
        });

        debug!("Training complete (placeholder — actual Burn training loop not yet implemented)");

        let output_path = config.output_dir.join("adapter_weights.safetensors");

        Ok(TrainedModelArtifact {
            model_path: output_path,
            format: "adapter_only".to_string(),
            base_model: config.model_path.to_string_lossy().to_string(),
            metrics: crate::types::TrainingMetrics {
                total_steps,
                total_epochs: config.hyperparams.epochs,
                ..Default::default()
            },
            lora_config: Some(config.lora),
        })
    }
}
