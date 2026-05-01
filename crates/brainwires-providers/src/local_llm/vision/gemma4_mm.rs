//! End-to-end vision-language inference for Gemma-4.
//!
//! Uses the upstream `candle_transformers::models::gemma4` types directly —
//! no vendored copy needed since `TextModel` already exposes public
//! `embed_tokens()`, `forward_embeds()`, `forward_embeds_hidden()`, and
//! `lm_head()`.
//!
//! Mixed-device execution: `embed_tokens` / `lm_head` weights live on CPU
//! (the BF16 embedding table exceeds WebGPU's max buffer size). Decoder
//! layers run on GPU. Transfers happen at the embedding/logit boundary.

use std::sync::Mutex;

use thiserror::Error;
use tokenizers::Tokenizer;

use candle_transformers::models::gemma4::config::Gemma4Config;
use candle_transformers::models::gemma4::Model as Gemma4Model;

use crate::CandleDType as DType;
use crate::CandleDevice as Device;
use crate::CandleTensor as Tensor;

/// Best-effort diagnostic log. On wasm32 forwards to `console.log`; on
/// native, writes to stderr. Used by the gemma4 pipeline to surface
/// per-step generation state when the user reports "no output".
fn diag_log(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&msg.into());
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{msg}");
}

/// Per-token diagnostic emitted for the first 5 generation steps and at
/// step 0. Logs `next_id`, decoded text for that single token, and the
/// top-5 logit values so we can tell whether the model is producing
/// reasonable distributions vs collapsing to NaN/Inf vs always picking
/// EOS.
fn log_step_diag(step: usize, logits: &Tensor, next_id: u32, tokenizer: &Tokenizer) {
    if step > 5 {
        return;
    }
    // logits: [B, N, vocab]; take the last row.
    let last_row = match logits
        .dims3()
        .ok()
        .and_then(|(_, n, _)| logits.narrow(1, n - 1, 1).ok())
        .and_then(|t| t.squeeze(1).ok())
        .and_then(|t| t.squeeze(0).ok())
    {
        Some(row) => row,
        None => return,
    };
    let row_f32 = match last_row.dtype() {
        DType::F32 => last_row,
        _ => match last_row.to_dtype(DType::F32) {
            Ok(t) => t,
            Err(_) => return,
        },
    };
    let v: Vec<f32> = match row_f32.to_vec1::<f32>() {
        Ok(v) => v,
        Err(_) => return,
    };

    // Top-5 by magnitude
    let mut idx: Vec<usize> = (0..v.len()).collect();
    idx.sort_unstable_by(|&a, &b| v[b].partial_cmp(&v[a]).unwrap_or(std::cmp::Ordering::Equal));
    let top5: Vec<String> = idx
        .iter()
        .take(5)
        .map(|&i| format!("{i}={:.3}", v[i]))
        .collect();
    let any_nan = v.iter().any(|x| x.is_nan());
    let any_inf = v.iter().any(|x| x.is_infinite());
    let decoded = tokenizer.decode(&[next_id], false).unwrap_or_default();
    diag_log(&format!(
        "[gemma4] step {step}: next_id={next_id} decoded={decoded:?} \
         top5=[{}] nan={any_nan} inf={any_inf}",
        top5.join(", "),
    ));
}

/// Image token id for Gemma-4 (from config defaults).
pub const GEMMA4_IMAGE_TOKEN_ID: u32 = 258_880;

/// Errors emitted by the Gemma-4 multimodal pipeline.
#[derive(Debug, Error)]
pub enum Gemma4PipelineError {
    /// Decoder forward / KV-cache failure.
    #[error("decoder: {0}")]
    Decoder(String),
    /// Tokenizer encode / decode failure.
    #[error("tokenizer: {0}")]
    Tokenizer(String),
    /// Tensor shape / device error from Candle.
    #[error("tensor: {0}")]
    Tensor(String),
    /// Caller passed mismatched inputs.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl From<candle_core::Error> for Gemma4PipelineError {
    fn from(e: candle_core::Error) -> Self {
        Gemma4PipelineError::Tensor(e.to_string())
    }
}

/// Gemma-4 multimodal inference pipeline with mixed-device execution.
pub struct Gemma4MultiModal {
    model: Mutex<Gemma4Model>,
    tokenizer: Tokenizer,
    gpu_device: Device,
    cfg: Gemma4Config,
}

impl Gemma4MultiModal {
    /// Build a pipeline from already-loaded components.
    pub fn from_components(
        model: Gemma4Model,
        tokenizer: Tokenizer,
        gpu_device: Device,
        cfg: Gemma4Config,
    ) -> Self {
        Self {
            model: Mutex::new(model),
            tokenizer,
            gpu_device,
            cfg,
        }
    }

    /// Reference to the underlying Gemma4 config.
    pub fn config(&self) -> &Gemma4Config {
        &self.cfg
    }

    /// Reset the decoder's KV cache. Call between unrelated turns.
    pub fn clear_kv_cache(&self) {
        if let Ok(mut model) = self.model.lock() {
            model.clear_kv_cache();
        }
    }

    /// Whether the vision tower is currently loaded.
    pub fn has_vision(&self) -> bool {
        self.model
            .lock()
            .map(|m| m.has_vision())
            .unwrap_or(false)
    }

    /// Whether the audio tower is currently loaded.
    pub fn has_audio(&self) -> bool {
        self.model
            .lock()
            .map(|m| m.has_audio())
            .unwrap_or(false)
    }

    /// Attach vision weights to a model that was loaded with
    /// `Model::new_text_only`. Idempotent — does nothing if already attached.
    pub fn attach_vision(
        &self,
        vb: candle_nn::VarBuilder,
    ) -> Result<(), Gemma4PipelineError> {
        let mut model = self
            .model
            .lock()
            .map_err(|_| Gemma4PipelineError::Decoder("mutex poisoned".to_string()))?;
        if model.has_vision() {
            return Ok(());
        }
        model
            .attach_vision(vb)
            .map_err(|e| Gemma4PipelineError::Decoder(e.to_string()))
    }

    /// Attach audio weights to a model that was loaded with
    /// `Model::new_text_only`. Requires `cfg.audio_config` to be `Some`.
    /// Idempotent — does nothing if already attached.
    pub fn attach_audio(
        &self,
        vb: candle_nn::VarBuilder,
    ) -> Result<(), Gemma4PipelineError> {
        let mut model = self
            .model
            .lock()
            .map_err(|_| Gemma4PipelineError::Decoder("mutex poisoned".to_string()))?;
        if model.has_audio() {
            return Ok(());
        }
        model
            .attach_audio(vb)
            .map_err(|e| Gemma4PipelineError::Decoder(e.to_string()))
    }

    /// Greedy generation with optional image inputs.
    ///
    /// `pixel_values`: preprocessed `[1, 3, H, W]` f32 tensors in `[0, 1]`.
    /// The prompt should contain `<start_of_image>` tokens (id 258880) where
    /// images are expected — the tokenizer template handles this.
    pub async fn generate_greedy(
        &self,
        prompt_text: &str,
        pixel_values: &[Tensor],
        max_new_tokens: usize,
        eos_token_id: Option<u32>,
    ) -> Result<String, Gemma4PipelineError> {
        let enc = self
            .tokenizer
            .encode(prompt_text, false)
            .map_err(|e| Gemma4PipelineError::Tokenizer(e.to_string()))?;
        let input_ids: Vec<u32> = enc.get_ids().to_vec();
        let prompt_len = input_ids.len();

        let image_token_id = self.cfg.image_token_id as u32;

        // Find positions where image tokens appear (on CPU, no GPU eq needed).
        let image_positions: Vec<usize> = input_ids
            .iter()
            .enumerate()
            .filter(|&(_, id)| *id == image_token_id)
            .map(|(i, _)| i)
            .collect();

        let mut model = self
            .model
            .lock()
            .map_err(|_| Gemma4PipelineError::Decoder("mutex poisoned".to_string()))?;

        // 1. Embed tokens on CPU, transfer to GPU.
        let input_ids_tensor =
            Tensor::from_vec(input_ids.clone(), (1, prompt_len), &Device::Cpu)?;
        let mut embeds = model.language_model.embed_tokens(&input_ids_tensor)?;
        embeds = embeds.to_device(&self.gpu_device)?;

        // 2. If we have images, run vision tower + embedder and splice results.
        if !pixel_values.is_empty() {
            let vision_tower = model.vision_tower.as_ref().ok_or_else(|| {
                Gemma4PipelineError::InvalidInput(
                    "vision tower not attached — call attach_vision() before passing images".into(),
                )
            })?;
            let embed_vision = model.embed_vision.as_ref().ok_or_else(|| {
                Gemma4PipelineError::InvalidInput(
                    "embed_vision not attached — call attach_vision() before passing images".into(),
                )
            })?;
            let vision_features = vision_tower.forward(pixel_values)?;
            let image_embeds = embed_vision
                .forward(&vision_features)?
                .to_dtype(embeds.dtype())?
                .to_device(&self.gpu_device)?;

            // image_embeds: [1, num_vision_tokens, hidden_size]
            // Replace the image token positions in embeds with vision embeddings.
            let num_vision_tokens = image_embeds.dim(1)?;
            if image_positions.len() >= num_vision_tokens {
                let image_embeds_2d = image_embeds.squeeze(0)?;
                embeds = replace_positions(&embeds, &image_positions, &image_embeds_2d)?;
            }
        }

        // 3. Initial forward pass through decoder layers on GPU.
        let hidden = model
            .language_model
            .forward_embeds_hidden(&embeds, 0, 1, prompt_len)?;
        // Transfer to CPU for lm_head. `to_device_async` routes through
        // `WgpuStorage::read_to_cpu_async` when the source is Wgpu so the
        // wasm32 worker doesn't deadlock on the GPU map callback.
        let hidden_cpu = hidden.to_device_async(&Device::Cpu).await?;
        let logits = model.language_model.lm_head(&hidden_cpu)?;
        let mut next_id = argmax_last(&logits)?;
        log_step_diag(0, &logits, next_id, &self.tokenizer);

        let mut generated: Vec<u32> = Vec::with_capacity(max_new_tokens);
        if Some(next_id) == eos_token_id {
            diag_log(&format!(
                "[gemma4] step 0 produced EOS ({next_id}) — generation terminated immediately"
            ));
            return self.decode_tokens(&generated);
        }
        generated.push(next_id);

        // 4. Autoregressive loop — one token at a time.
        for step in 0..max_new_tokens.saturating_sub(1) {
            let token_tensor = Tensor::from_vec(vec![next_id], (1, 1), &Device::Cpu)?;
            let single_embed = model.language_model.embed_tokens(&token_tensor)?;
            let single_embed = single_embed.to_device(&self.gpu_device)?;

            let hidden = model
                .language_model
                .forward_embeds_hidden(&single_embed, prompt_len + step, 1, 1)?;
            let hidden_cpu = hidden.to_device_async(&Device::Cpu).await?;
            let logits = model.language_model.lm_head(&hidden_cpu)?;
            next_id = argmax_last(&logits)?;
            log_step_diag(step + 1, &logits, next_id, &self.tokenizer);

            if Some(next_id) == eos_token_id {
                break;
            }
            generated.push(next_id);
        }

        self.decode_tokens(&generated)
    }

    fn decode_tokens(&self, ids: &[u32]) -> Result<String, Gemma4PipelineError> {
        self.tokenizer
            .decode(ids, true)
            .map_err(|e| Gemma4PipelineError::Tokenizer(e.to_string()))
    }
}

/// Replace specific positions in an embedding tensor with vision embeddings.
///
/// `embeds`: `[1, seq_len, hidden]`
/// `positions`: sorted indices where image tokens appear
/// `vision_embeds`: `[num_vision_tokens, hidden]`
///
/// Fills positions sequentially from `vision_embeds`.
fn replace_positions(
    embeds: &Tensor,
    positions: &[usize],
    vision_embeds: &Tensor,
) -> Result<Tensor, Gemma4PipelineError> {
    let seq_len = embeds.dim(1)?;
    let num_vis = vision_embeds.dim(0)?;
    let to_replace = positions.len().min(num_vis);

    // Build the result by concatenating segments between replaced positions.
    let embeds_2d = embeds.squeeze(0)?;
    let mut parts: Vec<Tensor> = Vec::new();
    let mut prev_end = 0usize;
    for (vi, &pos) in positions.iter().take(to_replace).enumerate() {
        if pos > prev_end {
            parts.push(embeds_2d.narrow(0, prev_end, pos - prev_end)?);
        }
        parts.push(vision_embeds.narrow(0, vi, 1)?);
        prev_end = pos + 1;
    }
    if prev_end < seq_len {
        parts.push(embeds_2d.narrow(0, prev_end, seq_len - prev_end)?);
    }

    let combined = Tensor::cat(&parts, 0)?;
    Ok(combined.unsqueeze(0)?)
}

fn argmax_last(logits: &Tensor) -> Result<u32, Gemma4PipelineError> {
    let (_b, n, _v) = logits.dims3()?;
    let last = logits.narrow(1, n - 1, 1)?.squeeze(1)?.squeeze(0)?;
    let last = match last.dtype() {
        DType::F32 => last,
        _ => last.to_dtype(DType::F32)?,
    };
    let v = last.to_vec1::<f32>()?;
    let mut best_idx = 0usize;
    let mut best_val = f32::NEG_INFINITY;
    for (i, x) in v.iter().enumerate() {
        if *x > best_val {
            best_val = *x;
            best_idx = i;
        }
    }
    Ok(best_idx as u32)
}
