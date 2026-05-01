//! Multimodal (vision-language) wasm exports for the chat PWA.
//!
//! Companion to the text-only [`crate::LocalModelHandle`] surface in `lib.rs`.
//! Loads a Gemma-family vision-language model and exposes a JS-callable
//! `local_chat_stream_with_image(handle, messages_json, params_json)` that
//! emits the same NDJSON `ReadableStream<Uint8Array>` shape the text path
//! uses — making the JS-side dispatcher in `local-worker.js` route-agnostic.
//!
//! Supports two model architectures:
//!
//! **Gemma-3** (SigLIP-based): SigLIP tower + MM projector + vendored Gemma3 decoder
//! **Gemma-4** (native vision tower): Gemma4 VisionTower + MultimodalEmbedder + upstream Gemma4 TextModel
//!
//! Model type is auto-detected from safetensors tensor names during chunked loading.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use brainwires_providers::local_llm::candle_provider::default_gemma_e2b_config;
use brainwires_providers::local_llm::vision::{
    Gemma3MultiModal, ImageInput, MmPipelineError, MultiModalProjector, PROJECTOR_DEFAULT_EPS,
    SiglipVisionTower, preprocess_image_bytes,
};
use brainwires_providers::local_llm::vision::gemma3_mm::{
    Config as Gemma3MmConfig, Model as Gemma3MmModel,
};
use brainwires_providers::local_llm::vision::{
    Gemma4MultiModal, Gemma4PipelineError, preprocess_image_for_gemma4,
};
use brainwires_providers::gemma4::config::Gemma4Config;
use brainwires_providers::gemma4::Model as Gemma4Model;
use brainwires_providers::{
    CandleDType as DType, CandleDevice as Device, CandleTensor as Tensor, CandleVarBuilder,
};
use candle_nn::Activation;
use js_sys::{Function, Object, Reflect, Uint8Array};
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{ReadableStream, ReadableStreamDefaultController};

use crate::{StTensorInfo, call_read_fn, load_tensor_to_gpu, st_dtype_to_candle};

// ---------------------------------------------------------------------------
// Multimodal handle
// ---------------------------------------------------------------------------

/// Detected model architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelType {
    Gemma3,
    Gemma4,
}

/// Inner pipeline — either Gemma-3 (SigLIP-based) or Gemma-4 (native vision tower).
enum MultimodalInner {
    Gemma3(Arc<Gemma3MultiModal>),
    Gemma4 {
        pipeline: Arc<Gemma4MultiModal>,
        gpu_device: Device,
    },
}

/// Per-handle state that survives init for on-demand `attach_vision` /
/// `attach_audio` calls. Holds enough context to re-read tensors from the
/// original safetensors file/blob and merge them into the loaded model.
///
/// Lives only on the wasm-bindgen handle (single-threaded), because
/// [`js_sys::Function`] is not `Send` and cannot sit behind `Arc<Mutex<…>>`.
struct Gemma4LazyState {
    read_fn: Function,
    tensor_meta: Vec<(String, StTensorInfo)>,
    data_start: u64,
    cfg: brainwires_providers::gemma4::config::Gemma4Config,
    device: Device,
    wgpu_dev: Option<brainwires_providers::WgpuDevice>,
}

/// Multimodal Gemma handle. Loaded separately from the text-only
/// [`crate::LocalModelHandle`] because the safetensors file structure differs
/// (text-only vs full vision-language weights). The JS-side worker tracks
/// which shape was loaded and routes `chat` vs `vision_chat` accordingly.
///
/// Disposal: wasm-bindgen autogenerates a JS-side `free()`. Calling it
/// drops the inner pipeline (and, if last reference, all model weights).
#[wasm_bindgen]
pub struct LocalMultiModalHandle {
    inner: MultimodalInner,
    model_id: String,
    /// Present iff this is a Gemma4 handle whose vision and/or audio tower
    /// was deferred at init. Consumed to drive `attach_vision`/`attach_audio`.
    lazy: Option<Gemma4LazyState>,
}

#[wasm_bindgen]
impl LocalMultiModalHandle {
    #[wasm_bindgen(getter)]
    pub fn model_id(&self) -> String {
        self.model_id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn device_type(&self) -> String {
        match &self.inner {
            MultimodalInner::Gemma3(p) => {
                match p.device().location() {
                    brainwires_providers::CandleDeviceLocation::Cpu => "cpu".into(),
                    brainwires_providers::CandleDeviceLocation::Wgpu { .. } => "webgpu".into(),
                    _ => "unknown".into(),
                }
            }
            MultimodalInner::Gemma4 { gpu_device, .. } => {
                match gpu_device.location() {
                    brainwires_providers::CandleDeviceLocation::Cpu => "cpu".into(),
                    brainwires_providers::CandleDeviceLocation::Wgpu { .. } => "webgpu".into(),
                    _ => "unknown".into(),
                }
            }
        }
    }

    #[wasm_bindgen(getter)]
    pub fn is_multimodal(&self) -> bool {
        true
    }

    /// Whether the vision tower is currently attached. Returns `true` for
    /// Gemma3 (always loaded eagerly) and reflects actual state for Gemma4.
    #[wasm_bindgen(getter)]
    pub fn has_vision(&self) -> bool {
        match &self.inner {
            MultimodalInner::Gemma3(_) => true,
            MultimodalInner::Gemma4 { pipeline, .. } => pipeline.has_vision(),
        }
    }

    /// Whether the audio tower is currently attached.
    #[wasm_bindgen(getter)]
    pub fn has_audio(&self) -> bool {
        match &self.inner {
            MultimodalInner::Gemma3(_) => false,
            MultimodalInner::Gemma4 { pipeline, .. } => pipeline.has_audio(),
        }
    }

    /// Stream the vision-tower tensors from the original safetensors file
    /// and attach them to the loaded Gemma4 model. Idempotent; a no-op if
    /// vision is already attached.
    ///
    /// Errors if the handle is Gemma3 (vision is always eager there) or if
    /// no lazy state was retained at init.
    pub async fn attach_vision(&self) -> Result<(), JsValue> {
        let pipeline = match &self.inner {
            MultimodalInner::Gemma3(_) => {
                return Err(JsValue::from_str(
                    "attach_vision is only supported for Gemma4 handles",
                ));
            }
            MultimodalInner::Gemma4 { pipeline, .. } => pipeline.clone(),
        };
        if pipeline.has_vision() {
            return Ok(());
        }
        let lazy = self.lazy.as_ref().ok_or_else(|| {
            JsValue::from_str("attach_vision: handle has no retained lazy state")
        })?;
        let vb = build_subset_var_builder(lazy, is_vision_tensor)?;
        pipeline
            .attach_vision(vb)
            .map_err(|e| JsValue::from_str(&format!("attach_vision: {e}")))?;
        web_sys::console::log_1(&"[wasm/mm] vision tower attached".into());
        Ok(())
    }

    /// Stream the audio-tower tensors from the original safetensors file
    /// and attach them to the loaded Gemma4 model. Idempotent.
    ///
    /// Currently errors with "audio config not inferred" — `build_gemma4_config`
    /// synthesizes `audio_config: None`, so the model has no audio shapes
    /// to validate against. Wiring this end-to-end requires inferring the
    /// `Gemma4AudioConfig` from `audio_tower.*` tensor shapes; the candle-fork
    /// side (`Gemma4Model::attach_audio`) is already in place for when that
    /// inference lands.
    pub async fn attach_audio(&self) -> Result<(), JsValue> {
        let pipeline = match &self.inner {
            MultimodalInner::Gemma3(_) => {
                return Err(JsValue::from_str(
                    "attach_audio is only supported for Gemma4 handles",
                ));
            }
            MultimodalInner::Gemma4 { pipeline, .. } => pipeline.clone(),
        };
        if pipeline.has_audio() {
            return Ok(());
        }
        let lazy = self.lazy.as_ref().ok_or_else(|| {
            JsValue::from_str("attach_audio: handle has no retained lazy state")
        })?;
        if lazy.cfg.audio_config.is_none() {
            return Err(JsValue::from_str(
                "attach_audio: audio_config not inferred from tensor shapes — \
                 audio support is not yet wired through the Gemma4 config builder",
            ));
        }
        let vb = build_subset_var_builder(lazy, is_audio_tensor)?;
        pipeline
            .attach_audio(vb)
            .map_err(|e| JsValue::from_str(&format!("attach_audio: {e}")))?;
        web_sys::console::log_1(&"[wasm/mm] audio tower attached".into());
        Ok(())
    }
}

/// Stream the subset of tensors matching `predicate` from the safetensors
/// file pointed at by `lazy.read_fn`, run them through `gemma4_remap_key`,
/// and return a `CandleVarBuilder` ready for `Model::attach_vision/audio`.
fn build_subset_var_builder<'a>(
    lazy: &'a Gemma4LazyState,
    predicate: fn(&str) -> bool,
) -> Result<CandleVarBuilder<'a>, JsValue> {
    let mut tensors: HashMap<String, Tensor> = HashMap::new();
    let mut total_bytes: u64 = 0;
    let mut count: usize = 0;
    for (name, info) in &lazy.tensor_meta {
        if !predicate(name) {
            continue;
        }
        if gemma4_skip_reason(name).is_some() {
            // QAT activation stats etc. — skip even on lazy attach.
            continue;
        }
        let tensor = load_one_tensor(
            name,
            info,
            lazy.data_start,
            &lazy.read_fn,
            &lazy.device,
            lazy.wgpu_dev.as_ref(),
            false,
        )?;
        let key = gemma4_remap_key(name);
        let length = info.data_offsets.1 - info.data_offsets.0;
        total_bytes += length;
        count += 1;
        tensors.insert(key, tensor);
    }
    web_sys::console::log_1(
        &format!(
            "[wasm/mm] attach: streamed {count} tensors ({:.2} MB)",
            total_bytes as f64 / 1_048_576.0,
        )
        .into(),
    );
    Ok(CandleVarBuilder::from_tensors(
        tensors,
        DType::BF16,
        &lazy.device,
    ))
}

// ---------------------------------------------------------------------------
// Loader (bulk-read only)
// ---------------------------------------------------------------------------

/// Conservative default Gemma 4 E2B vision-language config. Mirrors
/// [`default_gemma_e2b_config`] but as the vendored
/// [`Gemma3MmConfig`] type — they have the same fields, but live in
/// different crates (the vendored decoder needed an `embed_tokens()` accessor
/// the upstream `Model` doesn't expose).
fn default_gemma_e2b_mm_config() -> Gemma3MmConfig {
    let txt = default_gemma_e2b_config();
    Gemma3MmConfig {
        attention_bias: txt.attention_bias,
        head_dim: txt.head_dim,
        // Upstream `Activation` type is the same one re-exported by
        // candle-nn; Gemma 4 E2B uses GeluPytorchTanh.
        hidden_activation: Activation::GeluPytorchTanh,
        hidden_size: txt.hidden_size,
        intermediate_size: txt.intermediate_size,
        num_attention_heads: txt.num_attention_heads,
        num_hidden_layers: txt.num_hidden_layers,
        num_key_value_heads: txt.num_key_value_heads,
        rms_norm_eps: txt.rms_norm_eps,
        rope_theta: txt.rope_theta,
        rope_local_base_freq: txt.rope_local_base_freq,
        vocab_size: txt.vocab_size,
        final_logit_softcapping: txt.final_logit_softcapping,
        attn_logit_softcapping: txt.attn_logit_softcapping,
        query_pre_attn_scalar: txt.query_pre_attn_scalar,
        sliding_window: txt.sliding_window,
        sliding_window_pattern: txt.sliding_window_pattern,
        max_position_embeddings: txt.max_position_embeddings,
    }
}

/// SigLIP-So400m hidden size used by the `paligemma_3b_896` preset that the
/// [`SiglipVisionTower`] wraps. Hardcoded because the preset itself is
/// hardcoded — no field on the wrapper exposes it pre-load.
const SIGLIP_HIDDEN: usize = 1152;

// ---------------------------------------------------------------------------
// Model type detection + Gemma4 config inference
// ---------------------------------------------------------------------------

fn detect_model_type(tensor_meta: &[(String, StTensorInfo)]) -> ModelType {
    let has_gemma4_vision = tensor_meta
        .iter()
        .any(|(n, _)| n.contains("vision_tower.patch_embedder"));
    if has_gemma4_vision {
        ModelType::Gemma4
    } else {
        ModelType::Gemma3
    }
}

/// Build a [`Gemma4Config`] by inspecting tensor shapes in the safetensors header.
fn build_gemma4_config(tensor_meta: &[(String, StTensorInfo)]) -> Result<Gemma4Config, String> {
    let find = |suffix: &str| -> Option<&Vec<usize>> {
        tensor_meta
            .iter()
            .find(|(n, _)| n.ends_with(suffix))
            .map(|(_, info)| &info.shape)
    };

    // hidden_size from embed_tokens.weight [vocab_size, hidden_size]
    let embed_shape = find("embed_tokens.weight")
        .ok_or("missing embed_tokens.weight")?;
    let vocab_size = embed_shape[0];
    let hidden_size = embed_shape[1];

    // intermediate_size from first layer's gate_proj [intermediate_size, hidden_size].
    // Gemma4-E2B (Gemma 3n) ships with elastic MLP widths — some layers
    // are 6144-wide, others 12288-wide — so we also build a per-layer
    // override table below. The scalar `intermediate_size` is the layer-0
    // value and is used as a fallback for any layer whose gate_proj is
    // missing from the safetensors index (shouldn't happen in practice).
    let intermediate_size = find("language_model.layers.0.mlp.gate_proj.weight")
        .map(|s| s[0])
        .ok_or("missing layers.0.mlp.gate_proj.weight")?;

    // num_hidden_layers: count unique layer indices
    let num_hidden_layers = tensor_meta
        .iter()
        .filter_map(|(n, _)| {
            let rest = n.strip_prefix("model.language_model.layers.")?;
            rest.split('.').next()?.parse::<usize>().ok()
        })
        .max()
        .map(|m| m + 1)
        .ok_or("no decoder layers found")?;

    // num_attention_heads from q_proj.weight shape[0] / head_dim
    // For Gemma4, global layers use global_head_dim and sliding layers use head_dim.
    // Detect from layer 0's q_proj.
    let q_proj_shape = find("language_model.layers.0.self_attn.q_proj.weight")
        .ok_or("missing layers.0 q_proj")?;
    let kv_proj_shape = find("language_model.layers.0.self_attn.k_proj.weight")
        .ok_or("missing layers.0 k_proj")?;

    // Infer layer_types by checking each layer's q_proj shape.
    // Sliding layers have q_proj [num_heads * head_dim, hidden_size]
    // Global layers have q_proj [num_heads * global_head_dim, hidden_size]
    // The first layer is typically sliding in Gemma4's alternating pattern.
    let layer0_q_out = q_proj_shape[0];

    // Check a later layer to find the other head_dim
    let layer1_q_out = find("language_model.layers.1.self_attn.q_proj.weight")
        .map(|s| s[0]);

    // Determine head_dim and global_head_dim from the two layer types.
    // Gemma 3n's canonical config has a single uniform `head_dim: 256` —
    // there is no separate `global_head_dim`. The "two layers carry
    // different sizes" branch below is left in place for variants that
    // genuinely vary, but if both layers report equal q_proj widths we
    // honor that uniform value rather than fabricating `global_head_dim
    // = 512` from thin air (which produced a 1/√2 attention-scale drift
    // and could shape-mismatch the projections under the right config).
    const NUM_HEADS: usize = 8; // Gemma 3n / Gemma 4 default
    let (head_dim, global_head_dim, num_attention_heads) = if let Some(l1_out) = layer1_q_out {
        if layer0_q_out < l1_out {
            // layer 0 is sliding (smaller head_dim), layer 1 is global
            (layer0_q_out / NUM_HEADS, l1_out / NUM_HEADS, NUM_HEADS)
        } else if layer0_q_out > l1_out {
            (l1_out / NUM_HEADS, layer0_q_out / NUM_HEADS, NUM_HEADS)
        } else {
            // Uniform across layer 0 / layer 1 — most likely a Gemma 3n
            // config with `head_dim: 256` everywhere. Use the actual
            // inferred value for both rather than the (256, 512) default.
            let hd = layer0_q_out / NUM_HEADS;
            (hd, hd, NUM_HEADS)
        }
    } else {
        // Single-layer model (shouldn't happen) — fall back to defaults.
        (256, 256, NUM_HEADS)
    };

    let num_key_value_heads = kv_proj_shape[0] / head_dim;

    // Build layer_types: check each layer's q_proj output dimension
    let sliding_q_out = num_attention_heads * head_dim;
    let mut layer_types = Vec::with_capacity(num_hidden_layers);
    for i in 0..num_hidden_layers {
        let key = format!("language_model.layers.{i}.self_attn.q_proj.weight");
        let is_sliding = tensor_meta
            .iter()
            .find(|(n, _)| n.ends_with(&key))
            .map(|(_, info)| info.shape[0] == sliding_q_out)
            .unwrap_or(i % 2 == 0); // fallback: even=sliding
        layer_types.push(if is_sliding {
            "sliding_attention".to_string()
        } else {
            "full_attention".to_string()
        });
    }

    // Build the per-layer intermediate_size table by reading each layer's
    // gate_proj.weight shape — the first dim is that layer's MLP width.
    // Required for Gemma4-E2B's elastic MLP layout (mix of 6144 and 12288).
    let mut intermediate_sizes_vec = Vec::with_capacity(num_hidden_layers);
    for i in 0..num_hidden_layers {
        let key = format!("language_model.layers.{i}.mlp.gate_proj.weight");
        let size = tensor_meta
            .iter()
            .find(|(n, _)| n.ends_with(&key))
            .map(|(_, info)| info.shape[0])
            .unwrap_or(intermediate_size);
        intermediate_sizes_vec.push(size);
    }
    // Only populate the override when widths actually vary; otherwise the
    // scalar field carries the same information and we keep the config
    // payload smaller.
    let intermediate_sizes = if intermediate_sizes_vec
        .iter()
        .any(|&s| s != intermediate_size)
    {
        Some(intermediate_sizes_vec)
    } else {
        None
    };

    use brainwires_providers::gemma4::config::*;

    Ok(Gemma4Config {
        text_config: Gemma4TextConfig {
            attention_bias: false,
            head_dim,
            hidden_activation: Activation::GeluPytorchTanh,
            hidden_size,
            intermediate_size,
            intermediate_sizes,
            num_attention_heads,
            num_hidden_layers,
            num_key_value_heads,
            rms_norm_eps: 1e-6,
            rope_theta: 1_000_000.0,
            vocab_size,
            // Per the canonical Gemma 3n / Gemma 4 config.json. The earlier
            // value (4096) was a Gemma 2 carry-over and over-extended the
            // RotatingKvCache + sliding-attention mask span on local layers.
            sliding_window: 512,
            // Real config: `final_logit_softcapping: 30.0` (the Gemma 2
            // soft-cap returned in 3n alongside QK-norm). Without it the
            // last-layer logits aren't squashed to the trained range and
            // the sampler sees out-of-distribution magnitudes.
            final_logit_softcapping: Some(30.0),
            query_pre_attn_scalar: head_dim,
            max_position_embeddings: 32768,
            tie_word_embeddings: true,
            // 4 sliding + 1 full per group (Gemma 3n / Gemma 4 layer_types
            // pattern), down from Gemma 3's 5+1.
            sliding_window_pattern: 5,
            layer_types,
            global_head_dim,
            num_global_key_value_heads: None,
            rope_parameters: None,
            use_bidirectional_attention: None,
            use_flash_attn: false,
            // Gemma 3n PLE — width of the per-layer auxiliary input
            // (256 for E2B / E4B per the canonical config). When the
            // safetensors index includes `embed_tokens_per_layer.weight`
            // we infer the actual width from its shape; otherwise we
            // disable PLE so non-Gemma3n configs keep working.
            hidden_size_per_layer_input: find("language_model.embed_tokens_per_layer.weight")
                .map(|s| s[1] / num_hidden_layers),
            vocab_size_per_layer_input: find("language_model.embed_tokens_per_layer.weight")
                .map(|s| s[0]),
            // Gemma 3n AltUp — 4 parallel hidden streams. When the
            // safetensors index includes the `altup_projections.0.weight`
            // tensor we know the model carries AltUp; otherwise we
            // disable it (`altup_num_inputs = 1` collapses the stack
            // path back to the classic single-stream forward).
            altup_num_inputs: find("language_model.altup_projections.0.weight")
                .map(|_| 4)
                .unwrap_or(1),
            altup_active_idx: 0,
            altup_correct_scale: true,
            altup_coef_clip: Some(120.0),
            // Gemma 3n LAuReL low-rank residual — `laurel_rank: 64` per
            // the canonical config.
            laurel_rank: find("language_model.layers.0.laurel.linear_left.weight")
                .map(|s| s[0])
                .unwrap_or(64),
            // Gemma 3n activation sparsity — first 10 layers train at
            // 0.95 (zero the bottom 95% of gate_proj per-token), the
            // rest run dense. Only enable when AltUp is also wired
            // (i.e. we're loading a real Gemma 3n checkpoint).
            activation_sparsity_pattern: find("language_model.altup_projections.0.weight")
                .map(|_| {
                    let mut v = Vec::with_capacity(num_hidden_layers);
                    for i in 0..num_hidden_layers {
                        v.push(if i < 10 { 0.95 } else { 0.0 });
                    }
                    v
                }),
        },
        vision_config: Gemma4VisionConfig {
            hidden_size: 768,
            intermediate_size: 3072,
            num_hidden_layers: 16,
            num_attention_heads: 12,
            num_key_value_heads: 12,
            head_dim: 64,
            hidden_activation: Activation::GeluPytorchTanh,
            rms_norm_eps: 1e-6,
            patch_size: 16,
            position_embedding_size: 10240,
            pooling_kernel_size: 3,
            default_output_length: 280,
            standardize: false,
            rope_parameters: None,
        },
        audio_config: None,
        image_token_id: 258880,
        audio_token_id: 258881,
        video_token_id: 258884,
    })
}

/// Tensors present in the Gemma 3n / Gemma4 QAT safetensors that the
/// candle-fork [`Gemma4Model`] does not reference. Skipping them avoids
/// loading audio tower weights when audio is disabled and dropping the
/// QAT input/output min/max statistics (training-time only — they don't
/// participate in the forward pass).
///
/// Per-Layer Embeddings (PLE) used to be skipped here too, but Phase 2
/// of the Gemma 3n implementation now consumes those tensors via the
/// new `PerLayerEmbedding` module and per-layer side-channel into each
/// decoder layer. They flow through the loader normally.
///
/// Returns `Some(reason)` if the tensor should be skipped, `None` otherwise.
fn gemma4_skip_reason(name: &str) -> Option<&'static str> {
    if name.contains(".audio_tower.") {
        return Some("audio");
    }
    if name.ends_with(".input_min")
        || name.ends_with(".input_max")
        || name.ends_with(".output_min")
        || name.ends_with(".output_max")
    {
        return Some("qat-stat");
    }
    None
}

/// Map a tensor name from the HF Gemma 4 safetensors layout to the path the
/// vendored candle `gemma4::Model` expects.
///
/// Two transformations:
///
/// 1. QAT `.linear.weight` → `.weight`. HF QAT layout wraps each `nn.Linear`
///    so the underlying weight is stored at `.../linear.weight`. The
///    candle-fork uses plain `linear_no_bias`, which expects `.../weight`.
///
/// 2. Insert the missing inner `.model.` segment under `language_model`.
///    `Gemma4Model::new_partial` applies `vb.pp("model")`, then
///    `TextModel::new` applies `vb.pp("model")` *again* — so the candle
///    lookup path for the decoder is `model.language_model.model.<sub>`.
///    The HF safetensors file omits that inner segment (paths are
///    `model.language_model.embed_tokens.weight`, `…layers.X.…`,
///    `…norm.weight`). Insert it here for those three roots. `lm_head` is
///    loaded one level up (`vb.pp("lm_head")`), so it stays as-is.
fn gemma4_remap_key(name: &str) -> String {
    let s = if let Some(stripped) = name.strip_suffix(".linear.weight") {
        format!("{stripped}.weight")
    } else {
        name.to_string()
    };
    if let Some(rest) = s.strip_prefix("model.language_model.") {
        // Anything that lives directly on `Gemma3nTextModel` in HF needs
        // the inner `.model.` segment inserted to match the candle
        // double-`vb.pp("model")` nesting.
        let needs_remap = rest.starts_with("layers.")
            || rest.starts_with("embed_tokens")
            || rest.starts_with("norm.")
            // Gemma 3n top-level tensors (Phase 2 PLE + Phase 3 AltUp).
            || rest.starts_with("per_layer_model_projection")
            || rest.starts_with("per_layer_projection_norm")
            || rest.starts_with("altup_projections.")
            || rest.starts_with("altup_unembed_projections.");
        if needs_remap {
            return format!("model.language_model.model.{rest}");
        }
    }
    s
}

/// Tensors that belong to the Gemma4 vision tower (encoder + projector).
/// Used to either skip them at init (lazy) or load them on `attach_vision`.
fn is_vision_tensor(name: &str) -> bool {
    name.contains(".vision_tower.") || name.contains(".embed_vision.")
}

/// Tensors that belong to the Gemma4 audio tower (encoder + projector).
fn is_audio_tensor(name: &str) -> bool {
    name.contains(".audio_tower.") || name.contains(".embed_audio.")
}

/// Options accepted by `init_local_multimodal_chunked`. Defaults to eager
/// vision (current behavior) and lazy audio (audio is unsupported in the
/// chat UI today, and `build_gemma4_config` synthesizes a `None` audio
/// config regardless).
#[derive(Debug, Clone, Default, Deserialize)]
struct LoadOptions {
    #[serde(default)]
    lazy_vision: bool,
    #[serde(default = "default_true")]
    lazy_audio: bool,
}

fn default_true() -> bool {
    true
}

async fn try_webgpu_device() -> Result<Device, String> {
    let has_gpu = js_sys::Reflect::get(
        &js_sys::global(),
        &JsValue::from_str("navigator"),
    )
    .ok()
    .and_then(|nav| js_sys::Reflect::get(&nav, &JsValue::from_str("gpu")).ok())
    .map_or(false, |gpu| !gpu.is_undefined() && !gpu.is_null());

    if !has_gpu {
        return Err("navigator.gpu not available".into());
    }

    let gpu_device = brainwires_providers::WgpuDevice::new_async()
        .await
        .map_err(|e| format!("{e}"))?;
    Ok(Device::Wgpu(gpu_device))
}

/// Build a [`LocalMultiModalHandle`] from JS-supplied byte buffers.
///
/// `weights` is the contents of a single safetensors file containing the
/// vision tower (`vision_tower.vision_model.*`), the projector
/// (`multi_modal_projector.*`), and the Gemma decoder (`model.*`,
/// `lm_head.*`). One [`CandleVarBuilder`] is built over the whole buffer
/// and sub-prefixed for each component.
///
/// Bulk-read only: the chunked-loader path used for the text-only handle
/// would need prefix-aware tensor routing for vision_tower vs decoder, which
/// is non-trivial enough to defer. With Gemma 4 E2B + SigLIP-So400m the
/// safetensors fits in a single allocation on the `Device::Cpu` path the
/// browser already uses for the text-only chunked fallback (~5 GB for the
/// vision-language weights vs ~10 GB for the standalone decoder).
#[wasm_bindgen]
pub async fn init_local_multimodal(
    weights: Vec<u8>,
    tokenizer_json: Vec<u8>,
    model_id: String,
) -> Result<LocalMultiModalHandle, JsValue> {
    // Try WebGPU first; fall back to CPU. Same policy as the text-only path.
    let device = match try_webgpu_device().await {
        Ok(dev) => {
            web_sys::console::log_1(&"[wasm/mm] using WebGPU device".into());
            dev
        }
        Err(e) => {
            web_sys::console::warn_1(
                &format!("[wasm/mm] WebGPU unavailable ({e}), CPU fallback").into(),
            );
            Device::Cpu
        }
    };

    let cfg = default_gemma_e2b_mm_config();
    let hidden_size = cfg.hidden_size;

    // One VarBuilder over the full safetensors. `from_buffered_safetensors`
    // takes the bytes by value, so we pass `weights` directly — no clone.
    let vb = CandleVarBuilder::from_buffered_safetensors(weights, DType::F32, &device)
        .map_err(|e| JsValue::from_str(&format!("safetensors load: {e}")))?;

    // Build the three sub-models, each rooted at the appropriate prefix.
    let vision = SiglipVisionTower::load(vb.pp("vision_tower").pp("vision_model"), device.clone())
        .map_err(|e| JsValue::from_str(&format!("siglip load: {e}")))?;

    let projector = MultiModalProjector::load(
        vb.pp("multi_modal_projector"),
        SIGLIP_HIDDEN,
        hidden_size,
        PROJECTOR_DEFAULT_EPS,
    )
    .map_err(|e| JsValue::from_str(&format!("projector load: {e}")))?;

    let decoder = Gemma3MmModel::new(false, &cfg, vb)
        .map_err(|e| JsValue::from_str(&format!("decoder load: {e}")))?;

    let tokenizer = Tokenizer::from_bytes(&tokenizer_json)
        .map_err(|e| JsValue::from_str(&format!("tokenizer parse: {e}")))?;

    let pipeline =
        Gemma3MultiModal::from_components(vision, projector, decoder, tokenizer, device, cfg);

    Ok(LocalMultiModalHandle {
        inner: MultimodalInner::Gemma3(Arc::new(pipeline)),
        model_id,
        lazy: None,
    })
}

/// Read one tensor from the JS-backed safetensors file and materialize it
/// onto `device` (or stream it directly to GPU for tensors larger than
/// wasm32's `isize::MAX`). Shared by the bulk init loop and the on-demand
/// `attach_*` methods so the routing logic stays in one place.
fn load_one_tensor(
    name: &str,
    info: &StTensorInfo,
    data_start: u64,
    read_fn: &Function,
    device: &Device,
    wgpu_dev: Option<&brainwires_providers::WgpuDevice>,
    force_cpu: bool,
) -> Result<Tensor, JsValue> {
    let offset = data_start + info.data_offsets.0;
    let length = info.data_offsets.1 - info.data_offsets.0;
    let src_dtype = st_dtype_to_candle(&info.dtype)
        .map_err(|e| JsValue::from_str(&format!("tensor {name}: {e}")))?;

    let needs_gpu_stream = length > (isize::MAX as u64);

    if force_cpu && needs_gpu_stream {
        let w = wgpu_dev.ok_or_else(|| {
            JsValue::from_str(&format!(
                "tensor {name} is {length} bytes — too large for CPU and no \
                 WebGPU device available"
            ))
        })?;
        load_tensor_to_gpu(read_fn, offset, length, src_dtype, &info.shape, w)
    } else if force_cpu {
        let bytes = call_read_fn(read_fn, offset, length)?;
        Tensor::from_raw_buffer(&bytes, src_dtype, &info.shape, &Device::Cpu)
            .map_err(|e| JsValue::from_str(&format!("tensor {name}: {e}")))
    } else if needs_gpu_stream {
        let w = wgpu_dev.ok_or_else(|| {
            JsValue::from_str(&format!(
                "tensor {name} is {length} bytes — too large for wasm32 and no \
                 WebGPU device available for direct upload"
            ))
        })?;
        load_tensor_to_gpu(read_fn, offset, length, src_dtype, &info.shape, w)
    } else {
        let bytes = call_read_fn(read_fn, offset, length)?;
        Tensor::from_raw_buffer(&bytes, src_dtype, &info.shape, device)
            .map_err(|e| JsValue::from_str(&format!("tensor {name}: {e}")))
    }
}

/// Chunked variant of [`init_local_multimodal`]. Reads tensors one at a time
/// via a JS callback, avoiding a single multi-GB allocation.
///
/// `options_js` is an optional JS object: `{lazy_vision?: bool, lazy_audio?: bool}`.
/// When `lazy_vision` is `true`, the vision tower is not loaded at init time;
/// call [`LocalMultiModalHandle::attach_vision`] before sending an image.
#[wasm_bindgen]
pub async fn init_local_multimodal_chunked(
    read_fn: Function,
    file_size: f64,
    tokenizer_json: Vec<u8>,
    model_id: String,
    options_js: JsValue,
) -> Result<LocalMultiModalHandle, JsValue> {
    let options: LoadOptions = if options_js.is_null() || options_js.is_undefined() {
        LoadOptions::default()
    } else {
        serde_wasm_bindgen::from_value(options_js)
            .map_err(|e| JsValue::from_str(&format!("invalid options: {e}")))?
    };

    let file_size = file_size as u64;
    web_sys::console::log_1(
        &format!(
            "[wasm/mm] chunked load: file_size={file_size}, model={model_id}, \
             lazy_vision={}, lazy_audio={}",
            options.lazy_vision, options.lazy_audio,
        )
        .into(),
    );

    let header_size_bytes = call_read_fn(&read_fn, 0, 8)?;
    if header_size_bytes.len() < 8 {
        return Err(JsValue::from_str("failed to read safetensors header size"));
    }
    let header_size =
        u64::from_le_bytes(header_size_bytes[..8].try_into().unwrap());

    let header_bytes = call_read_fn(&read_fn, 8, header_size)?;
    let header_str = std::str::from_utf8(&header_bytes)
        .map_err(|e| JsValue::from_str(&format!("invalid header UTF-8: {e}")))?;

    let raw: HashMap<String, serde_json::Value> = serde_json::from_str(header_str)
        .map_err(|e| JsValue::from_str(&format!("invalid safetensors header: {e}")))?;

    let mut tensor_meta: Vec<(String, StTensorInfo)> = Vec::new();
    for (name, value) in &raw {
        if name == "__metadata__" {
            continue;
        }
        let info: StTensorInfo = serde_json::from_value(value.clone()).map_err(|e| {
            JsValue::from_str(&format!("bad tensor info for {name}: {e}"))
        })?;
        tensor_meta.push((name.clone(), info));
    }
    tensor_meta.sort_by_key(|(_, info)| info.data_offsets.0);

    let total = tensor_meta.len();
    web_sys::console::log_1(
        &format!("[wasm/mm] parsed {total} tensor entries").into(),
    );

    let data_start: u64 = 8 + header_size;

    let device = match try_webgpu_device().await {
        Ok(dev) => {
            web_sys::console::log_1(&"[wasm/mm] chunked load: using WebGPU device".into());
            dev
        }
        Err(e) => {
            web_sys::console::warn_1(
                &format!("[wasm/mm] WebGPU unavailable ({e}), CPU fallback").into(),
            );
            Device::Cpu
        }
    };

    let wgpu_dev = match &device {
        Device::Wgpu(w) => Some(w.clone()),
        _ => None,
    };

    let model_type = detect_model_type(&tensor_meta);
    web_sys::console::log_1(
        &format!("[wasm/mm] detected model type: {model_type:?}").into(),
    );

    let mut tensors: HashMap<String, Tensor> = HashMap::with_capacity(total);
    // (audio_unused, qat-stat, lazy_vision, lazy_audio, total bytes deferred)
    let mut skipped = (0usize, 0usize, 0usize, 0usize, 0u64);
    for (idx, (name, info)) in tensor_meta.iter().enumerate() {
        let length = info.data_offsets.1 - info.data_offsets.0;

        if model_type == ModelType::Gemma4 {
            if let Some(reason) = gemma4_skip_reason(name) {
                match reason {
                    "audio" => skipped.0 += 1,
                    "qat-stat" => skipped.1 += 1,
                    _ => {}
                }
                skipped.4 += length;
                continue;
            }
            if options.lazy_vision && is_vision_tensor(name) {
                skipped.2 += 1;
                skipped.4 += length;
                continue;
            }
            if options.lazy_audio && is_audio_tensor(name) {
                // Audio is also caught by gemma4_skip_reason("audio") above,
                // but keep this guarded in case that filter is relaxed.
                skipped.3 += 1;
                skipped.4 += length;
                continue;
            }
        }

        let force_cpu = model_type == ModelType::Gemma4
            && (name.ends_with("embed_tokens.weight") || name.ends_with("lm_head.weight"));

        let tensor = load_one_tensor(
            name,
            info,
            data_start,
            &read_fn,
            &device,
            wgpu_dev.as_ref(),
            force_cpu,
        )?;

        let key = if model_type == ModelType::Gemma4 {
            gemma4_remap_key(name)
        } else {
            name.strip_prefix("model.").unwrap_or(name).to_string()
        };
        tensors.insert(key, tensor);

        let needs_gpu_stream = length > (isize::MAX as u64);
        if idx % 20 == 0 || idx == total - 1 || needs_gpu_stream || force_cpu || length > 100_000_000 {
            let tag = if needs_gpu_stream {
                " [gpu-direct]"
            } else if force_cpu {
                " [cpu]"
            } else {
                ""
            };
            web_sys::console::log_1(
                &format!(
                    "[wasm/mm] loaded tensor {}/{total}: {name} {:?} [{}] ({length} bytes){tag}",
                    idx + 1,
                    info.shape,
                    info.dtype,
                )
                .into(),
            );
        }
    }

    if model_type == ModelType::Gemma4 {
        let (audio, qat, lazy_v, lazy_a, bytes) = skipped;
        web_sys::console::log_1(
            &format!(
                "[wasm/mm] skipped {} tensors ({} audio-unused, {} QAT-stat, \
                 {} deferred-vision, {} deferred-audio), saved {:.2} GB",
                audio + qat + lazy_v + lazy_a,
                audio, qat, lazy_v, lazy_a,
                bytes as f64 / 1_073_741_824.0,
            )
            .into(),
        );
    }
    web_sys::console::log_1(
        &format!("[wasm/mm] all {total} tensors loaded, building {model_type:?} model...").into(),
    );

    let tokenizer = Tokenizer::from_bytes(&tokenizer_json)
        .map_err(|e| JsValue::from_str(&format!("tokenizer parse: {e}")))?;

    let (inner, lazy) = match model_type {
        ModelType::Gemma3 => {
            let cfg = default_gemma_e2b_mm_config();
            let hidden_size = cfg.hidden_size;
            let vb = CandleVarBuilder::from_tensors(tensors, DType::F32, &device);

            let vision = SiglipVisionTower::load(
                vb.pp("vision_tower").pp("vision_model"),
                device.clone(),
            )
            .map_err(|e| JsValue::from_str(&format!("siglip load: {e}")))?;

            let projector = MultiModalProjector::load(
                vb.pp("multi_modal_projector"),
                SIGLIP_HIDDEN,
                hidden_size,
                PROJECTOR_DEFAULT_EPS,
            )
            .map_err(|e| JsValue::from_str(&format!("projector load: {e}")))?;

            let decoder = Gemma3MmModel::new(false, &cfg, vb)
                .map_err(|e| JsValue::from_str(&format!("decoder load: {e}")))?;

            let pipeline = Gemma3MultiModal::from_components(
                vision, projector, decoder, tokenizer, device, cfg,
            );
            (MultimodalInner::Gemma3(Arc::new(pipeline)), None)
        }
        ModelType::Gemma4 => {
            let cfg = build_gemma4_config(&tensor_meta)
                .map_err(|e| JsValue::from_str(&format!("gemma4 config: {e}")))?;

            web_sys::console::log_1(
                &format!(
                    "[wasm/mm] Gemma4 config: hidden={}, layers={}, heads={}, head_dim={}, global_head_dim={}",
                    cfg.text_config.hidden_size,
                    cfg.text_config.num_hidden_layers,
                    cfg.text_config.num_attention_heads,
                    cfg.text_config.head_dim,
                    cfg.text_config.global_head_dim,
                )
                .into(),
            );

            // Gemma4-E2B's embed_tokens / lm_head weights weigh ~800 MB
            // each in bf16. They were loaded onto CPU above (force_cpu in
            // load_one_tensor) so they don't eat WebGPU memory, but the
            // default `HashMap`-backed VarBuilder always calls
            // `tensor.to_device(dev)` on every fetch — silently undoing the
            // CPU placement and causing the runtime device-mismatch we hit
            // in `index_select` (`embed_tokens` weight on Wgpu, the input
            // ids constructed on Cpu in `Gemma4MultiModal::generate_greedy`).
            //
            // `Gemma4MultiModal::generate_greedy` is built around mixed-
            // device execution: embed_tokens / lm_head on CPU, decoder on
            // GPU, with explicit `to_device` shuffles around `forward_embeds_hidden`.
            // To make that design actually take effect we route the
            // VarBuilder through a small backend that honors the loaded
            // device for a pinned set of names and falls back to the
            // default to-device behavior for everything else.
            // The HashMap key is the post-`gemma4_remap_key` name; the
            // VarBuilder path (`vb.pp("model").pp("language_model")
            // .pp("model").pp("embed_tokens")`) lands at exactly the same
            // string. With `tie_word_embeddings: true` (the Gemma4-E2B
            // default) `lm_head` shares this same tensor — pinning it once
            // covers both, so we only need a single entry.
            let cpu_pinned: HashSet<String> =
                ["model.language_model.model.embed_tokens.weight"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
            let vb = CandleVarBuilder::from_backend(
                Box::new(CpuPinnedBackend {
                    inner: tensors,
                    cpu_pinned,
                }),
                DType::BF16,
                device.clone(),
            );

            // `lazy_audio` defaults to true and `cfg.audio_config` is currently
            // synthesized as `None`, so audio is never built at init regardless
            // of the flag. The flag exists for symmetry with vision.
            let with_vision = !options.lazy_vision;
            let with_audio = !options.lazy_audio && cfg.audio_config.is_some();
            let model = Gemma4Model::new_partial(&cfg, vb, with_vision, with_audio)
                .map_err(|e| JsValue::from_str(&format!("gemma4 model load: {e}")))?;

            let pipeline = Gemma4MultiModal::from_components(
                model,
                tokenizer,
                device.clone(),
                cfg.clone(),
            );
            // Retain enough state to honor a later attach_vision/attach_audio call.
            let lazy_state = Gemma4LazyState {
                read_fn: read_fn.clone(),
                tensor_meta: tensor_meta.clone(),
                data_start,
                cfg,
                device: device.clone(),
                wgpu_dev: wgpu_dev.clone(),
            };
            (
                MultimodalInner::Gemma4 {
                    pipeline: Arc::new(pipeline),
                    gpu_device: device,
                },
                Some(lazy_state),
            )
        }
    };

    Ok(LocalMultiModalHandle {
        inner,
        model_id,
        lazy,
    })
}

// ---------------------------------------------------------------------------
// Streaming chat
// ---------------------------------------------------------------------------

/// Subset of [`brainwires_core::provider::ChatOptions`] the JS side passes
/// through. Cloud-only fields (cache strategy, etc.) do not apply here.
#[derive(Debug, Clone, Default, Deserialize)]
struct VisionStreamParams {
    #[serde(default)]
    max_tokens: Option<u32>,
}

/// JS-side message shape. `content` is either a plain string OR an array of
/// `{type: 'text'|'image', ...}` parts. We accept both via `untagged` and the
/// `JsContent` enum below.
#[derive(Debug, Clone, Deserialize)]
struct JsMessage {
    role: String,
    content: JsContent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum JsContent {
    Text(String),
    Parts(Vec<JsPart>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum JsPart {
    Text {
        text: String,
    },
    Image {
        #[allow(dead_code)]
        #[serde(default)]
        media_type: Option<String>,
        #[allow(dead_code)]
        #[serde(default, rename = "mediaType")]
        media_type_camel: Option<String>,
        data: String,
    },
}

/// Wire-format chunk emitted into the [`ReadableStream`]. Mirrors the
/// text-only path's `WireChunk` — same fields, same NDJSON contract.
#[derive(Debug, Clone, Default, Serialize)]
struct VisionWireChunk<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    delta: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    finished: bool,
}

/// Drive a multimodal chat against a loaded [`LocalMultiModalHandle`].
///
/// `messages_json` is a JSON array of `{role, content}` where `content` is
/// either a string OR `[{type:'text'|'image', ...}]`.
/// `params_json` is `{ max_tokens? }`.
///
/// Returns a `ReadableStream<Uint8Array>` of NDJSON-encoded
/// [`VisionWireChunk`]s — the same shape `local_chat_stream` produces, so
/// the JS-side reader in `local-worker.js#runChatStream` is reused
/// verbatim.
#[wasm_bindgen]
pub fn local_chat_stream_with_image(
    handle: &LocalMultiModalHandle,
    messages_json: String,
    params_json: String,
) -> Result<ReadableStream, JsValue> {
    let messages: Vec<JsMessage> = serde_json::from_str(&messages_json)
        .map_err(|e| JsValue::from_str(&format!("messages_json parse: {e}")))?;
    let params: VisionStreamParams = if params_json.trim().is_empty() {
        VisionStreamParams::default()
    } else {
        serde_json::from_str(&params_json)
            .map_err(|e| JsValue::from_str(&format!("params_json parse: {e}")))?
    };

    let inner = match &handle.inner {
        MultimodalInner::Gemma3(p) => StreamInner::Gemma3(p.clone()),
        MultimodalInner::Gemma4 { pipeline, .. } => StreamInner::Gemma4(pipeline.clone()),
    };

    let underlying = Object::new();
    let start_cb = Closure::once_into_js(move |controller: JsValue| {
        let controller: ReadableStreamDefaultController = match controller.dyn_into() {
            Ok(c) => c,
            Err(_) => return,
        };
        spawn_local(run_vision_stream(inner, messages, params, controller));
    });
    Reflect::set(&underlying, &JsValue::from_str("start"), &start_cb)
        .map_err(|_| JsValue::from_str("failed to set ReadableStream start callback"))?;

    ReadableStream::new_with_underlying_source(&underlying)
}

/// Cloneable inner for streaming — avoids moving the full handle into spawn_local.
enum StreamInner {
    Gemma3(Arc<Gemma3MultiModal>),
    Gemma4(Arc<Gemma4MultiModal>),
}

/// Runs greedy, one-shot generation and pushes a `delta` + `finished`
/// chunk into the controller. Errors surface as a `{error: "..."}` chunk
/// followed by `controller.error_with_e`, matching the text-only path.
async fn run_vision_stream(
    inner: StreamInner,
    messages: Vec<JsMessage>,
    params: VisionStreamParams,
    controller: ReadableStreamDefaultController,
) {
    let result = match &inner {
        StreamInner::Gemma3(pipeline) => {
            build_and_generate_gemma3(pipeline, &messages, &params)
                .map_err(|e| format!("{e}"))
        }
        StreamInner::Gemma4(pipeline) => {
            build_and_generate_gemma4(pipeline, &messages, &params)
                .await
                .map_err(|e| format!("{e}"))
        }
    };
    match result {
        Ok(text) => {
            enqueue_vision_chunk(
                &controller,
                &VisionWireChunk {
                    delta: Some(&text),
                    ..Default::default()
                },
            );
            enqueue_vision_chunk(
                &controller,
                &VisionWireChunk {
                    finished: true,
                    ..Default::default()
                },
            );
            let _ = controller.close();
        }
        Err(e) => {
            let msg = format!("local_chat_stream_with_image: {e}");
            enqueue_vision_chunk(
                &controller,
                &VisionWireChunk {
                    error: Some(msg.clone()),
                    finished: true,
                    ..Default::default()
                },
            );
            controller.error_with_e(&JsValue::from_str(&msg));
        }
    }
}

/// Gemma-3 generation: extract text/images, run SigLIP + projector + decoder.
fn build_and_generate_gemma3(
    pipeline: &Gemma3MultiModal,
    messages: &[JsMessage],
    params: &VisionStreamParams,
) -> Result<String, MmPipelineError> {
    if messages.is_empty() {
        return Err(MmPipelineError::InvalidInput("empty messages".into()));
    }

    pipeline.clear_kv_cache();

    let last = &messages[messages.len() - 1];
    let mut text_segments: Vec<String> = Vec::new();
    let mut image_bytes: Vec<Vec<u8>> = Vec::new();

    let prefix = build_history_prefix(&messages[..messages.len() - 1], &last.role);

    match &last.content {
        JsContent::Text(t) => {
            text_segments.push(format!("{prefix}{t}"));
        }
        JsContent::Parts(parts) => {
            let mut current = String::new();
            current.push_str(&prefix);
            for p in parts {
                match p {
                    JsPart::Text { text } => current.push_str(text),
                    JsPart::Image { data, .. } => {
                        text_segments.push(std::mem::take(&mut current));
                        let bytes = BASE64
                            .decode(data.as_bytes())
                            .map_err(|e| MmPipelineError::InvalidInput(format!("base64: {e}")))?;
                        image_bytes.push(bytes);
                    }
                }
            }
            text_segments.push(current);
        }
    }

    let pixel_tensors: Vec<Tensor> = image_bytes
        .iter()
        .map(|b| {
            preprocess_image_bytes(b, pipeline.device())
                .map_err(|e| MmPipelineError::InvalidInput(format!("preprocess: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let images: Vec<ImageInput> = pixel_tensors
        .iter()
        .map(|t| ImageInput { pixel_values: t })
        .collect();

    let segs_ref: Vec<&str> = text_segments.iter().map(|s| s.as_str()).collect();
    let max_new = params.max_tokens.unwrap_or(256) as usize;
    let eos: Option<u32> = None;

    pipeline.generate_greedy(&segs_ref, &images, max_new, eos)
}

/// Gemma-4 generation: extract text/images, run native vision tower + embedder + decoder.
async fn build_and_generate_gemma4(
    pipeline: &Gemma4MultiModal,
    messages: &[JsMessage],
    params: &VisionStreamParams,
) -> Result<String, Gemma4PipelineError> {
    if messages.is_empty() {
        return Err(Gemma4PipelineError::InvalidInput("empty messages".into()));
    }

    pipeline.clear_kv_cache();

    let last = &messages[messages.len() - 1];
    let prefix = build_history_prefix(&messages[..messages.len() - 1], &last.role);

    let mut prompt_text = String::new();
    let mut image_bytes: Vec<Vec<u8>> = Vec::new();

    match &last.content {
        JsContent::Text(t) => {
            prompt_text.push_str(&prefix);
            prompt_text.push_str(t);
        }
        JsContent::Parts(parts) => {
            prompt_text.push_str(&prefix);
            for p in parts {
                match p {
                    JsPart::Text { text } => prompt_text.push_str(text),
                    JsPart::Image { data, .. } => {
                        let bytes = BASE64.decode(data.as_bytes()).map_err(|e| {
                            Gemma4PipelineError::InvalidInput(format!("base64: {e}"))
                        })?;
                        image_bytes.push(bytes);
                    }
                }
            }
        }
    }

    // Preprocess images to [1, 3, target, target] f32 in [0,1].
    // Gemma4 default vision input is 768px (48 patches of 16).
    let target_size = 768u32;
    let pixel_tensors: Vec<Tensor> = image_bytes
        .iter()
        .map(|b| {
            preprocess_image_for_gemma4(b, &Device::Cpu, target_size)
                .map_err(|e| Gemma4PipelineError::InvalidInput(format!("preprocess: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let max_new = params.max_tokens.unwrap_or(256) as usize;
    let eos: Option<u32> = Some(1); // Gemma EOS token

    pipeline
        .generate_greedy(&prompt_text, &pixel_tensors, max_new, eos)
        .await
}

/// Build a `<role>: <text>\n…` prefix from earlier turns. Plain join — same
/// formatter the text-only path uses for `format_prompt`.
fn build_history_prefix(history: &[JsMessage], current_role: &str) -> String {
    let mut buf = String::new();
    for m in history {
        let text = match &m.content {
            JsContent::Text(t) => t.clone(),
            JsContent::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    JsPart::Text { text } => Some(text.as_str()),
                    JsPart::Image { .. } => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        };
        buf.push_str(&m.role);
        buf.push_str(": ");
        buf.push_str(&text);
        buf.push('\n');
    }
    if !buf.is_empty() {
        buf.push_str(current_role);
        buf.push_str(": ");
    }
    buf
}

fn enqueue_vision_chunk(
    controller: &ReadableStreamDefaultController,
    chunk: &VisionWireChunk<'_>,
) {
    let mut bytes = match serde_json::to_vec(chunk) {
        Ok(b) => b,
        Err(_) => return,
    };
    bytes.push(b'\n');
    let view = Uint8Array::from(bytes.as_slice());
    let _ = controller.enqueue_with_chunk(&view);
}

// ── Per-tensor device-pinning VarBuilder backend ─────────────────────
//
// `candle_nn`'s default `SimpleBackend for HashMap<String, Tensor>` calls
// `tensor.to_device(dev)` on every fetch, which silently moves CPU-loaded
// weights onto the VarBuilder's GPU device — defeating the chat-pwa's
// `force_cpu` placement for Gemma4-E2B's 800 MB embed_tokens / lm_head
// table. This backend honors the loaded device for a small allow-list of
// pinned names so the table stays where it was loaded; everything else
// keeps the default to-device behavior.
struct CpuPinnedBackend {
    inner: HashMap<String, Tensor>,
    cpu_pinned: HashSet<String>,
}

impl candle_nn::var_builder::SimpleBackend for CpuPinnedBackend {
    fn get(
        &self,
        s: candle_core::Shape,
        name: &str,
        _: candle_nn::Init,
        dtype: DType,
        dev: &Device,
    ) -> candle_core::Result<Tensor> {
        let tensor = self
            .inner
            .get(name)
            .ok_or_else(|| {
                candle_core::Error::CannotFindTensor {
                    path: name.to_string(),
                }
                .bt()
            })?
            .clone();
        if tensor.shape() != &s {
            Err(candle_core::Error::UnexpectedShape {
                msg: format!("shape mismatch for {name}"),
                expected: s,
                got: tensor.shape().clone(),
            }
            .bt())?
        }
        if self.cpu_pinned.contains(name) {
            // Honor the loaded device; don't move to `dev`.
            tensor.to_dtype(dtype)
        } else {
            tensor.to_device(dev)?.to_dtype(dtype)
        }
    }

    fn get_unchecked(&self, name: &str, dtype: DType, dev: &Device) -> candle_core::Result<Tensor> {
        let tensor = self
            .inner
            .get(name)
            .ok_or_else(|| {
                candle_core::Error::CannotFindTensor {
                    path: name.to_string(),
                }
                .bt()
            })?
            .clone();
        if self.cpu_pinned.contains(name) {
            tensor.to_dtype(dtype)
        } else {
            tensor.to_device(dev)?.to_dtype(dtype)
        }
    }

    fn contains_tensor(&self, name: &str) -> bool {
        self.inner.contains_key(name)
    }
}
