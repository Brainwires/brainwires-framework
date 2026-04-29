//! Multimodal (vision-language) wasm exports for the chat PWA.
//!
//! Companion to the text-only [`crate::LocalModelHandle`] surface in `lib.rs`.
//! Loads a Gemma-family vision-language model (SigLIP tower + MM projector +
//! Gemma-3/4 decoder) and exposes a JS-callable
//! `local_chat_stream_with_image(handle, messages_json, params_json)` that
//! emits the same NDJSON `ReadableStream<Uint8Array>` shape the text path
//! uses — making the JS-side dispatcher in `local-worker.js` route-agnostic.
//!
//! ## JS messages format
//!
//! The JS side sends each message's `content` either as a string OR as an
//! array of parts:
//!
//! ```json
//!   { "role": "user", "content": [
//!       { "type": "text",  "text": "..." },
//!       { "type": "image", "mediaType": "image/jpeg", "data": "<base64>" }
//!   ] }
//! ```
//!
//! This is **not** the framework's Rust [`brainwires_core::message::Message`]
//! shape (which uses `ContentBlock::Image { source: Base64 { media_type, data } }`).
//! We parse the JS shape directly here so the two sides do not have to round-
//! trip through a translator on every chat turn.
//!
//! ## Generation strategy (Stage E v1)
//!
//! Greedy, **one-shot** generation: the wasm side runs the full prompt +
//! `max_tokens` decode under one `spawn_local`, then enqueues a single
//! `delta` chunk and a `finished` chunk. Per-token streaming is gated on
//! follow-up work — the streaming variant requires interleaving sample/
//! forward across an `await` and would push past Stage E's 150-LOC budget
//! while exposing wasm-only blockers in the vendored `gemma3_mm` decoder
//! (RoPE narrowing, sliding-window mask). One-shot is correct,
//! observable on the JS side via `obj.delta`, and the upgrade path to
//! streaming is a single `for_each` substitution in [`run_vision_stream`].

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
use brainwires_providers::{
    CandleDType as DType, CandleDevice as Device, CandleTensor as Tensor, CandleVarBuilder,
};
use candle_nn::Activation;
use js_sys::{Object, Reflect, Uint8Array};
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{ReadableStream, ReadableStreamDefaultController};

// ---------------------------------------------------------------------------
// Multimodal handle
// ---------------------------------------------------------------------------

/// Multimodal Gemma handle. Loaded separately from the text-only
/// [`crate::LocalModelHandle`] because the safetensors file structure differs
/// (text-only vs full vision-language weights). The JS-side worker tracks
/// which shape was loaded and routes `chat` vs `vision_chat` accordingly.
///
/// Disposal: wasm-bindgen autogenerates a JS-side `free()`. Calling it
/// drops the inner `Arc<Gemma3MultiModal>` (and, if last reference, the
/// SigLIP tower + projector + decoder weights).
#[wasm_bindgen]
pub struct LocalMultiModalHandle {
    inner: Arc<Gemma3MultiModal>,
    model_id: String,
}

#[wasm_bindgen]
impl LocalMultiModalHandle {
    #[wasm_bindgen(getter)]
    pub fn model_id(&self) -> String {
        self.model_id.clone()
    }

    /// Returns `"webgpu"` or `"cpu"` so JS can report which device is active.
    /// Mirrors the field name on [`crate::LocalModelHandle`] so the worker
    /// can read it without branching on handle type.
    #[wasm_bindgen(getter)]
    pub fn device_type(&self) -> String {
        match self.inner.device().location() {
            brainwires_providers::CandleDeviceLocation::Cpu => "cpu".into(),
            brainwires_providers::CandleDeviceLocation::Wgpu { .. } => "webgpu".into(),
            _ => "unknown".into(),
        }
    }

    /// Always true; the JS worker uses this to confirm the handle is
    /// multimodal-capable before dispatching `vision_chat`.
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

async fn try_webgpu_device() -> Result<Device, String> {
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
        inner: Arc::new(pipeline),
        model_id,
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

    let pipeline = handle.inner.clone();

    let underlying = Object::new();
    let start_cb = Closure::once_into_js(move |controller: JsValue| {
        let controller: ReadableStreamDefaultController = match controller.dyn_into() {
            Ok(c) => c,
            Err(_) => return,
        };
        spawn_local(run_vision_stream(pipeline, messages, params, controller));
    });
    Reflect::set(&underlying, &JsValue::from_str("start"), &start_cb)
        .map_err(|_| JsValue::from_str("failed to set ReadableStream start callback"))?;

    ReadableStream::new_with_underlying_source(&underlying)
}

/// Runs greedy, one-shot generation and pushes a `delta` + `finished`
/// chunk into the controller. Errors surface as a `{error: "..."}` chunk
/// followed by `controller.error_with_e`, matching the text-only path.
async fn run_vision_stream(
    pipeline: Arc<Gemma3MultiModal>,
    messages: Vec<JsMessage>,
    params: VisionStreamParams,
    controller: ReadableStreamDefaultController,
) {
    match build_and_generate(&pipeline, &messages, &params) {
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

/// Walks the JS-shape messages to extract text segments + decoded image
/// bytes, then drives [`Gemma3MultiModal::generate_greedy`] for the latest
/// user turn.
///
/// Today only the latest message is interpreted multimodally — earlier
/// turns are flattened to text and prefixed onto the prompt. This matches
/// how the JS side composes the prompt for cloud providers and lets us
/// avoid re-running the SigLIP tower over a chat history that already
/// landed text-only deltas in the conversation.
fn build_and_generate(
    pipeline: &Gemma3MultiModal,
    messages: &[JsMessage],
    params: &VisionStreamParams,
) -> Result<String, MmPipelineError> {
    if messages.is_empty() {
        return Err(MmPipelineError::InvalidInput("empty messages".into()));
    }

    // Reset the decoder's KV cache between turns — the wasm crate does not
    // currently retain conversation state across calls.
    pipeline.clear_kv_cache();

    // Split the trailing user turn into text segments + image bytes.
    let last = &messages[messages.len() - 1];
    let mut text_segments: Vec<String> = Vec::new();
    let mut image_bytes: Vec<Vec<u8>> = Vec::new();

    let prefix = build_history_prefix(&messages[..messages.len() - 1], &last.role);

    match &last.content {
        JsContent::Text(t) => {
            text_segments.push(format!("{prefix}{t}"));
        }
        JsContent::Parts(parts) => {
            // Walk parts, splitting at each image. Text segments always
            // bracket images — N images ⇒ N+1 segments.
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
            // Final text segment after the last image (or the only segment if
            // no images).
            text_segments.push(current);
        }
    }

    // Preprocess each image to [1, 3, 896, 896] f32.
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

    // Tie text_segments lifetime to a `&[&str]` slice the pipeline expects.
    let segs_ref: Vec<&str> = text_segments.iter().map(|s| s.as_str()).collect();

    let max_new = params.max_tokens.unwrap_or(256) as usize;
    // EOS for Gemma is conventionally token 1 (`<eos>`). We don't have a
    // public accessor on the pipeline today; passing None means we run to
    // `max_new` and the JS side trims the response as it does for cloud
    // providers. A follow-up can wire `tokenizer.token_to_id("<eos>")`
    // through the `Gemma3MultiModal` API.
    let eos: Option<u32> = None;

    pipeline.generate_greedy(&segs_ref, &images, max_new, eos)
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
