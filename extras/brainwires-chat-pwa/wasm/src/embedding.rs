//! Candle BERT embedder for in-browser RAG.
//!
//! v1: hardcoded gte-small config (384-dim, 12 layers, 12 heads, 1536 ff).
//! When other embedding models land, generalize to deserialize `config.json`
//! that the JS side already downloads as part of the model file list.
//!
//! JS contract (matches `web/src/local-worker.js` and `embeddings.js`):
//!
//! ```text
//! mod.init_embedding_model(weights, tokenizer_json, model_id) → EmbeddingHandle
//! handle.embed_text(text) → Float32Array
//! handle.dim → number   (getter)
//! handle.free()
//! ```

use std::sync::Mutex;

use brainwires_provider::{
    CandleDType as DType, CandleDevice as Device, CandleTensor as Tensor, CandleVarBuilder,
};
use candle_transformers::models::bert::{
    BertModel, Config, HiddenAct, PositionEmbeddingType,
};
use tokenizers::{
    PaddingDirection, PaddingParams, PaddingStrategy, Tokenizer, TruncationDirection,
    TruncationParams, TruncationStrategy,
};
use wasm_bindgen::prelude::*;

/// Hardcoded gte-small config. Confirmed against the model card on HF
/// (thenlper/gte-small). Mean-pool + L2-normalize on top to produce the
/// 384-dim sentence embedding.
fn gte_small_config() -> Config {
    Config {
        vocab_size: 30522,
        hidden_size: 384,
        num_hidden_layers: 12,
        num_attention_heads: 12,
        intermediate_size: 1536,
        hidden_act: HiddenAct::Gelu,
        hidden_dropout_prob: 0.1,
        max_position_embeddings: 512,
        type_vocab_size: 2,
        initializer_range: 0.02,
        layer_norm_eps: 1e-12,
        pad_token_id: 0,
        position_embedding_type: PositionEmbeddingType::Absolute,
        use_cache: true,
        classifier_dropout: None,
        model_type: Some("bert".to_string()),
    }
}

/// Loaded embedder: BERT model on CPU + paired tokenizer.
///
/// Disposal: wasm-bindgen autogenerates a JS-side `free()` method on this
/// struct. Calling `handle.free()` from JS drops the inner `Mutex` guarding
/// the BERT model and tokenizer, releasing the wasm-side memory. The call
/// is idempotent in the same sense as [`LocalModelHandle::free`] — JS
/// callers can wrap it in `try`/`catch` to tolerate double-free during
/// model swaps.
#[wasm_bindgen]
pub struct EmbeddingHandle {
    inner: Mutex<EmbedderInner>,
    dim: usize,
    model_id: String,
}

struct EmbedderInner {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

#[wasm_bindgen]
impl EmbeddingHandle {
    /// Output dimension (384 for gte-small). Read by JS via `handle.dim`.
    #[wasm_bindgen(getter)]
    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Model id this handle was loaded from.
    #[wasm_bindgen(getter, js_name = modelId)]
    pub fn model_id(&self) -> String {
        self.model_id.clone()
    }

    /// Encode a single string. Mean-pools over the token dimension with the
    /// attention mask applied, then L2-normalizes. Returns a Float32Array of
    /// length `dim`.
    #[wasm_bindgen]
    pub fn embed_text(&self, text: String) -> Result<js_sys::Float32Array, JsValue> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| JsValue::from_str("EmbeddingHandle poisoned"))?;
        let v = inner
            .embed_one(&text)
            .map_err(|e| JsValue::from_str(&format!("embed_text: {e}")))?;
        let arr = js_sys::Float32Array::new_with_length(v.len() as u32);
        arr.copy_from(&v);
        Ok(arr)
    }
}

impl EmbedderInner {
    fn embed_one(&mut self, text: &str) -> Result<Vec<f32>, String> {
        // Configure truncation to max_position_embeddings (512). Padding for a
        // single sequence is a no-op (BatchLongest pads to longest in batch),
        // but we still pre-set it so future batched APIs share the policy.
        self.tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: 512,
                strategy: TruncationStrategy::LongestFirst,
                stride: 0,
                direction: TruncationDirection::Right,
            }))
            .map_err(|e| format!("tokenizer truncation: {e}"))?;
        self.tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            direction: PaddingDirection::Right,
            pad_to_multiple_of: None,
            pad_id: 0,
            pad_type_id: 0,
            pad_token: "[PAD]".into(),
        }));

        let enc = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| format!("tokenizer encode: {e}"))?;
        let ids: Vec<i64> = enc.get_ids().iter().map(|&x| x as i64).collect();
        let attn: Vec<i64> = enc
            .get_attention_mask()
            .iter()
            .map(|&x| x as i64)
            .collect();
        let toks: Vec<i64> = enc.get_type_ids().iter().map(|&x| x as i64).collect();
        let n = ids.len();

        let input_ids = Tensor::from_vec(ids, (1, n), &self.device).map_err(|e| e.to_string())?;
        let token_type_ids =
            Tensor::from_vec(toks, (1, n), &self.device).map_err(|e| e.to_string())?;
        let attn_mask =
            Tensor::from_vec(attn, (1, n), &self.device).map_err(|e| e.to_string())?;

        let hidden = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attn_mask))
            .map_err(|e| e.to_string())?; // [1, n, hidden]

        // Mean-pool with attention mask: sum(hidden * mask) / sum(mask).
        let mask_f = attn_mask
            .to_dtype(DType::F32)
            .map_err(|e| e.to_string())?
            .unsqueeze(2)
            .map_err(|e| e.to_string())?; // [1, n, 1]
        let masked = hidden.broadcast_mul(&mask_f).map_err(|e| e.to_string())?;
        let summed = masked.sum(1).map_err(|e| e.to_string())?; // [1, hidden]
        let counts = mask_f.sum(1).map_err(|e| e.to_string())?; // [1, 1]
        let pooled = summed.broadcast_div(&counts).map_err(|e| e.to_string())?; // [1, hidden]

        // L2 normalize.
        let sq = pooled.sqr().map_err(|e| e.to_string())?;
        let norm = sq
            .sum_keepdim(1)
            .map_err(|e| e.to_string())?
            .sqrt()
            .map_err(|e| e.to_string())?
            .clamp(1e-12_f32, f32::INFINITY)
            .map_err(|e| e.to_string())?;
        let normed = pooled.broadcast_div(&norm).map_err(|e| e.to_string())?;

        let out = normed
            .flatten_all()
            .map_err(|e| e.to_string())?
            .to_vec1::<f32>()
            .map_err(|e| e.to_string())?;
        Ok(out)
    }
}

/// Build an [`EmbeddingHandle`] from JS-supplied bytes. Currently hardcoded to
/// the gte-small architecture; when a second embedding model lands, accept a
/// `config_json: Vec<u8>` arg and dispatch on `model_id`.
#[wasm_bindgen]
pub fn init_embedding_model(
    weights: Vec<u8>,
    tokenizer_json: Vec<u8>,
    model_id: String,
) -> Result<EmbeddingHandle, JsValue> {
    let device = Device::Cpu;
    let cfg = gte_small_config();
    let dim = cfg.hidden_size;

    let vb = CandleVarBuilder::from_buffered_safetensors(weights, DType::F32, &device)
        .map_err(|e| JsValue::from_str(&format!("init_embedding_model varbuilder: {e}")))?;
    let model = BertModel::load(vb, &cfg)
        .map_err(|e| JsValue::from_str(&format!("init_embedding_model load: {e}")))?;
    let tokenizer = Tokenizer::from_bytes(&tokenizer_json)
        .map_err(|e| JsValue::from_str(&format!("init_embedding_model tokenizer: {e}")))?;

    Ok(EmbeddingHandle {
        inner: Mutex::new(EmbedderInner {
            model,
            tokenizer,
            device,
        }),
        dim,
        model_id,
    })
}
