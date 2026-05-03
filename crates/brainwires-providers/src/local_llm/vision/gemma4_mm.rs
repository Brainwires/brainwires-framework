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

// ── Process-global NaN tally ──────────────────────────────────────────────
//
// `nan_scan` invocations record into these statics so external test
// drivers (e.g. the `gemma4_diag` example binary) can detect failure
// without parsing stderr. Tokio task migration across worker threads is
// safe — `AtomicUsize` and `Mutex` are process-global.
//
// Reset before each generate call via `nan_scan_reset`. Read after via
// `nan_scan_count` and `nan_scan_first_label`.

use std::sync::atomic::{AtomicUsize, Ordering};

static NAN_SCAN_COUNT: AtomicUsize = AtomicUsize::new(0);
static NAN_SCAN_FIRST_LABEL: Mutex<Option<String>> = Mutex::new(None);

fn nan_scan_record(label: &str) {
    NAN_SCAN_COUNT.fetch_add(1, Ordering::Relaxed);
    if let Ok(mut slot) = NAN_SCAN_FIRST_LABEL.lock() {
        if slot.is_none() {
            *slot = Some(label.to_string());
        }
    }
}

/// Number of `nan_scan` invocations that observed at least one
/// non-finite cell since the last call to [`nan_scan_reset`].
pub fn nan_scan_count() -> usize {
    NAN_SCAN_COUNT.load(Ordering::Relaxed)
}

/// Label of the first `nan_scan` that observed a non-finite cell since
/// the last reset, if any.
pub fn nan_scan_first_label() -> Option<String> {
    NAN_SCAN_FIRST_LABEL.lock().ok().and_then(|s| s.clone())
}

/// Reset the global NaN tally. Test drivers should call this immediately
/// before invoking a forward pass they want to monitor.
pub fn nan_scan_reset() {
    NAN_SCAN_COUNT.store(0, Ordering::Relaxed);
    if let Ok(mut slot) = NAN_SCAN_FIRST_LABEL.lock() {
        *slot = None;
    }
}

/// One-shot NaN/Inf scan over a tensor's values. Logs `label` with the
/// shape, dtype, count of non-finite cells, abs-max, and a few sample
/// values so we can bisect *where* a forward-pass corruption first
/// appears.
///
/// Async because wgpu→cpu readback on wasm32 is poll-driven and the sync
/// `to_device(&Device::Cpu)` path returns an error on that target. Going
/// through `to_device_async` works for both CPU and WGPU tensors.
async fn nan_scan(label: &str, t: &Tensor) {
    let dims = t.dims().to_vec();
    let dtype = t.dtype();
    let cpu = match t.to_device_async(&Device::Cpu).await {
        Ok(c) => c,
        Err(e) => {
            diag_log(&format!(
                "[gemma4/diag] {label}: to_device_async(Cpu) failed: {e}"
            ));
            return;
        }
    };
    let f32_t = match cpu.to_dtype(DType::F32) {
        Ok(t) => t,
        Err(e) => {
            diag_log(&format!(
                "[gemma4/diag] {label}: to_dtype(F32) failed: {e}"
            ));
            return;
        }
    };
    let flat = match f32_t.flatten_all().and_then(|t| t.to_vec1::<f32>()) {
        Ok(v) => v,
        Err(e) => {
            diag_log(&format!(
                "[gemma4/diag] {label}: flatten/to_vec1 failed: {e}"
            ));
            return;
        }
    };
    let nans = flat.iter().filter(|x| x.is_nan()).count();
    let infs = flat.iter().filter(|x| x.is_infinite()).count();
    let finite_n = flat.len() - nans - infs;
    if nans > 0 || infs > 0 {
        nan_scan_record(label);
    }
    let preview: Vec<String> = flat.iter().take(4).map(|x| format!("{x:.4}")).collect();
    let abs_max = flat
        .iter()
        .filter(|x| x.is_finite())
        .map(|x| x.abs())
        .fold(0.0_f32, f32::max);
    let abs_min_nonzero = flat
        .iter()
        .filter(|x| x.is_finite() && **x != 0.0)
        .map(|x| x.abs())
        .fold(f32::INFINITY, f32::min);
    let abs_min_str = if abs_min_nonzero.is_finite() {
        format!("{abs_min_nonzero:.6}")
    } else {
        "n/a".to_string()
    };
    diag_log(&format!(
        "[gemma4/diag] {label}: shape={dims:?} dtype={dtype:?} \
         nan={nans} inf={infs} finite={finite_n}/{} \
         abs_max={abs_max:.4} abs_min_nonzero={abs_min_str} head=[{}]",
        flat.len(),
        preview.join(", "),
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
        // Non-streaming wrapper — collects everything and returns the
        // full string at the end. For per-token streaming use
        // `generate_greedy_streaming` directly.
        self.generate_greedy_streaming(
            prompt_text,
            pixel_values,
            max_new_tokens,
            eos_token_id,
            |_, _| {},
        )
        .await
    }

    /// Streaming generate — invokes `on_delta(token_id, &decoded_delta)`
    /// after each new token, where `decoded_delta` is the suffix newly
    /// added to the decoded output by that token.
    ///
    /// Returns the full decoded string when generation finishes (either
    /// EOS or `max_new_tokens` reached).
    ///
    /// The callback is sync and may not block — it runs on the same task
    /// as the forward pass. For wasm callers writing to a
    /// `ReadableStreamDefaultController`, that's fine since the
    /// controller's enqueue is sync.
    pub async fn generate_greedy_streaming(
        &self,
        prompt_text: &str,
        pixel_values: &[Tensor],
        max_new_tokens: usize,
        eos_token_id: Option<u32>,
        mut on_delta: impl FnMut(u32, &str),
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
        nan_scan("step0/embeds_cpu", &embeds).await;

        // Compute the Gemma 3n per-layer-input table (if PLE is wired).
        // The table is `[B, T, num_layers, hidden_per_layer]` — small
        // relative to the main embed (hidden_per_layer is typically 256
        // vs hidden=2048-ish). Built on CPU using the CPU input_ids +
        // CPU embeds, then moved to GPU alongside the main embeds.
        let per_layer_inputs_cpu = model
            .language_model
            .compute_per_layer_inputs(&input_ids_tensor, &embeds)?;
        match &per_layer_inputs_cpu {
            Some(t) => nan_scan("step0/per_layer_inputs_cpu", t).await,
            None => diag_log("[gemma4/diag] step0/per_layer_inputs_cpu: None (PLE skipped)"),
        }

        embeds = embeds.to_device(&self.gpu_device)?;
        nan_scan("step0/embeds_gpu", &embeds).await;
        let per_layer_inputs_gpu = match per_layer_inputs_cpu {
            Some(t) => {
                let g = t.to_device(&self.gpu_device)?;
                nan_scan("step0/per_layer_inputs_gpu", &g).await;
                // Per-layer slice scan — `nan` summary per slice tells
                // us whether a specific layer's PLE input is poisoned
                // (e.g. a NaN cell in the OPFS-streamed PLE row that
                // would propagate through `act_fn(gate(h)) * per_layer`
                // into all 1536 dims after `per_layer_projection`).
                let n_layers = self.cfg.text_config.num_hidden_layers;
                for li in 0..n_layers {
                    if let Ok(slice) = g.narrow(2, li, 1).and_then(|s| s.squeeze(2)) {
                        nan_scan(&format!("step0/per_layer_input/{li:02}"), &slice).await;
                    }
                }
                Some(g)
            }
            None => None,
        };

        // Per-layer `layer_scalar` scan. The Gemma 4 checkpoint stores
        // one [1]-shaped tensor per layer; if any of them encode a NaN
        // BF16 bit pattern, `broadcast_mul(scalar)` poisons every cell
        // of that layer's output uniformly. Reads via the candle-fork
        // `TextModel::layer_scalars()` accessor (rev 3f1ab470+).
        for (li, scalar) in model.language_model.layer_scalars().iter().enumerate() {
            match scalar {
                Some(s) => nan_scan(&format!("step0/layer_scalar/{li:02}"), s).await,
                None => diag_log(&format!(
                    "[gemma4/diag] step0/layer_scalar/{li:02}: None (not in checkpoint)"
                )),
            }
        }

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

        // 3. Initial forward pass — decoder + lm_head all on GPU.
        // `forward_embeds_with_per_layer` applies lm_head + final logit
        // softcapping on the device the lm_head weight lives on, which
        // is the GPU device per the candle-fork untie-on-vb.device()
        // change. Per-token cost: one GPU matmul + a ~1 MB
        // [1, 1, vocab_size] readback (vs ~hundreds of ms of CPU bf16
        // matmul before).
        //
        // Step-0 diagnostic detour via the hooked variant: we capture
        // every layer's post-state, async-readback each via `nan_scan`,
        // and locate the first non-finite layer if the forward goes
        // bad. Decoder + final norm are emitted by the hooked fn;
        // lm_head is applied separately afterward via `lm_head(hidden)`.
        let mut layer_states: Vec<(usize, Tensor)> = Vec::new();
        // Intra-layer states are only captured for `target_intra_layer`
        // to keep memory and async-readback cost bounded. Test drivers
        // override via the `BW_GEMMA4_DIAG_LAYER` env var ("none"
        // disables intra-capture entirely; any non-numeric value also
        // disables; default is layer 8 since that's the current bug
        // bisection target).
        let target_intra_layer: Option<usize> = match std::env::var("BW_GEMMA4_DIAG_LAYER") {
            Ok(v) if v.eq_ignore_ascii_case("none") => None,
            Ok(v) => v.parse::<usize>().ok(),
            Err(_) => Some(8),
        };
        let mut intra_states: Vec<(usize, String, Tensor)> = Vec::new();
        let hidden_gpu = model.language_model.forward_embeds_hidden_with_intra_hook(
            &embeds,
            per_layer_inputs_gpu.as_ref(),
            0,
            1,
            prompt_len,
            |layer_idx: usize, state: &Tensor| {
                layer_states.push((layer_idx, state.clone()));
            },
            |layer_idx: usize, step: &str, state: &Tensor| {
                if Some(layer_idx) == target_intra_layer {
                    intra_states.push((layer_idx, step.to_string(), state.clone()));
                }
            },
        )?;
        for (idx, state) in &layer_states {
            let label = if *idx >= self.cfg.text_config.num_hidden_layers {
                "step0/layers/altup_consolidate".to_string()
            } else {
                format!("step0/layers/{idx:02}_post")
            };
            nan_scan(&label, state).await;
        }
        for (idx, step, state) in &intra_states {
            nan_scan(&format!("step0/layers/{idx:02}/{step}"), state).await;
        }
        nan_scan("step0/hidden_gpu_pre_lm_head", &hidden_gpu).await;
        let logits_gpu = model.language_model.lm_head(&hidden_gpu)?;
        nan_scan("step0/logits_gpu_pre_readback", &logits_gpu).await;
        let logits = logits_gpu.to_device_async(&Device::Cpu).await?;
        nan_scan("step0/logits_cpu_post_readback", &logits).await;
        let mut next_id = argmax_last(&logits)?;
        log_step_diag(0, &logits, next_id, &self.tokenizer);

        let mut generated: Vec<u32> = Vec::with_capacity(max_new_tokens);
        let mut prev_decoded = String::new();
        if Some(next_id) == eos_token_id {
            diag_log(&format!(
                "[gemma4] step 0 produced EOS ({next_id}) — generation terminated immediately"
            ));
            return self.decode_tokens(&generated);
        }
        generated.push(next_id);
        // Per-token streaming delta. Re-decode the full sequence each
        // step and emit the suffix that's new since last call. O(N²)
        // decode cost over `N = generated.len()` but tolerable at
        // typical chat-generation scales (≤ ~500 tokens). The
        // tokenizer's incremental decoders aren't a stable API across
        // candle's `tokenizers` versions, so this is the portable path.
        if let Ok(full) = self.decode_tokens(&generated) {
            if full.len() > prev_decoded.len() {
                on_delta(next_id, &full[prev_decoded.len()..]);
                prev_decoded = full;
            }
        }

        // 4. Autoregressive loop — one token at a time.
        for step in 0..max_new_tokens.saturating_sub(1) {
            let token_tensor = Tensor::from_vec(vec![next_id], (1, 1), &Device::Cpu)?;
            let single_embed = model.language_model.embed_tokens(&token_tensor)?;
            // Single-token PLE table: same shape rules but with T=1.
            let per_layer_step_cpu = model
                .language_model
                .compute_per_layer_inputs(&token_tensor, &single_embed)?;
            let single_embed = single_embed.to_device(&self.gpu_device)?;
            let per_layer_step_gpu = match per_layer_step_cpu {
                Some(t) => Some(t.to_device(&self.gpu_device)?),
                None => None,
            };

            let logits_gpu = model.language_model.forward_embeds_with_per_layer(
                &single_embed,
                per_layer_step_gpu.as_ref(),
                prompt_len + step,
                1,
                1,
            )?;
            let logits = logits_gpu.to_device_async(&Device::Cpu).await?;
            next_id = argmax_last(&logits)?;
            log_step_diag(step + 1, &logits, next_id, &self.tokenizer);

            if Some(next_id) == eos_token_id {
                break;
            }
            generated.push(next_id);
            if let Ok(full) = self.decode_tokens(&generated) {
                if full.len() > prev_decoded.len() {
                    on_delta(next_id, &full[prev_decoded.len()..]);
                    prev_decoded = full;
                }
            }
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
