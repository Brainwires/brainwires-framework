//! End-to-end vision-language inference for Gemma 3/4.
//!
//! Pulls together the four pieces from Stages A–C and the vendored decoder
//! from Stage D's [`super::gemma3_mm`]:
//!
//! ```text
//!   ImageInput.pixel_values  ── SigLIP tower ──→ [B, 4096, 1152]
//!                                  ↓
//!                        MultiModalProjector  ──→ [B, 256, hidden_size]
//!
//!   text_segments ── tokenizer ──→ token streams
//!                                  ↓
//!                  splice_image_token_block      ──→ unified [N] token stream
//!                                  ↓
//!         decoder.embed_tokens.forward      ──→ [B, N, hidden_size]
//!                                  ↓
//!     overwrite placeholder rows with projector output
//!                                  ↓
//!                  decoder.forward_embeds        ──→ logits → next token
//! ```
//!
//! Stage D ships **greedy** generation. Sampling, temperature, top-p and
//! streaming arrive in Stage E via the existing wasm chat-stream framework.
//!
//! # KV-cache handling
//!
//! Candle's `Attention` carries its KV cache as an inline field, so the
//! `Gemma3MMModel` must be `&mut self` for every forward pass. We wrap it in
//! a [`Mutex`] so `Gemma3MultiModal` can stay `Sync` and be parked on a
//! channel alongside the (read-only) tower + projector. The mutex is
//! single-flight by construction — no two `forward_embeds` calls overlap.

use std::sync::Mutex;

use candle_core::Module;
use thiserror::Error;
use tokenizers::Tokenizer;

use super::gemma3_mm::{Config as Gemma3Config, Model as Gemma3MMModel};
use super::projector::{MultiModalProjector, ProjectorError};
use super::siglip::{SiglipError, SiglipVisionTower};
use super::tokens::{
    splice_image_token_block, GEMMA_IMAGE_TOKEN_COUNT,
};

use crate::CandleDType as DType;
use crate::CandleDevice as Device;
use crate::CandleTensor as Tensor;

/// Errors emitted by the multimodal pipeline.
#[derive(Debug, Error)]
pub enum MmPipelineError {
    /// SigLIP vision tower failure (load or forward).
    #[error("siglip: {0}")]
    Siglip(#[from] SiglipError),

    /// MultiModalProjector failure (load or forward).
    #[error("projector: {0}")]
    Projector(#[from] ProjectorError),

    /// Decoder forward / KV-cache failure.
    #[error("decoder: {0}")]
    Decoder(String),

    /// Tokenizer encode / decode failure.
    #[error("tokenizer: {0}")]
    Tokenizer(String),

    /// Generic tensor-shape / device error from Candle.
    #[error("tensor: {0}")]
    Tensor(String),

    /// Caller passed mismatched `text_segments` / `images` lengths.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl From<candle_core::Error> for MmPipelineError {
    fn from(e: candle_core::Error) -> Self {
        MmPipelineError::Tensor(e.to_string())
    }
}

/// One image attachment in a chat turn. The caller has already preprocessed
/// the bytes into a `[1, 3, 896, 896]` tensor (Stage A's helper). The
/// pipeline runs SigLIP + projector at inference time.
pub struct ImageInput<'a> {
    /// Preprocessed pixel tensor of shape `[1, 3, 896, 896]`, f32, normalized
    /// with SigLIP mean/std (see [`super::preprocess`]).
    pub pixel_values: &'a Tensor,
}

/// One inference unit. Owns the full multimodal stack: vision tower,
/// projector, decoder, tokenizer, plus the runtime cache state (KV cache).
pub struct Gemma3MultiModal {
    vision: SiglipVisionTower,
    projector: MultiModalProjector,
    decoder: Mutex<Gemma3MMModel>,
    tokenizer: Tokenizer,
    device: Device,
    config: Gemma3Config,
}

impl Gemma3MultiModal {
    /// Build a pipeline from already-loaded components.
    ///
    /// Loading individual safetensors / tokenizer files belongs to the
    /// chat-PWA's model-load layer — this constructor takes the assembled
    /// pieces so unit tests can exercise the pipeline with synthetic
    /// weights and a tiny tokenizer. Stage E wires the real loader.
    pub fn from_components(
        vision: SiglipVisionTower,
        projector: MultiModalProjector,
        decoder: Gemma3MMModel,
        tokenizer: Tokenizer,
        device: Device,
        config: Gemma3Config,
    ) -> Self {
        Self {
            vision,
            projector,
            decoder: Mutex::new(decoder),
            tokenizer,
            device,
            config,
        }
    }

    /// Device the pipeline runs on.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Reference to the underlying Gemma3 config.
    pub fn config(&self) -> &Gemma3Config {
        &self.config
    }

    /// Reset the decoder's KV cache. Call between unrelated turns.
    pub fn clear_kv_cache(&self) {
        if let Ok(mut decoder) = self.decoder.lock() {
            decoder.clear_kv_cache();
        }
    }

    /// Build the unified token stream and the corresponding embedding tensor
    /// for a chat prompt with optional images.
    ///
    /// `text_segments` contains the user's text split at the points where
    /// each image should appear:
    ///   - `images.is_empty()` → `text_segments.len() == 1`
    ///   - `images.len() == k` → `text_segments.len() == k + 1`
    ///
    /// Returns `(embeds, input_ids)` where:
    ///   - `embeds` is `[1, total_seq_len, hidden_size]` — pre-scale, ready
    ///     to feed into [`Gemma3MMModel::forward_embeds`] which applies the
    ///     `sqrt(hidden_size)` scale itself.
    ///   - `input_ids` is the flattened token stream — useful for sampling
    ///     bookkeeping and for tests.
    pub fn build_prompt_embeds(
        &self,
        text_segments: &[&str],
        images: &[ImageInput],
    ) -> Result<(Tensor, Vec<u32>), MmPipelineError> {
        if images.is_empty() {
            if text_segments.len() != 1 {
                return Err(MmPipelineError::InvalidInput(format!(
                    "no images requires exactly 1 text segment, got {}",
                    text_segments.len()
                )));
            }
        } else if text_segments.len() != images.len() + 1 {
            return Err(MmPipelineError::InvalidInput(format!(
                "{} images requires {} text segments, got {}",
                images.len(),
                images.len() + 1,
                text_segments.len()
            )));
        }

        // 1. Tokenize each text segment (no special tokens — we splice manually).
        let mut segment_ids: Vec<Vec<u32>> = Vec::with_capacity(text_segments.len());
        for seg in text_segments {
            let enc = self
                .tokenizer
                .encode(*seg, false)
                .map_err(|e| MmPipelineError::Tokenizer(e.to_string()))?;
            segment_ids.push(enc.get_ids().to_vec());
        }

        // 2. Run vision tower + projector for each image, remember outputs.
        let mut projector_outputs: Vec<Tensor> = Vec::with_capacity(images.len());
        for img in images {
            let vision_features = self.vision.forward(img.pixel_values)?;
            let soft_tokens = self.projector.forward(&vision_features)?;
            // soft_tokens: [1, 256, hidden_size]
            projector_outputs.push(soft_tokens);
        }

        // 3. Build the unified token stream and remember each image's
        //    placeholder range so we can overwrite those rows later.
        let mut tokens: Vec<u32> = Vec::new();
        let mut placeholder_ranges: Vec<std::ops::Range<usize>> =
            Vec::with_capacity(images.len());

        // First text segment.
        tokens.extend_from_slice(&segment_ids[0]);
        for (img_idx, _img) in images.iter().enumerate() {
            // Splice an image-token block at the current end of `tokens`.
            let insert_at = tokens.len();
            let range = splice_image_token_block(&mut tokens, insert_at, None);
            placeholder_ranges.push(range);
            // Then append the next text segment.
            tokens.extend_from_slice(&segment_ids[img_idx + 1]);
        }

        let total_len = tokens.len();
        let hidden_size = self.config.hidden_size;

        // 4. Compute text embeddings via the decoder's embed table.
        let input_ids = Tensor::from_vec(tokens.clone(), (1, total_len), &self.device)?;
        let mut embeds = {
            let decoder = self.decoder.lock().map_err(|_| {
                MmPipelineError::Decoder("decoder mutex poisoned".to_string())
            })?;
            decoder.embed_tokens().forward(&input_ids)?
        };

        // 5. For each image, overwrite the placeholder rows with the
        //    projector's output.
        //
        //    `embeds` shape: [1, total_len, hidden_size]
        //    For each placeholder range r, we replace embeds[:, r, :] with
        //    `projector_outputs[i]` (which is [1, 256, hidden_size]).
        //
        //    Candle has no in-place slice assignment so we rebuild via cat.
        for (i, range) in placeholder_ranges.iter().enumerate() {
            let start = range.start;
            let end = range.end;
            debug_assert_eq!(end - start, GEMMA_IMAGE_TOKEN_COUNT);

            let mut parts: Vec<Tensor> = Vec::with_capacity(3);
            if start > 0 {
                parts.push(embeds.narrow(1, 0, start)?);
            }
            // Match dtype/device of the embeddings stream.
            let soft = projector_outputs[i]
                .to_dtype(embeds.dtype())?
                .to_device(embeds.device())?;
            parts.push(soft);
            if end < total_len {
                parts.push(embeds.narrow(1, end, total_len - end)?);
            }
            embeds = Tensor::cat(&parts, 1)?;
            debug_assert_eq!(embeds.dims(), &[1, total_len, hidden_size]);
        }

        Ok((embeds, tokens))
    }

    /// Greedy single-shot generate. Stops on EOS or after `max_new_tokens`,
    /// whichever comes first.
    ///
    /// Stage D's generator is intentionally minimal — sampling, temperature,
    /// top-p, repetition penalty, and streaming arrive in Stage E via the
    /// chat-stream framework that already handles those concerns for the
    /// text-only path.
    ///
    /// `eos_token_id`: stop token id; if `None`, runs to `max_new_tokens`.
    pub fn generate_greedy(
        &self,
        text_segments: &[&str],
        images: &[ImageInput],
        max_new_tokens: usize,
        eos_token_id: Option<u32>,
    ) -> Result<String, MmPipelineError> {
        let (prompt_embeds, prompt_tokens) =
            self.build_prompt_embeds(text_segments, images)?;
        let prompt_len = prompt_tokens.len();

        let mut generated: Vec<u32> = Vec::with_capacity(max_new_tokens);

        // Initial pass — feed full prompt embeddings, capture last-pos logits.
        let mut next_id = {
            let mut decoder = self.decoder.lock().map_err(|_| {
                MmPipelineError::Decoder("decoder mutex poisoned".to_string())
            })?;
            let logits = decoder.forward_embeds(&prompt_embeds, 0)?;
            argmax_last(&logits)?
        };

        if Some(next_id) == eos_token_id {
            return Ok(self.decode_tokens(&generated)?);
        }
        generated.push(next_id);

        // Subsequent passes — feed one token at a time as a 1-row embedding.
        for step in 0..max_new_tokens.saturating_sub(1) {
            let token_tensor = Tensor::from_vec(vec![next_id], (1, 1), &self.device)?;
            let single_embed = {
                let decoder = self.decoder.lock().map_err(|_| {
                    MmPipelineError::Decoder("decoder mutex poisoned".to_string())
                })?;
                decoder.embed_tokens().forward(&token_tensor)?
            };

            let logits = {
                let mut decoder = self.decoder.lock().map_err(|_| {
                    MmPipelineError::Decoder("decoder mutex poisoned".to_string())
                })?;
                decoder.forward_embeds(&single_embed, prompt_len + step)?
            };
            next_id = argmax_last(&logits)?;
            if Some(next_id) == eos_token_id {
                break;
            }
            generated.push(next_id);
        }

        self.decode_tokens(&generated)
    }

    fn decode_tokens(&self, ids: &[u32]) -> Result<String, MmPipelineError> {
        self.tokenizer
            .decode(ids, true)
            .map_err(|e| MmPipelineError::Tokenizer(e.to_string()))
    }
}

/// Argmax over the last dim of a `[1, 1, vocab]` (or `[1, n, vocab]` —
/// we take the last position) logits tensor, returning a single token id.
fn argmax_last(logits: &Tensor) -> Result<u32, MmPipelineError> {
    let (_b, n, _v) = logits.dims3()?;
    let last = logits.narrow(1, n - 1, 1)?.squeeze(1)?.squeeze(0)?;
    // F32 path; if the model runs in bf16 promote first.
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

#[cfg(test)]
mod tests {
    use super::super::gemma3_mm::Config as Gemma3Config;
    use super::*;
    use candle_core::{DType, Device};
    use candle_nn::{Activation, VarBuilder};

    /// Tiny Gemma3 config that matches the projector output dims.
    fn tiny_gemma_config(hidden_size: usize) -> Gemma3Config {
        Gemma3Config {
            attention_bias: false,
            head_dim: 8,
            hidden_activation: Activation::Gelu,
            hidden_size,
            intermediate_size: hidden_size * 2,
            num_attention_heads: 2,
            num_hidden_layers: 2,
            num_key_value_heads: 1,
            rms_norm_eps: 1e-6,
            rope_theta: 10_000.0,
            rope_local_base_freq: 10_000.0,
            vocab_size: 300_000, // > 256_000 to cover SOI/EOI ids
            final_logit_softcapping: None,
            attn_logit_softcapping: None,
            query_pre_attn_scalar: 8,
            sliding_window: 64,
            sliding_window_pattern: 2,
            max_position_embeddings: 1024,
        }
    }

    /// Trivial whitespace + word-level tokenizer with a fixed tiny vocab.
    /// Built from a serialized JSON spec to avoid the AHashMap-vs-HashMap
    /// API friction on `WordLevelBuilder::vocab` in tokenizers 0.22.
    fn make_dummy_tokenizer() -> Tokenizer {
        let tokenizer_json = r#"{
          "version": "1.0",
          "truncation": null,
          "padding": null,
          "added_tokens": [],
          "normalizer": null,
          "pre_tokenizer": { "type": "Whitespace" },
          "post_processor": null,
          "decoder": null,
          "model": {
            "type": "WordLevel",
            "vocab": {
              "[UNK]": 0,
              "hi": 1,
              "bye": 2,
              "X": 3,
              "Y": 4,
              "hello": 5,
              "world": 6
            },
            "unk_token": "[UNK]"
          }
        }"#;
        Tokenizer::from_bytes(tokenizer_json.as_bytes()).unwrap()
    }

    /// Build a real-but-tiny `Gemma3MultiModal` with all-zero weights. Note
    /// that the SigLIP tower and projector come along for the ride but we
    /// test only the build/embedding paths here — the vision path itself is
    /// covered by Stage B/C tests.
    fn build_pipeline(hidden_size: usize) -> Gemma3MultiModal {
        let device = Device::Cpu;
        let dtype = DType::F32;
        let cfg = tiny_gemma_config(hidden_size);

        let decoder_vb = VarBuilder::zeros(dtype, &device);
        let decoder = Gemma3MMModel::new(false, &cfg, decoder_vb).unwrap();

        // SigLIP zero-weight VarBuilder — paligemma_3b_896 has its own
        // shapes. We don't actually call .forward() in the no-image path.
        let siglip_vb = VarBuilder::zeros(dtype, &device);
        let vision = SiglipVisionTower::load(siglip_vb, device.clone()).unwrap();

        // Projector: vision_hidden=1152 (siglip), text_hidden=hidden_size.
        let proj_vb = VarBuilder::zeros(dtype, &device);
        let projector = MultiModalProjector::load(
            proj_vb.pp("multi_modal_projector"),
            1152,
            hidden_size,
            super::super::projector::DEFAULT_EPS,
        )
        .unwrap();

        let tokenizer = make_dummy_tokenizer();

        Gemma3MultiModal::from_components(
            vision, projector, decoder, tokenizer, device, cfg,
        )
    }

    #[test]
    fn test_text_only_prompt_embeds() {
        let hidden = 16;
        let pipeline = build_pipeline(hidden);
        let (embeds, tokens) =
            pipeline.build_prompt_embeds(&["hello world"], &[]).unwrap();
        // Two whitespace-separated words → 2 tokens.
        assert_eq!(tokens.len(), 2);
        assert_eq!(embeds.dims(), &[1, 2, hidden]);
    }

    #[test]
    fn test_build_prompt_embeds_shape() {
        let hidden = 16;
        let pipeline = build_pipeline(hidden);

        // Construct a fake projector output by *not* calling the SigLIP
        // tower. We do this by going through `build_prompt_embeds` with one
        // image but bypassing the vision call by using a precomputed soft-
        // token tensor in place of `pixel_values`. Since `build_prompt_embeds`
        // unconditionally calls `vision.forward`, we instead exercise a
        // pure-text equivalent and synthesize the splice ourselves to get
        // the same final shape — that's what we assert below.
        //
        // For the genuine tower path, we'd need realistic pixel values; that
        // is covered in the Stage E end-to-end test on real weights.
        //
        // Here we just verify the *empty image* path expands tokens to the
        // expected count: `tokens(hi) + tokens(bye) = 2`.
        let (embeds, tokens) = pipeline
            .build_prompt_embeds(&["hi bye"], &[])
            .unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(embeds.dims(), &[1, 2, hidden]);

        // And the explicit splice contract: 260 image-block tokens land
        // between two single-token segments → total 1 + 260 + 1 = 262. We
        // verify this without running the vision tower by going through
        // `splice_image_token_block` directly.
        let mut t: Vec<u32> = vec![1];
        let pos = t.len();
        let _r = splice_image_token_block(&mut t, pos, None);
        t.push(2);
        assert_eq!(t.len(), 1 + 260 + 1);
    }

    #[test]
    fn test_image_block_count_matches_constant() {
        // Regression guard for the 260-token block layout
        // (NL + SOI + 256 placeholders + EOI + NL).
        let mut t: Vec<u32> = vec![10, 20];
        let r = splice_image_token_block(&mut t, 1, None);
        assert_eq!(r.end - r.start, GEMMA_IMAGE_TOKEN_COUNT);
        assert_eq!(t.len(), 2 + 260);
    }

    #[test]
    fn test_text_segments_image_count_mismatch_errors() {
        let pipeline = build_pipeline(16);
        let device = Device::Cpu;
        let dummy_pixels =
            Tensor::zeros((1, 3, 896, 896), DType::F32, &device).unwrap();
        let img = ImageInput {
            pixel_values: &dummy_pixels,
        };
        // 1 image needs 2 segments, give 1 → must error.
        let err = pipeline
            .build_prompt_embeds(&["only one segment"], &[img])
            .unwrap_err();
        match err {
            MmPipelineError::InvalidInput(_) => {}
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }
}
