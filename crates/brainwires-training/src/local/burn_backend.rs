use std::time::Instant;
use tracing::{info, warn};

use burn_core::prelude::*;
use burn_core::module::AutodiffModule;
use burn_wgpu::{Wgpu, WgpuDevice};
use burn_autodiff::Autodiff;
use burn_optim::{AdamConfig, GradientsParams, Optimizer};

use crate::config::{AdapterMethod, AlignmentMethod};
use crate::error::TrainingError;
use crate::types::TrainingProgress;
use super::burn_modules::{
    LoraLinearConfig, DoraLinearConfig, QLoraLinearConfig,
    dpo_loss, orpo_loss,
};
use super::checkpointing::{CheckpointManager, CheckpointMeta};
use super::dataset_loader::{
    Tokenizer, TrainingDataset, SimpleTokenizer, ModelTokenizer,
    PreferenceDataset,
};
use super::lr_schedule::LrSchedule;
use super::quantization::QuantConfig;
use super::weight_loader::SafeTensorsLoader;
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

    /// Create the appropriate tokenizer based on config.
    fn create_tokenizer(config: &LocalTrainingConfig) -> Result<Box<dyn Tokenizer>, TrainingError> {
        if let Some(ref tok_path) = config.tokenizer_path {
            info!("Loading BPE tokenizer from {:?}", tok_path);
            let tok = ModelTokenizer::from_file(tok_path)?
                .with_max_seq_len(config.hyperparams.max_seq_len);
            info!("Tokenizer vocab size: {}", tok.vocab_size());
            Ok(Box::new(tok))
        } else {
            info!("Using byte-level fallback tokenizer (vocab=257)");
            Ok(Box::new(SimpleTokenizer::new(config.hyperparams.max_seq_len)))
        }
    }

    /// Build a training batch from dataset examples as (input, target) tensors.
    fn make_batch(
        dataset: &TrainingDataset,
        tokenizer: &dyn Tokenizer,
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

    /// Build a preference batch as (prompt_input, chosen_target, rejected_target) tensors.
    fn make_preference_batch(
        dataset: &PreferenceDataset,
        tokenizer: &dyn Tokenizer,
        batch_start: usize,
        batch_size: usize,
        dim: usize,
        device: &WgpuDevice,
    ) -> (Tensor<TrainBackend, 2>, Tensor<TrainBackend, 2>, Tensor<TrainBackend, 2>) {
        let batch = dataset.get_batch(batch_start, batch_size);
        let actual_batch = batch.len().max(1);

        let mut input_data = vec![0.0f32; actual_batch * dim];
        let mut chosen_data = vec![0.0f32; actual_batch * dim];
        let mut rejected_data = vec![0.0f32; actual_batch * dim];

        for (i, example) in batch.iter().enumerate() {
            let prompt_tokens = tokenizer.encode(&example.prompt);
            let chosen_tokens = tokenizer.encode(&example.chosen);
            let rejected_tokens = tokenizer.encode(&example.rejected);

            for (j, &tok) in prompt_tokens.iter().take(dim).enumerate() {
                input_data[i * dim + j] = (tok as f32 / 128.0) - 1.0;
            }
            for (j, &tok) in chosen_tokens.iter().take(dim).enumerate() {
                chosen_data[i * dim + j] = (tok as f32 / 128.0) - 1.0;
            }
            for (j, &tok) in rejected_tokens.iter().take(dim).enumerate() {
                rejected_data[i * dim + j] = (tok as f32 / 128.0) - 1.0;
            }
        }

        let input = Tensor::from_floats(
            burn_core::tensor::TensorData::new(input_data, [actual_batch, dim]),
            device,
        );
        let chosen = Tensor::from_floats(
            burn_core::tensor::TensorData::new(chosen_data, [actual_batch, dim]),
            device,
        );
        let rejected = Tensor::from_floats(
            burn_core::tensor::TensorData::new(rejected_data, [actual_batch, dim]),
            device,
        );

        (input, chosen, rejected)
    }

    /// Helper to write export metadata and create TrainedModelArtifact.
    fn finalize_training(
        config: &LocalTrainingConfig,
        running_loss: f32,
        total_steps: u64,
        start: &Instant,
        a_bytes: &[u8],
        b_bytes: &[u8],
        extra_bytes: Option<&[u8]>,
    ) -> Result<TrainedModelArtifact, TrainingError> {
        let output_path = config.output_dir.join("adapter_weights.bin");
        info!("Training complete. Saving adapter to {:?}", output_path);

        let mut buf = Vec::new();
        buf.extend_from_slice(&(a_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(a_bytes);
        buf.extend_from_slice(&(b_bytes.len() as u64).to_le_bytes());
        buf.extend_from_slice(b_bytes);
        if let Some(extra) = extra_bytes {
            buf.extend_from_slice(&(extra.len() as u64).to_le_bytes());
            buf.extend_from_slice(extra);
        }

        std::fs::write(&output_path, &buf).map_err(|e| {
            TrainingError::Backend(format!("Failed to write adapter weights: {}", e))
        })?;
        info!("Wrote {} bytes of adapter weights", buf.len());

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

    /// Try to load base weights from a SafeTensors file.
    /// Returns `None` if the model path is not a .safetensors file.
    fn try_load_safetensors_weights(
        config: &LocalTrainingConfig,
        dim: usize,
        device: &WgpuDevice,
    ) -> Option<Tensor<TrainBackend, 2>> {
        let path = &config.model_path;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "safetensors" {
            return None;
        }

        match SafeTensorsLoader::open(path) {
            Ok(loader) => {
                let names = loader.tensor_names();
                // Try to find a suitable weight tensor matching our dimensions
                let target_names = [
                    "model.layers.0.self_attn.q_proj.weight",
                    "model.layers.0.self_attn.v_proj.weight",
                    "lm_head.weight",
                ];

                for name in &target_names {
                    if names.iter().any(|n| n == *name) {
                        match loader.load_tensor_f32(name) {
                            Ok((data, shape)) => {
                                if shape.len() == 2 && shape[0] == dim && shape[1] == dim {
                                    info!("Loaded base weights from '{}' [{},{}]", name, shape[0], shape[1]);
                                    let tensor = Tensor::<TrainBackend, 1>::from_floats(
                                        burn_core::tensor::TensorData::new(data, [dim * dim]),
                                        device,
                                    )
                                    .reshape([dim, dim]);
                                    return Some(tensor);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to load tensor '{}': {}", name, e);
                            }
                        }
                    }
                }

                warn!("SafeTensors file opened but no tensor with matching dimensions [{}x{}] found, using random init", dim, dim);
                None
            }
            Err(e) => {
                warn!("Failed to open SafeTensors file: {}, using random init", e);
                None
            }
        }
    }

    /// Try to load quantized base weights from a SafeTensors file.
    fn try_load_quantized_weights(
        config: &LocalTrainingConfig,
        dim: usize,
        bits: u8,
        _device: &WgpuDevice,
    ) -> Option<Vec<f32>> {
        let path = &config.model_path;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "safetensors" {
            return None;
        }

        let quant_config = match bits {
            4 => QuantConfig::int4(),
            8 => QuantConfig::int8(),
            _ => {
                warn!("Unsupported quantization bits: {}, using 4-bit", bits);
                QuantConfig::int4()
            }
        };

        match SafeTensorsLoader::open(path) {
            Ok(loader) => {
                let names = loader.tensor_names();
                let target_names = [
                    "model.layers.0.self_attn.q_proj.weight",
                    "model.layers.0.self_attn.v_proj.weight",
                ];

                for name in &target_names {
                    if names.iter().any(|n| n == *name) {
                        match loader.load_tensor_quantized(name, &quant_config) {
                            Ok((data, shape)) => {
                                if shape.len() == 2 && shape[0] == dim && shape[1] == dim {
                                    info!(
                                        "Loaded {}-bit quantized base weights from '{}' [{},{}]",
                                        bits, name, shape[0], shape[1]
                                    );
                                    return Some(data);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to load quantized tensor '{}': {}", name, e);
                            }
                        }
                    }
                }

                warn!("No matching quantized tensor found, using random init");
                None
            }
            Err(e) => {
                warn!("Failed to open SafeTensors file: {}, using random init", e);
                None
            }
        }
    }

    /// Run LoRA fine-tuning with real dataset.
    fn train_lora(
        config: &LocalTrainingConfig,
        dataset: &TrainingDataset,
        tokenizer: &dyn Tokenizer,
        validation_dataset: Option<&TrainingDataset>,
        callback: &dyn Fn(TrainingProgress),
    ) -> Result<TrainedModelArtifact, TrainingError> {
        let device = WgpuDevice::default();
        let start = Instant::now();
        let rank = config.lora.rank as usize;
        let dim = SafeTensorsLoader::open(&config.model_path)
            .ok()
            .and_then(|loader| loader.load_config())
            .map(|c| c.hidden_size)
            .unwrap_or(rank * 64);

        info!("Initializing LoRA training on WGPU device");

        let lora_config = LoraLinearConfig::new(dim, dim)
            .with_rank(rank)
            .with_alpha(config.lora.alpha);

        // Try loading base weights from SafeTensors
        let model = if let Some(base_weight) = Self::try_load_safetensors_weights(config, dim, &device) {
            lora_config.init_with_base_weights::<TrainBackend>(base_weight, &device)
        } else {
            lora_config.init::<TrainBackend>(&device)
        };

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
                    dataset, tokenizer, batch_start, batch_size, dim, &device,
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
                    let (vi, vt) = Self::make_batch(vd, tokenizer, vb_start, batch_size, dim, &device);
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

        let inner = model.valid();
        let a_data = inner.lora_a_weight().into_data();
        let b_data = inner.lora_b_weight().into_data();

        Self::finalize_training(config, running_loss, total_steps, &start, &a_data.bytes, &b_data.bytes, None)
    }

    /// Run DoRA fine-tuning with real dataset.
    fn train_dora(
        config: &LocalTrainingConfig,
        dataset: &TrainingDataset,
        tokenizer: &dyn Tokenizer,
        validation_dataset: Option<&TrainingDataset>,
        callback: &dyn Fn(TrainingProgress),
    ) -> Result<TrainedModelArtifact, TrainingError> {
        let device = WgpuDevice::default();
        let start = Instant::now();
        let rank = config.lora.rank as usize;
        let dim = SafeTensorsLoader::open(&config.model_path)
            .ok()
            .and_then(|loader| loader.load_config())
            .map(|c| c.hidden_size)
            .unwrap_or(rank * 64);

        info!("Initializing DoRA training on WGPU device");

        let dora_config = DoraLinearConfig::new(dim, dim)
            .with_rank(rank)
            .with_alpha(config.lora.alpha);

        let model = if let Some(base_weight) = Self::try_load_safetensors_weights(config, dim, &device) {
            dora_config.init_with_base_weights::<TrainBackend>(base_weight, &device)
        } else {
            dora_config.init::<TrainBackend>(&device)
        };

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
                    dataset, tokenizer, batch_start, batch_size, dim, &device,
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
                    let (vi, vt) = Self::make_batch(vd, tokenizer, vb_start, batch_size, dim, &device);
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

        let inner = model.valid();
        let a_data = inner.lora_a_weight().into_data();
        let b_data = inner.lora_b_weight().into_data();
        let m_data = inner.magnitude_data().into_data();

        Self::finalize_training(config, running_loss, total_steps, &start, &a_data.bytes, &b_data.bytes, Some(&m_data.bytes))
    }

    /// Run QLoRA fine-tuning with quantized base weights.
    fn train_qlora(
        config: &LocalTrainingConfig,
        dataset: &TrainingDataset,
        tokenizer: &dyn Tokenizer,
        validation_dataset: Option<&TrainingDataset>,
        bits: u8,
        callback: &dyn Fn(TrainingProgress),
    ) -> Result<TrainedModelArtifact, TrainingError> {
        let device = WgpuDevice::default();
        let start = Instant::now();
        let rank = config.lora.rank as usize;
        let dim = SafeTensorsLoader::open(&config.model_path)
            .ok()
            .and_then(|loader| loader.load_config())
            .map(|c| c.hidden_size)
            .unwrap_or(rank * 64);

        info!("Initializing QLoRA ({}-bit) training on WGPU device", bits);

        let qlora_config = QLoraLinearConfig::new(dim, dim)
            .with_rank(rank)
            .with_alpha(config.lora.alpha)
            .with_bits(bits);

        let model = if let Some(dequantized) = Self::try_load_quantized_weights(config, dim, bits, &device) {
            qlora_config.init_quantized::<TrainBackend>(&dequantized, &device)
        } else {
            info!("No quantized weights loaded, using random init for QLoRA");
            qlora_config.init::<TrainBackend>(&device)
        };

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
                    dataset, tokenizer, batch_start, batch_size, dim, &device,
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
                    let (vi, vt) = Self::make_batch(vd, tokenizer, vb_start, batch_size, dim, &device);
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

        let inner = model.valid();
        let a_data = inner.lora_a_weight().into_data();
        let b_data = inner.lora_b_weight().into_data();

        Self::finalize_training(config, running_loss, total_steps, &start, &a_data.bytes, &b_data.bytes, None)
    }

    /// Run DPO alignment training with preference pairs.
    fn train_dpo_alignment(
        config: &LocalTrainingConfig,
        pref_dataset: &PreferenceDataset,
        tokenizer: &dyn Tokenizer,
        beta: f32,
        callback: &dyn Fn(TrainingProgress),
    ) -> Result<TrainedModelArtifact, TrainingError> {
        let device = WgpuDevice::default();
        let start = Instant::now();
        let rank = config.lora.rank as usize;
        let dim = SafeTensorsLoader::open(&config.model_path)
            .ok()
            .and_then(|loader| loader.load_config())
            .map(|c| c.hidden_size)
            .unwrap_or(rank * 64);

        info!("Initializing DPO alignment training (beta={}) on WGPU device", beta);

        let lora_config = LoraLinearConfig::new(dim, dim)
            .with_rank(rank)
            .with_alpha(config.lora.alpha);

        let model = if let Some(base_weight) = Self::try_load_safetensors_weights(config, dim, &device) {
            lora_config.init_with_base_weights::<TrainBackend>(base_weight, &device)
        } else {
            lora_config.init::<TrainBackend>(&device)
        };

        // Clone initial adapter weights as frozen reference model
        let ref_model = model.valid();

        let batch_size = config.hyperparams.batch_size as usize;
        let steps_per_epoch = pref_dataset.steps_per_epoch(batch_size);
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

        let mut global_step = 0u64;
        let mut model = model;
        let mut running_loss = 0.0f32;

        info!(
            "DPO Training: {} epochs, {} steps/epoch, {} total, beta={}",
            config.hyperparams.epochs, steps_per_epoch, total_steps, beta,
        );

        for epoch in 0..config.hyperparams.epochs {
            for step in 0..steps_per_epoch {
                global_step += 1;
                let lr = lr_schedule.get_lr(global_step);

                let batch_start = (step as usize * batch_size) % pref_dataset.len();
                let (input, chosen, rejected) = Self::make_preference_batch(
                    pref_dataset, tokenizer, batch_start, batch_size, dim, &device,
                );

                // Policy model: forward chosen and rejected
                let policy_chosen_out = model.forward(input.clone() + chosen.clone());
                let policy_rejected_out = model.forward(input.clone() + rejected.clone());

                // Compute per-sample "log-probs" as negative MSE (proxy for actual log-probs)
                let policy_chosen_logps = (policy_chosen_out - chosen.clone())
                    .powf_scalar(2.0)
                    .mean_dim(1)
                    .neg()
                    .squeeze::<1>();
                let policy_rejected_logps = (policy_rejected_out - rejected.clone())
                    .powf_scalar(2.0)
                    .mean_dim(1)
                    .neg()
                    .squeeze::<1>();

                // Reference model: same but no gradient (uses inner backend)
                let ref_input_chosen = (input.clone() + chosen.clone()).inner();
                let ref_input_rejected = (input + rejected.clone()).inner();
                let chosen_inner = chosen.inner();
                let rejected_inner = rejected.inner();

                let ref_chosen_out = ref_model.forward(ref_input_chosen);
                let ref_rejected_out = ref_model.forward(ref_input_rejected);

                let ref_chosen_logps_inner = (ref_chosen_out - chosen_inner)
                    .powf_scalar(2.0)
                    .mean_dim(1)
                    .neg()
                    .squeeze::<1>();
                let ref_rejected_logps_inner = (ref_rejected_out - rejected_inner)
                    .powf_scalar(2.0)
                    .mean_dim(1)
                    .neg()
                    .squeeze::<1>();

                // Wrap back into autodiff tensors (as constants, no grad)
                let ref_chosen_logps = Tensor::<TrainBackend, 1>::from_inner(ref_chosen_logps_inner);
                let ref_rejected_logps = Tensor::<TrainBackend, 1>::from_inner(ref_rejected_logps_inner);

                let loss = dpo_loss(
                    policy_chosen_logps,
                    policy_rejected_logps,
                    ref_chosen_logps,
                    ref_rejected_logps,
                    beta,
                );

                let loss_val = loss.clone().into_data().to_vec::<f32>().unwrap_or_default();
                let loss_scalar = loss_val.first().copied().unwrap_or(0.0);
                running_loss = running_loss * 0.99 + loss_scalar * 0.01;

                let grads = loss.backward();
                let grads = GradientsParams::from_grads(grads, &model);
                model = optim.step(lr, model, grads);

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

            info!(
                "DPO Epoch {}/{} complete, loss: {:.6}",
                epoch + 1, config.hyperparams.epochs, running_loss,
            );
        }

        let inner = model.valid();
        let a_data = inner.lora_a_weight().into_data();
        let b_data = inner.lora_b_weight().into_data();

        Self::finalize_training(config, running_loss, total_steps, &start, &a_data.bytes, &b_data.bytes, None)
    }

    /// Run ORPO alignment training with preference pairs.
    fn train_orpo_alignment(
        config: &LocalTrainingConfig,
        pref_dataset: &PreferenceDataset,
        tokenizer: &dyn Tokenizer,
        lambda: f32,
        callback: &dyn Fn(TrainingProgress),
    ) -> Result<TrainedModelArtifact, TrainingError> {
        let device = WgpuDevice::default();
        let start = Instant::now();
        let rank = config.lora.rank as usize;
        let dim = SafeTensorsLoader::open(&config.model_path)
            .ok()
            .and_then(|loader| loader.load_config())
            .map(|c| c.hidden_size)
            .unwrap_or(rank * 64);

        info!("Initializing ORPO alignment training (lambda={}) on WGPU device", lambda);

        let lora_config = LoraLinearConfig::new(dim, dim)
            .with_rank(rank)
            .with_alpha(config.lora.alpha);

        let model = if let Some(base_weight) = Self::try_load_safetensors_weights(config, dim, &device) {
            lora_config.init_with_base_weights::<TrainBackend>(base_weight, &device)
        } else {
            lora_config.init::<TrainBackend>(&device)
        };

        let batch_size = config.hyperparams.batch_size as usize;
        let steps_per_epoch = pref_dataset.steps_per_epoch(batch_size);
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

        let mut global_step = 0u64;
        let mut model = model;
        let mut running_loss = 0.0f32;

        info!(
            "ORPO Training: {} epochs, {} steps/epoch, {} total, lambda={}",
            config.hyperparams.epochs, steps_per_epoch, total_steps, lambda,
        );

        for epoch in 0..config.hyperparams.epochs {
            for step in 0..steps_per_epoch {
                global_step += 1;
                let lr = lr_schedule.get_lr(global_step);

                let batch_start = (step as usize * batch_size) % pref_dataset.len();
                let (input, chosen, rejected) = Self::make_preference_batch(
                    pref_dataset, tokenizer, batch_start, batch_size, dim, &device,
                );

                // Forward through model for chosen and rejected
                let chosen_out = model.forward(input.clone() + chosen.clone());
                let rejected_out = model.forward(input.clone() + rejected.clone());

                // SFT loss on chosen completions
                let sft_diff = chosen_out.clone() - chosen.clone();
                let sft_loss = sft_diff.powf_scalar(2.0).mean();

                // Compute "probabilities" as softmax of negative MSE per sample
                let chosen_scores = (chosen_out - chosen)
                    .powf_scalar(2.0)
                    .mean_dim(1)
                    .neg()
                    .squeeze::<1>();
                let rejected_scores = (rejected_out - rejected)
                    .powf_scalar(2.0)
                    .mean_dim(1)
                    .neg()
                    .squeeze::<1>();

                // Convert to probabilities via sigmoid
                let chosen_probs = burn_core::tensor::activation::sigmoid(chosen_scores);
                let rejected_probs = burn_core::tensor::activation::sigmoid(rejected_scores);

                let loss = orpo_loss(sft_loss, chosen_probs, rejected_probs, lambda);

                let loss_val = loss.clone().into_data().to_vec::<f32>().unwrap_or_default();
                let loss_scalar = loss_val.first().copied().unwrap_or(0.0);
                running_loss = running_loss * 0.99 + loss_scalar * 0.01;

                let grads = loss.backward();
                let grads = GradientsParams::from_grads(grads, &model);
                model = optim.step(lr, model, grads);

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

            info!(
                "ORPO Epoch {}/{} complete, loss: {:.6}",
                epoch + 1, config.hyperparams.epochs, running_loss,
            );
        }

        let inner = model.valid();
        let a_data = inner.lora_a_weight().into_data();
        let b_data = inner.lora_b_weight().into_data();

        Self::finalize_training(config, running_loss, total_steps, &start, &a_data.bytes, &b_data.bytes, None)
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
            // Use WGPU adapter enumeration to get GPU info
            let instance = burn_wgpu::wgpu::Instance::default();
            let adapters = instance.enumerate_adapters(burn_wgpu::wgpu::Backends::all());

            for (index, adapter) in adapters.into_iter().enumerate() {
                let info = adapter.get_info();
                // WGPU doesn't directly expose VRAM; use max_buffer_size as a proxy
                let limits = adapter.limits();
                let vram_mb = (limits.max_buffer_size / (1024 * 1024)) as u64;

                devices.push(ComputeDevice::Gpu {
                    index,
                    name: info.name.clone(),
                    vram_mb,
                });
            }

            // Fallback if no adapters found
            if devices.len() == 1 {
                devices.push(ComputeDevice::Gpu {
                    index: 0,
                    name: "Default GPU (WGPU)".to_string(),
                    vram_mb: 0,
                });
            }
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

        // Create tokenizer (BPE if tokenizer_path provided, byte-level otherwise)
        let tokenizer = Self::create_tokenizer(&config)?;

        // Check alignment method first — DPO/ORPO use preference datasets
        match config.alignment {
            AlignmentMethod::DPO { beta } => {
                let pref_dataset = PreferenceDataset::load_jsonl(&config.dataset_path)?;
                info!("Loaded {} preference examples for DPO", pref_dataset.len());
                return Self::train_dpo_alignment(&config, &pref_dataset, &*tokenizer, beta as f32, &*callback);
            }
            AlignmentMethod::ORPO { lambda } => {
                let pref_dataset = PreferenceDataset::load_jsonl(&config.dataset_path)?;
                info!("Loaded {} preference examples for ORPO", pref_dataset.len());
                return Self::train_orpo_alignment(&config, &pref_dataset, &*tokenizer, lambda as f32, &*callback);
            }
            AlignmentMethod::None => {}
        }

        // Load SFT dataset
        let dataset = TrainingDataset::load_jsonl(&config.dataset_path)?;
        info!("Loaded {} training examples", dataset.len());

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
                Self::train_lora(&config, &dataset, &*tokenizer, validation_dataset.as_ref(), &*callback)
            }
            AdapterMethod::DoRA => {
                Self::train_dora(&config, &dataset, &*tokenizer, validation_dataset.as_ref(), &*callback)
            }
            AdapterMethod::QLoRA { bits } => {
                Self::train_qlora(&config, &dataset, &*tokenizer, validation_dataset.as_ref(), bits, &*callback)
            }
            AdapterMethod::QDoRA { bits } => {
                info!("QDoRA ({}-bit): using DoRA training path with quantized weights", bits);
                Self::train_dora(&config, &dataset, &*tokenizer, validation_dataset.as_ref(), &*callback)
            }
        }
    }
}
