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

use std::collections::HashMap;
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

    // intermediate_size from first layer's gate_proj [intermediate_size, hidden_size]
    let intermediate_size = find("language_model.model.layers.0.mlp.gate_proj.weight")
        .map(|s| s[0])
        .ok_or("missing layers.0.mlp.gate_proj.weight")?;

    // num_hidden_layers: count unique layer indices
    let num_hidden_layers = tensor_meta
        .iter()
        .filter_map(|(n, _)| {
            let rest = n.strip_prefix("model.language_model.model.layers.")?;
            rest.split('.').next()?.parse::<usize>().ok()
        })
        .max()
        .map(|m| m + 1)
        .ok_or("no decoder layers found")?;

    // num_attention_heads from q_proj.weight shape[0] / head_dim
    // For Gemma4, global layers use global_head_dim and sliding layers use head_dim.
    // Detect from layer 0's q_proj.
    let q_proj_shape = find("language_model.model.layers.0.self_attn.q_proj.weight")
        .ok_or("missing layers.0 q_proj")?;
    let kv_proj_shape = find("language_model.model.layers.0.self_attn.k_proj.weight")
        .ok_or("missing layers.0 k_proj")?;

    // Infer layer_types by checking each layer's q_proj shape.
    // Sliding layers have q_proj [num_heads * head_dim, hidden_size]
    // Global layers have q_proj [num_heads * global_head_dim, hidden_size]
    // The first layer is typically sliding in Gemma4's alternating pattern.
    let layer0_q_out = q_proj_shape[0];

    // Check a later layer to find the other head_dim
    let layer1_q_out = find("language_model.model.layers.1.self_attn.q_proj.weight")
        .map(|s| s[0]);

    // Determine head_dim and global_head_dim from the two layer types
    let (head_dim, global_head_dim, num_attention_heads) = if let Some(l1_out) = layer1_q_out {
        if layer0_q_out < l1_out {
            // layer 0 is sliding (smaller head_dim), layer 1 is global
            let num_heads = 8; // Gemma4 default
            let hd = layer0_q_out / num_heads;
            let ghd = l1_out / num_heads;
            (hd, ghd, num_heads)
        } else if layer0_q_out > l1_out {
            let num_heads = 8;
            let hd = l1_out / num_heads;
            let ghd = layer0_q_out / num_heads;
            (hd, ghd, num_heads)
        } else {
            // Same size — use defaults
            (256, 512, 8)
        }
    } else {
        (256, 512, 8)
    };

    let num_key_value_heads = kv_proj_shape[0] / head_dim;

    // Build layer_types: check each layer's q_proj output dimension
    let sliding_q_out = num_attention_heads * head_dim;
    let mut layer_types = Vec::with_capacity(num_hidden_layers);
    for i in 0..num_hidden_layers {
        let key = format!("language_model.model.layers.{i}.self_attn.q_proj.weight");
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

    use brainwires_providers::gemma4::config::*;

    Ok(Gemma4Config {
        text_config: Gemma4TextConfig {
            attention_bias: false,
            head_dim,
            hidden_activation: Activation::GeluPytorchTanh,
            hidden_size,
            intermediate_size,
            num_attention_heads,
            num_hidden_layers,
            num_key_value_heads,
            rms_norm_eps: 1e-6,
            rope_theta: 1_000_000.0,
            vocab_size,
            sliding_window: 4096,
            final_logit_softcapping: None,
            query_pre_attn_scalar: head_dim,
            max_position_embeddings: 131072,
            tie_word_embeddings: true,
            sliding_window_pattern: 6,
            layer_types,
            global_head_dim,
            num_global_key_value_heads: None,
            rope_parameters: None,
            use_bidirectional_attention: None,
            use_flash_attn: false,
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
    })
}

/// Chunked variant of [`init_local_multimodal`]. Reads tensors one at a time
/// via a JS callback, avoiding a single multi-GB allocation.
#[wasm_bindgen]
pub async fn init_local_multimodal_chunked(
    read_fn: Function,
    file_size: f64,
    tokenizer_json: Vec<u8>,
    model_id: String,
) -> Result<LocalMultiModalHandle, JsValue> {
    let file_size = file_size as u64;
    web_sys::console::log_1(
        &format!("[wasm/mm] chunked load: file_size={file_size}, model={model_id}").into(),
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

    // For Gemma4, oversized tensors (embed_tokens ~4.37 GB) go to CPU instead
    // of GPU-direct streaming, because the decoder's mixed-device path runs
    // embed_tokens on CPU and decoder layers on GPU.
    let max_cpu_tensor = isize::MAX as u64;

    let mut tensors: HashMap<String, Tensor> = HashMap::with_capacity(total);
    for (idx, (name, info)) in tensor_meta.iter().enumerate() {
        let offset = data_start + info.data_offsets.0;
        let length = info.data_offsets.1 - info.data_offsets.0;

        let src_dtype = st_dtype_to_candle(&info.dtype).map_err(|e| {
            JsValue::from_str(&format!("tensor {name}: {e}"))
        })?;

        let needs_gpu_stream = length > max_cpu_tensor;
        let force_cpu = model_type == ModelType::Gemma4
            && (name.ends_with("embed_tokens.weight") || name.ends_with("lm_head.weight"));

        let tensor = if force_cpu && needs_gpu_stream {
            // Tensor exceeds wasm32 isize::MAX — stream to GPU even though
            // the mixed-device path wants it on CPU. The pipeline will
            // transfer slices as needed during embed_tokens / lm_head calls.
            let w = wgpu_dev.as_ref().ok_or_else(|| {
                JsValue::from_str(&format!(
                    "tensor {name} is {length} bytes — too large for CPU \
                     and no WebGPU device available"
                ))
            })?;
            load_tensor_to_gpu(&read_fn, offset, length, src_dtype, &info.shape, w)?
        } else if force_cpu {
            let bytes = call_read_fn(&read_fn, offset, length)?;
            let t = Tensor::from_raw_buffer(&bytes, src_dtype, &info.shape, &Device::Cpu)
                .map_err(|e| JsValue::from_str(&format!("tensor {name}: {e}")))?;
            drop(bytes);
            t
        } else if needs_gpu_stream {
            let w = wgpu_dev.as_ref().ok_or_else(|| {
                JsValue::from_str(&format!(
                    "tensor {name} is {length} bytes — too large for wasm32 \
                     and no WebGPU device available for direct upload"
                ))
            })?;
            load_tensor_to_gpu(&read_fn, offset, length, src_dtype, &info.shape, w)?
        } else {
            let bytes = call_read_fn(&read_fn, offset, length)?;
            let t = Tensor::from_raw_buffer(&bytes, src_dtype, &info.shape, &device)
                .map_err(|e| JsValue::from_str(&format!("tensor {name}: {e}")))?;
            drop(bytes);
            t
        };

        let key = name.strip_prefix("model.").unwrap_or(name).to_string();
        tensors.insert(key, tensor);

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

    web_sys::console::log_1(
        &format!("[wasm/mm] all {total} tensors loaded, building {model_type:?} model...").into(),
    );

    let tokenizer = Tokenizer::from_bytes(&tokenizer_json)
        .map_err(|e| JsValue::from_str(&format!("tokenizer parse: {e}")))?;

    let inner = match model_type {
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
            MultimodalInner::Gemma3(Arc::new(pipeline))
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

            let vb = CandleVarBuilder::from_tensors(tensors, DType::BF16, &device);

            let model = Gemma4Model::new(&cfg, vb)
                .map_err(|e| JsValue::from_str(&format!("gemma4 model load: {e}")))?;

            let pipeline = Gemma4MultiModal::from_components(
                model, tokenizer, device.clone(), cfg,
            );
            MultimodalInner::Gemma4 {
                pipeline: Arc::new(pipeline),
                gpu_device: device,
            }
        }
    };

    Ok(LocalMultiModalHandle { inner, model_id })
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
fn build_and_generate_gemma4(
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

    pipeline.generate_greedy(&prompt_text, &pixel_tensors, max_new, eos)
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
