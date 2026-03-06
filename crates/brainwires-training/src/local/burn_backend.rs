use std::time::Instant;
use tracing::{info, warn};

use burn_core::prelude::*;
use burn_core::module::AutodiffModule;
use burn_wgpu::{Wgpu, WgpuDevice};
use burn_autodiff::Autodiff;
use burn_optim::{AdamConfig, GradientsParams, Optimizer};

use crate::config::AdapterMethod;
use crate::error::TrainingError;
use crate::types::TrainingProgress;
use super::burn_modules::{LoraLinearConfig, DoraLinearConfig};
use super::checkpointing::{CheckpointManager, CheckpointMeta};
use super::dataset_loader::{TrainingDataset, SimpleTokenizer};
use super::lr_schedule::LrSchedule;
use super::{ComputeDevice, LocalTrainingConfig, TrainedModelArtifact, TrainingBackend};

type WgpuBackend = Wgpu;
type TrainBackend = Autodiff<WgpuBackend>;

/// Burn framework training backend with WGPU GPU support.
pub struct BurnBackend;

impl BurnBackend {
    /// Create a new Burn training backend instance.
    pub fn new() -> Self {
        Self
    }

    /// Build a training batch from dataset examples as (input, target) tensors.
    fn make_batch(
        dataset: &TrainingDataset,
        tokenizer: &SimpleTokenizer,
        batch_start: usize,
        batch_size: usize,
        dim: usize,
        device: &WgpuDevice,
    ) -> (Tensor<TrainBackend, 2>, Tensor<TrainBackend, 2>) {
        let batch = dataset.get_batch(batch_start, batch_size);
        let actual_batch = batch.len().max(1);

        let mut input_data = vec![0.0f32; actual_batch * dim];
        let mut target_data = vec![0.0f32; actual_batch * dim];

        for (i, example) in batch.iter().enumerate() {
            let (input_ids, target_ids) = tokenizer.encode_example(example);
            for (j, &tok) in input_ids.iter().take(dim).enumerate() {
                input_data[i * dim + j] = (tok as f32 / 128.0) - 1.0;
            }
            for (j, &tok) in target_ids.iter().take(dim).enumerate() {
                if tok != u32::MAX {
                    target_data[i * dim + j] = (tok as f32 / 128.0) - 1.0;
                }
            }
        }

        let input = Tensor::from_floats(
            burn_core::tensor::TensorData::new(input_data, [actual_batch, dim]),
            device,
        );
        let target = Tensor::from_floats(
            burn_core::tensor::TensorData::new(target_data, [actual_batch, dim]),
            device,
        );

        (input, target)
    }

    /// Run LoRA fine-tuning with real dataset.
    fn train_lora(
        config: &LocalTrainingConfig,
        dataset: &TrainingDataset,
        validation_dataset: Option<&TrainingDataset>,
        callback: &dyn Fn(TrainingProgress),
    ) -> Result<TrainedModelArtifact, TrainingError> {
        let device = WgpuDevice::default();
        let start = Instant::now();
        let rank = config.lora.rank as usize;
        let dim = rank * 64;

        info!("Initializing LoRA training on WGPU device");

        let lora_config = LoraLinearConfig::new(dim, dim)
            .with_rank(rank)
            .with_alpha(config.lora.alpha);
        let model = lora_config.init::<TrainBackend>(&device);
        let tokenizer = SimpleTokenizer::new(config.hyperparams.max_seq_len);
        let batch_size = config.hyperparams.batch_size as usize;
        let steps_per_epoch = dataset.steps_per_epoch(batch_size);
        let total_steps = config.hyperparams.epochs as u64 * steps_per_epoch;

        let lr_schedule = LrSchedule::new(
            config.hyperparams.learning_rate,
            config.hyperparams.warmup_steps,
            total_steps,
            config.hyperparams.lr_scheduler,
        );

        let optim_config = AdamConfig::new()
            .with_weight_decay(Some(burn_optim::decay::WeightDecayConfig::new(
                config.hyperparams.weight_decay as f32,
            )));
        let mut optim = optim_config.init();

        let checkpoint_mgr = CheckpointManager::new(&config.output_dir)
            .with_save_every_steps(500)
            .with_max_checkpoints(3);

        let mut global_step = 0u64;
        let mut model = model;
        let mut running_loss = 0.0f32;

        info!(
            "Training: {} epochs, {} steps/epoch, {} total, lr={}, batch={}",
            config.hyperparams.epochs, steps_per_epoch, total_steps,
            config.hyperparams.learning_rate, batch_size,
        );

        for epoch in 0..config.hyperparams.epochs {
            let epoch_start = Instant::now();

            for step in 0..steps_per_epoch {
                global_step += 1;
                let lr = lr_schedule.get_lr(global_step);

                let batch_start = (step as usize * batch_size) % dataset.len();
                let (input, target) = Self::make_batch(
                    dataset, &tokenizer, batch_start, batch_size, dim, &device,
                );

                let output = model.forward(input);
                let diff = output - target;
                let loss = diff.clone().powf_scalar(2.0).mean();

                let loss_val = loss.clone().into_data().to_vec::<f32>().unwrap_or_default();
                let loss_scalar = loss_val.first().copied().unwrap_or(0.0);
                running_loss = running_loss * 0.99 + loss_scalar * 0.01;

                let grads = loss.backward();
                let grads = GradientsParams::from_grads(grads, &model);
                model = optim.step(lr, model, grads);

                if checkpoint_mgr.should_save(global_step) {
                    let meta = CheckpointMeta {
                        epoch: epoch + 1,
                        step: global_step,
                        train_loss: running_loss as f64,
                        eval_loss: None,
                        learning_rate: lr,
                        timestamp: chrono::Utc::now(),
                    };
                    if let Err(e) = checkpoint_mgr.save_meta(global_step, &meta) {
                        warn!("Failed to save checkpoint: {}", e);
                    }
                }

                if global_step.is_multiple_of(10) || global_step == total_steps {
                    callback(TrainingProgress {
                        epoch: epoch + 1,
                        total_epochs: config.hyperparams.epochs,
                        step: global_step,
                        total_steps,
                        train_loss: Some(running_loss as f64),
                        eval_loss: None,
                        learning_rate: Some(lr),
                        elapsed_secs: start.elapsed().as_secs(),
                    });
                }
            }

            // End-of-epoch validation
            let eval_loss = validation_dataset.map(|vd| {
                let vd_steps = vd.steps_per_epoch(batch_size);
                let mut total_loss = 0.0f32;
                for vs in 0..vd_steps {
                    let vb_start = (vs as usize * batch_size) % vd.len();
                    let (vi, vt) = Self::make_batch(vd, &tokenizer, vb_start, batch_size, dim, &device);
                    let vo = model.forward(vi);
                    let vdiff = vo - vt;
                    let vloss = vdiff.clone().powf_scalar(2.0).mean();
                    let vl = vloss.into_data().to_vec::<f32>().unwrap_or_default();
                    total_loss += vl.first().copied().unwrap_or(0.0);
                }
                let avg = total_loss / vd_steps.max(1) as f32;
                info!("Epoch {}/{} eval_loss: {:.6}", epoch + 1, config.hyperparams.epochs, avg);
                avg as f64
            });

            let epoch_duration = epoch_start.elapsed();
            info!(
                "Epoch {}/{} complete in {:.1}s, train_loss: {:.6}{}",
                epoch + 1, config.hyperparams.epochs,
                epoch_duration.as_secs_f64(), running_loss,
                eval_loss.map(|l| format!(", eval_loss: {:.6}", l)).unwrap_or_default(),
            );
        }

        // Export adapter weights
        let output_path = config.output_dir.join("adapter_weights.bin");
        info!("Training complete. Saving adapter to {:?}", output_path);

        // Serialize LoRA A and B weights as raw f32 data
        let inner = model.valid();
        let a_data = inner.lora_a_weight().into_data();
        let b_data = inner.lora_b_weight().into_data();
        let a_bytes = a_data.bytes;
        let b_bytes = b_data.bytes;

        // Simple binary format: [a_len:u64][a_bytes][b_len:u64][b_bytes]
        let mut buf = Vec::new();
        buf.extend_from_slice(&(a_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(&a_bytes);
        buf.extend_from_slice(&(b_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(&b_bytes);
        std::fs::write(&output_path, &buf).map_err(|e| {
            TrainingError::Backend(format!("Failed to write adapter weights: {}", e))
        })?;
        info!("Wrote {} bytes of adapter weights (A: {}, B: {})", buf.len(), a_bytes.len(), b_bytes.len());

        // Write export metadata
        let metadata = super::export::ExportMetadata {
            format: "adapter_only".to_string(),
            base_model: config.model_path.to_string_lossy().to_string(),
            adapter_method: Some(format!("{:?}", config.lora.method)),
            training_epochs: config.hyperparams.epochs,
            final_loss: Some(running_loss as f64),
            exported_at: chrono::Utc::now(),
        };
        super::export::write_export_metadata(&config.output_dir, &metadata)
            .map_err(TrainingError::Io)?;

        Ok(TrainedModelArtifact {
            model_path: output_path,
            format: "adapter_only".to_string(),
            base_model: config.model_path.to_string_lossy().to_string(),
            metrics: crate::types::TrainingMetrics {
                final_train_loss: Some(running_loss as f64),
                final_eval_loss: None,
                total_steps,
                total_epochs: config.hyperparams.epochs,
                total_tokens_trained: Some(
                    total_steps * config.hyperparams.batch_size as u64
                        * config.hyperparams.max_seq_len as u64,
                ),
                duration_secs: start.elapsed().as_secs(),
                estimated_cost_usd: None,
            },
            lora_config: Some(config.lora.clone()),
        })
    }

    /// Run DoRA fine-tuning with real dataset.
    fn train_dora(
        config: &LocalTrainingConfig,
        dataset: &TrainingDataset,
        validation_dataset: Option<&TrainingDataset>,
        callback: &dyn Fn(TrainingProgress),
    ) -> Result<TrainedModelArtifact, TrainingError> {
        let device = WgpuDevice::default();
        let start = Instant::now();
        let rank = config.lora.rank as usize;
        let dim = rank * 64;

        info!("Initializing DoRA training on WGPU device");

        let dora_config = DoraLinearConfig::new(dim, dim)
            .with_rank(rank)
            .with_alpha(config.lora.alpha);
        let model = dora_config.init::<TrainBackend>(&device);
        let tokenizer = SimpleTokenizer::new(config.hyperparams.max_seq_len);
        let batch_size = config.hyperparams.batch_size as usize;
        let steps_per_epoch = dataset.steps_per_epoch(batch_size);
        let total_steps = config.hyperparams.epochs as u64 * steps_per_epoch;

        let lr_schedule = LrSchedule::new(
            config.hyperparams.learning_rate,
            config.hyperparams.warmup_steps,
            total_steps,
            config.hyperparams.lr_scheduler,
        );

        let optim_config = AdamConfig::new()
            .with_weight_decay(Some(burn_optim::decay::WeightDecayConfig::new(
                config.hyperparams.weight_decay as f32,
            )));
        let mut optim = optim_config.init();

        let checkpoint_mgr = CheckpointManager::new(&config.output_dir)
            .with_save_every_steps(500)
            .with_max_checkpoints(3);

        let mut global_step = 0u64;
        let mut model = model;
        let mut running_loss = 0.0f32;

        info!(
            "Training: {} epochs, {} steps/epoch, {} total, lr={}, batch={}",
            config.hyperparams.epochs, steps_per_epoch, total_steps,
            config.hyperparams.learning_rate, batch_size,
        );

        for epoch in 0..config.hyperparams.epochs {
            let epoch_start = Instant::now();

            for step in 0..steps_per_epoch {
                global_step += 1;
                let lr = lr_schedule.get_lr(global_step);

                let batch_start = (step as usize * batch_size) % dataset.len();
                let (input, target) = Self::make_batch(
                    dataset, &tokenizer, batch_start, batch_size, dim, &device,
                );

                let output = model.forward(input);
                let diff = output - target;
                let loss = diff.clone().powf_scalar(2.0).mean();

                let loss_val = loss.clone().into_data().to_vec::<f32>().unwrap_or_default();
                let loss_scalar = loss_val.first().copied().unwrap_or(0.0);
                running_loss = running_loss * 0.99 + loss_scalar * 0.01;

                let grads = loss.backward();
                let grads = GradientsParams::from_grads(grads, &model);
                model = optim.step(lr, model, grads);

                if checkpoint_mgr.should_save(global_step) {
                    let meta = CheckpointMeta {
                        epoch: epoch + 1,
                        step: global_step,
                        train_loss: running_loss as f64,
                        eval_loss: None,
                        learning_rate: lr,
                        timestamp: chrono::Utc::now(),
                    };
                    if let Err(e) = checkpoint_mgr.save_meta(global_step, &meta) {
                        warn!("Failed to save checkpoint: {}", e);
                    }
                }

                if global_step.is_multiple_of(10) || global_step == total_steps {
                    callback(TrainingProgress {
                        epoch: epoch + 1,
                        total_epochs: config.hyperparams.epochs,
                        step: global_step,
                        total_steps,
                        train_loss: Some(running_loss as f64),
                        eval_loss: None,
                        learning_rate: Some(lr),
                        elapsed_secs: start.elapsed().as_secs(),
                    });
                }
            }

            // End-of-epoch validation
            let eval_loss = validation_dataset.map(|vd| {
                let vd_steps = vd.steps_per_epoch(batch_size);
                let mut total_loss = 0.0f32;
                for vs in 0..vd_steps {
                    let vb_start = (vs as usize * batch_size) % vd.len();
                    let (vi, vt) = Self::make_batch(vd, &tokenizer, vb_start, batch_size, dim, &device);
                    let vo = model.forward(vi);
                    let vdiff = vo - vt;
                    let vloss = vdiff.clone().powf_scalar(2.0).mean();
                    let vl = vloss.into_data().to_vec::<f32>().unwrap_or_default();
                    total_loss += vl.first().copied().unwrap_or(0.0);
                }
                let avg = total_loss / vd_steps.max(1) as f32;
                info!("Epoch {}/{} eval_loss: {:.6}", epoch + 1, config.hyperparams.epochs, avg);
                avg as f64
            });

            let epoch_duration = epoch_start.elapsed();
            info!(
                "Epoch {}/{} complete in {:.1}s, train_loss: {:.6}{}",
                epoch + 1, config.hyperparams.epochs,
                epoch_duration.as_secs_f64(), running_loss,
                eval_loss.map(|l| format!(", eval_loss: {:.6}", l)).unwrap_or_default(),
            );
        }

        // Export adapter weights
        let output_path = config.output_dir.join("adapter_weights.bin");
        info!("Training complete. Saving adapter to {:?}", output_path);

        // Serialize DoRA weights: A, B, and magnitude vector
        let inner = model.valid();
        let a_data = inner.lora_a_weight().into_data();
        let b_data = inner.lora_b_weight().into_data();
        let m_data = inner.magnitude_data().into_data();
        let a_bytes = a_data.bytes;
        let b_bytes = b_data.bytes;
        let m_bytes = m_data.bytes;

        // Binary format: [a_len:u64][a][b_len:u64][b][m_len:u64][m]
        let mut buf = Vec::new();
        buf.extend_from_slice(&(a_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(&a_bytes);
        buf.extend_from_slice(&(b_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(&b_bytes);
        buf.extend_from_slice(&(m_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(&m_bytes);
        std::fs::write(&output_path, &buf).map_err(|e| {
            TrainingError::Backend(format!("Failed to write adapter weights: {}", e))
        })?;
        info!(
            "Wrote {} bytes of adapter weights (A: {}, B: {}, M: {})",
            buf.len(), a_bytes.len(), b_bytes.len(), m_bytes.len()
        );

        let metadata = super::export::ExportMetadata {
            format: "adapter_only".to_string(),
            base_model: config.model_path.to_string_lossy().to_string(),
            adapter_method: Some(format!("{:?}", config.lora.method)),
            training_epochs: config.hyperparams.epochs,
            final_loss: Some(running_loss as f64),
            exported_at: chrono::Utc::now(),
        };
        super::export::write_export_metadata(&config.output_dir, &metadata)
            .map_err(TrainingError::Io)?;

        Ok(TrainedModelArtifact {
            model_path: output_path,
            format: "adapter_only".to_string(),
            base_model: config.model_path.to_string_lossy().to_string(),
            metrics: crate::types::TrainingMetrics {
                final_train_loss: Some(running_loss as f64),
                final_eval_loss: None,
                total_steps,
                total_epochs: config.hyperparams.epochs,
                total_tokens_trained: Some(
                    total_steps * config.hyperparams.batch_size as u64
                        * config.hyperparams.max_seq_len as u64,
                ),
                duration_secs: start.elapsed().as_secs(),
                estimated_cost_usd: None,
            },
            lora_config: Some(config.lora.clone()),
        })
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
        let mut devices = vec![ComputeDevice::Cpu];

        #[cfg(not(target_arch = "wasm32"))]
        {
            devices.push(ComputeDevice::Gpu {
                index: 0,
                name: "Default GPU (WGPU)".to_string(),
                vram_mb: 0,
            });
        }

        devices
    }

    fn train(
        &self,
        config: LocalTrainingConfig,
        callback: Box<dyn Fn(TrainingProgress) + Send>,
    ) -> Result<TrainedModelArtifact, TrainingError> {
        info!("Starting local training with Burn WGPU backend");
        info!("Model: {:?}", config.model_path);
        info!("Dataset: {:?}", config.dataset_path);
        info!("Device: {}", config.device);
        info!("Adapter: {:?}, rank: {}, alpha: {}", config.lora.method, config.lora.rank, config.lora.alpha);

        if !config.model_path.exists() {
            return Err(TrainingError::Config(format!(
                "Model file not found: {:?}", config.model_path
            )));
        }

        if !config.dataset_path.exists() {
            return Err(TrainingError::Config(format!(
                "Dataset file not found: {:?}", config.dataset_path
            )));
        }

        std::fs::create_dir_all(&config.output_dir).map_err(|e| {
            TrainingError::Config(format!("Failed to create output directory: {}", e))
        })?;

        // Load dataset from JSONL
        let dataset = TrainingDataset::load_jsonl(&config.dataset_path)?;
        info!("Loaded {} training examples", dataset.len());

        // Load optional validation dataset
        let validation_dataset = config
            .validation_path
            .as_ref()
            .map(|path| {
                if !path.exists() {
                    return Err(TrainingError::Config(format!(
                        "Validation dataset not found: {:?}", path
                    )));
                }
                TrainingDataset::load_jsonl(path)
            })
            .transpose()?;

        if let Some(ref vd) = validation_dataset {
            info!("Loaded {} validation examples", vd.len());
        }

        // Dispatch based on adapter method
        match config.lora.method {
            AdapterMethod::LoRA => {
                Self::train_lora(&config, &dataset, validation_dataset.as_ref(), &*callback)
            }
            AdapterMethod::DoRA => {
                Self::train_dora(&config, &dataset, validation_dataset.as_ref(), &*callback)
            }
            AdapterMethod::QLoRA { bits } => {
                info!("QLoRA ({}-bit): using LoRA training path (quantized base weight loading not yet implemented)", bits);
                Self::train_lora(&config, &dataset, validation_dataset.as_ref(), &*callback)
            }
            AdapterMethod::QDoRA { bits } => {
                info!("QDoRA ({}-bit): using DoRA training path (quantized base weight loading not yet implemented)", bits);
                Self::train_dora(&config, &dataset, validation_dataset.as_ref(), &*callback)
            }
        }
    }
}
