//! Vendored copy of `candle_transformers::models::gemma3` at rev e50d81cb
//! (Brainwires/candle), extended with a `forward_embeds` entry point so the
//! vision-language pipeline can splice projector-produced soft tokens into
//! the input embeddings stream.
//!
//! Future maintenance: when Candle bumps its gemma3 module, cherry-pick the
//! upstream changes onto this file and re-add the `forward_embeds` method
//! (and the `embed_tokens()` accessor) — Candle's stock `Model` keeps
//! `embed_tokens` private and only exposes a token-id forward path, so
//! vision-language inference cannot work with the upstream type as-is.
//!
//! The text-only inference path continues to use the stock
//! [`candle_transformers::models::gemma3::Model`] unchanged. This module is
//! consumed only by [`super::mm_pipeline::Gemma3MultiModal`].

// This module is a near-verbatim copy of upstream Candle code; we suppress the
// crate's missing-docs lint here so we can keep field names / signatures
// matching upstream byte-for-byte for easy cherry-picking.
#![allow(missing_docs)]

use std::sync::Arc;

use candle_core::{DType, Device, Module, Result, Tensor, D};
use candle_nn::{linear_b as linear, Activation, Linear, VarBuilder};

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Config {
    pub attention_bias: bool,
    pub head_dim: usize,
    pub hidden_activation: Activation,
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub num_attention_heads: usize,
    pub num_hidden_layers: usize,
    pub num_key_value_heads: usize,
    pub rms_norm_eps: f64,
    pub rope_theta: f64,
    pub rope_local_base_freq: f64,
    pub vocab_size: usize,
    pub final_logit_softcapping: Option<f64>,
    pub attn_logit_softcapping: Option<f64>,
    pub query_pre_attn_scalar: usize,
    pub sliding_window: usize,
    pub sliding_window_pattern: usize,
    pub max_position_embeddings: usize,
}

#[derive(Debug, Clone)]
struct RmsNorm {
    weight: Tensor,
    eps: f64,
}

impl RmsNorm {
    fn new(dim: usize, eps: f64, vb: VarBuilder) -> Result<Self> {
        let weight = vb.get(dim, "weight")?;
        Ok(Self { weight, eps })
    }
}

impl Module for RmsNorm {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let x_dtype = x.dtype();
        let internal_dtype = match x_dtype {
            DType::F16 | DType::BF16 => DType::F32,
            d => d,
        };
        let hidden_size = x.dim(D::Minus1)?;
        let x = x.to_dtype(internal_dtype)?;
        let norm_x = (x.sqr()?.sum_keepdim(D::Minus1)? / hidden_size as f64)?;
        let x_normed = x.broadcast_div(&(norm_x + self.eps)?.sqrt()?)?;
        x_normed
            .to_dtype(x_dtype)?
            .broadcast_mul(&(&self.weight + 1.0)?)
    }
}

#[derive(Debug, Clone)]
struct RotaryEmbedding {
    sin: Tensor,
    cos: Tensor,
}

impl RotaryEmbedding {
    fn new(
        dtype: DType,
        cfg: &Config,
        dev: &Device,
        sliding_window: Option<usize>,
    ) -> Result<Self> {
        let dim = cfg.head_dim;
        let max_seq_len = cfg.max_position_embeddings;
        let rope_freq = if sliding_window.is_some() {
            cfg.rope_local_base_freq
        } else {
            cfg.rope_theta
        };
        let inv_freq: Vec<_> = (0..dim)
            .step_by(2)
            .map(|i| 1f32 / rope_freq.powf(i as f64 / dim as f64) as f32)
            .collect();
        let inv_freq_len = inv_freq.len();
        let inv_freq = Tensor::from_vec(inv_freq, (1, inv_freq_len), dev)?.to_dtype(dtype)?;
        let t = Tensor::arange(0u32, max_seq_len as u32, dev)?
            .to_dtype(dtype)?
            .reshape((max_seq_len, 1))?;
        let freqs = t.matmul(&inv_freq)?;
        Ok(Self {
            sin: freqs.sin()?,
            cos: freqs.cos()?,
        })
    }

    fn apply_rotary_emb_qkv(
        &self,
        q: &Tensor,
        k: &Tensor,
        seqlen_offset: usize,
    ) -> Result<(Tensor, Tensor)> {
        let (_b_sz, _h, seq_len, _n_embd) = q.dims4()?;
        let cos = self.cos.narrow(0, seqlen_offset, seq_len)?;
        let sin = self.sin.narrow(0, seqlen_offset, seq_len)?;
        let q_embed = candle_nn::rotary_emb::rope(&q.contiguous()?, &cos, &sin)?;
        let k_embed = candle_nn::rotary_emb::rope(&k.contiguous()?, &cos, &sin)?;
        Ok((q_embed, k_embed))
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::upper_case_acronyms)]
struct MLP {
    gate_proj: Linear,
    up_proj: Linear,
    down_proj: Linear,
    act_fn: candle_nn::Activation,
}

impl MLP {
    fn new(cfg: &Config, vb: VarBuilder) -> Result<Self> {
        let hidden_sz = cfg.hidden_size;
        let intermediate_sz = cfg.intermediate_size;
        let gate_proj = linear(hidden_sz, intermediate_sz, false, vb.pp("gate_proj"))?;
        let up_proj = linear(hidden_sz, intermediate_sz, false, vb.pp("up_proj"))?;
        let down_proj = linear(intermediate_sz, hidden_sz, false, vb.pp("down_proj"))?;
        Ok(Self {
            gate_proj,
            up_proj,
            down_proj,
            act_fn: cfg.hidden_activation,
        })
    }
}

impl Module for MLP {
    fn forward(&self, xs: &Tensor) -> Result<Tensor> {
        let lhs = xs.apply(&self.gate_proj)?.apply(&self.act_fn)?;
        let rhs = xs.apply(&self.up_proj)?;
        (lhs * rhs)?.apply(&self.down_proj)
    }
}

#[derive(Debug, Clone)]
enum KvCache {
    Normal(candle_nn::kv_cache::KvCache),
    Rotating(candle_nn::kv_cache::RotatingKvCache),
}

#[derive(Debug, Clone)]
struct Attention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    q_norm: RmsNorm,
    k_norm: RmsNorm,
    num_heads: usize,
    num_kv_heads: usize,
    num_kv_groups: usize,
    head_dim: usize,
    attn_logit_softcapping: Option<f64>,
    rotary_emb: Arc<RotaryEmbedding>,
    kv_cache: KvCache,
    use_flash_attn: bool,
}

impl Attention {
    fn new(
        rotary_emb: Arc<RotaryEmbedding>,
        use_flash_attn: bool,
        cfg: &Config,
        sliding_window: Option<usize>,
        vb: VarBuilder,
    ) -> Result<Self> {
        let hidden_sz = cfg.hidden_size;
        let num_heads = cfg.num_attention_heads;
        let num_kv_heads = cfg.num_key_value_heads;
        let num_kv_groups = num_heads / num_kv_heads;
        let head_dim = cfg.head_dim;
        let bias = cfg.attention_bias;
        let q_proj = linear(hidden_sz, num_heads * head_dim, bias, vb.pp("q_proj"))?;
        let k_proj = linear(hidden_sz, num_kv_heads * head_dim, bias, vb.pp("k_proj"))?;
        let v_proj = linear(hidden_sz, num_kv_heads * head_dim, bias, vb.pp("v_proj"))?;
        let o_proj = linear(num_heads * head_dim, hidden_sz, bias, vb.pp("o_proj"))?;
        let q_norm = RmsNorm::new(head_dim, cfg.rms_norm_eps, vb.pp("q_norm"))?;
        let k_norm = RmsNorm::new(head_dim, cfg.rms_norm_eps, vb.pp("k_norm"))?;
        let kv_cache = if let Some(sliding_window) = sliding_window {
            KvCache::Rotating(candle_nn::kv_cache::RotatingKvCache::new(2, sliding_window))
        } else {
            KvCache::Normal(candle_nn::kv_cache::KvCache::new(
                2,
                cfg.max_position_embeddings,
            ))
        };
        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            q_norm,
            k_norm,
            num_heads,
            num_kv_heads,
            num_kv_groups,
            head_dim,
            attn_logit_softcapping: cfg.attn_logit_softcapping,
            rotary_emb,
            kv_cache,
            use_flash_attn,
        })
    }

    fn forward(
        &mut self,
        xs: &Tensor,
        attention_mask: Option<&Tensor>,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let (b_sz, q_len, _) = xs.dims3()?;

        let query_states = self.q_proj.forward(xs)?;
        let key_states = self.k_proj.forward(xs)?;
        let value_states = self.v_proj.forward(xs)?;

        let query_states = query_states
            .reshape((b_sz, q_len, self.num_heads, self.head_dim))?
            .transpose(1, 2)?;
        let key_states = key_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let value_states = value_states
            .reshape((b_sz, q_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;
        let query_states = self.q_norm.forward(&query_states)?;
        let key_states = self.k_norm.forward(&key_states)?;

        let (query_states, key_states) =
            self.rotary_emb
                .apply_rotary_emb_qkv(&query_states, &key_states, seqlen_offset)?;

        let value_states = value_states.contiguous()?;
        let (key_states, value_states) = match &mut self.kv_cache {
            KvCache::Normal(cache) => cache.append(&key_states, &value_states)?,
            KvCache::Rotating(cache) => cache.append(&key_states, &value_states)?,
        };

        let key_states =
            candle_transformers::utils::repeat_kv(key_states, self.num_kv_groups)?.contiguous()?;
        let value_states =
            candle_transformers::utils::repeat_kv(value_states, self.num_kv_groups)?
                .contiguous()?;

        let attn_output = if self.use_flash_attn {
            // flash-attn expects (b_sz, seq_len, nheads, head_dim)
            let q = query_states.transpose(1, 2)?;
            let k = key_states.transpose(1, 2)?;
            let v = value_states.transpose(1, 2)?;
            let scale = 1f32 / (self.head_dim as f32).sqrt();
            flash_attn(&q, &k, &v, scale, attention_mask.is_some())?.transpose(1, 2)?
        } else {
            let scale = 1f64 / f64::sqrt(self.head_dim as f64);
            let attn_weights = (query_states.matmul(&key_states.transpose(2, 3)?)? * scale)?;

            let attn_weights = match self.attn_logit_softcapping {
                None => attn_weights,
                Some(sc) => ((attn_weights / sc)?.tanh()? * sc)?,
            };

            let attn_weights = match attention_mask {
                None => attn_weights,
                Some(mask) => attn_weights.broadcast_add(mask)?,
            };
            let attn_weights = candle_nn::ops::softmax_last_dim(&attn_weights)?;
            attn_weights.matmul(&value_states)?
        };
        attn_output
            .transpose(1, 2)?
            .reshape((b_sz, q_len, ()))?
            .apply(&self.o_proj)
    }

    fn clear_kv_cache(&mut self) {
        match &mut self.kv_cache {
            KvCache::Normal(c) => c.reset(),
            KvCache::Rotating(c) => c.reset(),
        }
    }
}

// flash-attn is never enabled for the multimodal vendored copy — Candle's
// `flash-attn` cargo feature is a separate crate-level toggle on
// candle-transformers and we don't propagate it here. Keep the unimplemented
// stub so that `use_flash_attn = false` (always, for this module) compiles.
fn flash_attn(_: &Tensor, _: &Tensor, _: &Tensor, _: f32, _: bool) -> Result<Tensor> {
    unimplemented!("flash-attn is not wired into the vendored gemma3_mm copy")
}

#[derive(Debug, Clone)]
struct DecoderLayer {
    self_attn: Attention,
    mlp: MLP,
    input_layernorm: RmsNorm,
    pre_feedforward_layernorm: RmsNorm,
    post_feedforward_layernorm: RmsNorm,
    post_attention_layernorm: RmsNorm,
    sliding_window: Option<usize>,
}

impl DecoderLayer {
    fn new(
        use_flash_attn: bool,
        cfg: &Config,
        vb: VarBuilder,
        sliding_window: Option<usize>,
    ) -> Result<Self> {
        let rotary_emb = Arc::new(RotaryEmbedding::new(
            vb.dtype(),
            cfg,
            vb.device(),
            sliding_window,
        )?);
        let self_attn = Attention::new(
            rotary_emb,
            use_flash_attn,
            cfg,
            sliding_window,
            vb.pp("self_attn"),
        )?;
        let mlp = MLP::new(cfg, vb.pp("mlp"))?;
        let input_layernorm =
            RmsNorm::new(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("input_layernorm"))?;
        let pre_feedforward_layernorm = RmsNorm::new(
            cfg.hidden_size,
            cfg.rms_norm_eps,
            vb.pp("pre_feedforward_layernorm"),
        )?;
        let post_feedforward_layernorm = RmsNorm::new(
            cfg.hidden_size,
            cfg.rms_norm_eps,
            vb.pp("post_feedforward_layernorm"),
        )?;
        let post_attention_layernorm = RmsNorm::new(
            cfg.hidden_size,
            cfg.rms_norm_eps,
            vb.pp("post_attention_layernorm"),
        )?;
        Ok(Self {
            self_attn,
            mlp,
            input_layernorm,
            pre_feedforward_layernorm,
            post_feedforward_layernorm,
            post_attention_layernorm,
            sliding_window,
        })
    }

    fn forward(
        &mut self,
        xs: &Tensor,
        attention_mask: Option<&Tensor>,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let residual = xs;
        let xs = self.input_layernorm.forward(xs)?;
        let xs = self.self_attn.forward(&xs, attention_mask, seqlen_offset)?;
        let xs = xs.apply(&self.post_attention_layernorm)?;
        let xs = (xs + residual)?;
        let residual = &xs;
        let xs = xs.apply(&self.pre_feedforward_layernorm)?;
        let xs = xs.apply(&self.mlp)?;
        let xs = xs.apply(&self.post_feedforward_layernorm)?;
        residual + xs
    }

    fn clear_kv_cache(&mut self) {
        self.self_attn.clear_kv_cache()
    }
}

fn prepare_decoder_attention_mask(
    b_size: usize,
    tgt_len: usize,
    seqlen_offset: usize,
    sliding_window: Option<usize>,
    dtype: DType,
    device: &Device,
) -> Result<Tensor> {
    let mask: Vec<_> = if let Some(sliding_window) = sliding_window {
        (0..tgt_len)
            .flat_map(|i| {
                (0..tgt_len).map(move |j| {
                    if i < j || j + sliding_window < i {
                        f32::NEG_INFINITY
                    } else {
                        0.
                    }
                })
            })
            .collect()
    } else {
        (0..tgt_len)
            .flat_map(|i| (0..tgt_len).map(move |j| if i < j { f32::NEG_INFINITY } else { 0f32 }))
            .collect()
    };
    let mask = Tensor::from_slice(&mask, (tgt_len, tgt_len), device)?;
    let mask = if seqlen_offset > 0 {
        let mask0 = Tensor::zeros((tgt_len, seqlen_offset), DType::F32, device)?;
        Tensor::cat(&[&mask0, &mask], D::Minus1)?
    } else {
        mask
    };
    mask.expand((b_size, 1, tgt_len, tgt_len + seqlen_offset))?
        .to_dtype(dtype)
}

#[derive(Debug, Clone)]
pub struct Model {
    embed_tokens: candle_nn::Embedding,
    layers: Vec<DecoderLayer>,
    norm: RmsNorm,
    lm_head: Linear,
    final_logit_softcapping: Option<f64>,
    device: Device,
    dtype: DType,
    hidden_size: usize,
    sliding_window: usize,
}

impl Model {
    pub fn new(use_flash_attn: bool, cfg: &Config, vb: VarBuilder) -> Result<Self> {
        let vb_m = vb.pp("model");
        let embed_tokens =
            candle_nn::embedding(cfg.vocab_size, cfg.hidden_size, vb_m.pp("embed_tokens"))?;
        let mut layers = Vec::with_capacity(cfg.num_hidden_layers);
        let vb_l = vb_m.pp("layers");
        for layer_idx in 0..cfg.num_hidden_layers {
            let sliding_window = (layer_idx + 1) % cfg.sliding_window_pattern > 0;
            let layer = DecoderLayer::new(
                use_flash_attn,
                cfg,
                vb_l.pp(layer_idx),
                sliding_window.then_some(cfg.sliding_window),
            )?;
            layers.push(layer)
        }
        let norm = RmsNorm::new(cfg.hidden_size, cfg.rms_norm_eps, vb_m.pp("norm"))?;
        let lm_head = Linear::new(embed_tokens.embeddings().clone(), None);
        Ok(Self {
            embed_tokens,
            layers,
            norm,
            lm_head,
            final_logit_softcapping: cfg.final_logit_softcapping,
            device: vb.device().clone(),
            dtype: vb.dtype(),
            hidden_size: cfg.hidden_size,
            sliding_window: cfg.sliding_window,
        })
    }

    /// Token-embedding table accessor — required by the multimodal pipeline,
    /// which computes text-token embeddings itself and then overwrites the
    /// image-placeholder rows with projector output before invoking
    /// [`Self::forward_embeds`].
    pub fn embed_tokens(&self) -> &candle_nn::Embedding {
        &self.embed_tokens
    }

    /// Hidden size (matches `cfg.hidden_size`). Used by the multimodal
    /// pipeline to size pre-computed embedding tensors.
    pub fn hidden_size(&self) -> usize {
        self.hidden_size
    }

    /// Device the decoder weights live on.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Dtype used by the decoder.
    pub fn dtype(&self) -> DType {
        self.dtype
    }

    fn create_attention_masks(
        &self,
        batch_size: usize,
        seq_len: usize,
        seqlen_offset: usize,
    ) -> Result<(Option<Tensor>, Option<Tensor>)> {
        if seq_len <= 1 {
            return Ok((None, None));
        }

        let mask = prepare_decoder_attention_mask(
            batch_size,
            seq_len,
            seqlen_offset,
            None,
            self.dtype,
            &self.device,
        )?;

        let sliding_mask = prepare_decoder_attention_mask(
            batch_size,
            seq_len,
            seqlen_offset,
            Some(self.sliding_window),
            self.dtype,
            &self.device,
        )?;

        Ok((Some(mask), Some(sliding_mask)))
    }

    pub fn forward(&mut self, input_ids: &Tensor, seqlen_offset: usize) -> Result<Tensor> {
        let (b_size, seq_len) = input_ids.dims2()?;
        let xs = self.embed_tokens.forward(input_ids)?;
        let mut xs = (xs * (self.hidden_size as f64).sqrt())?;

        let (attention_mask, sliding_attention_mask) =
            self.create_attention_masks(b_size, seq_len, seqlen_offset)?;

        for layer in self.layers.iter_mut() {
            let mask = if layer.sliding_window.is_some() {
                &sliding_attention_mask
            } else {
                &attention_mask
            };
            xs = layer.forward(&xs, mask.as_ref(), seqlen_offset)?
        }
        let logits = xs
            .narrow(1, seq_len - 1, 1)?
            .apply(&self.norm)?
            .apply(&self.lm_head)?;
        let logits = match self.final_logit_softcapping {
            None => logits,
            Some(sc) => ((logits / sc)?.tanh()? * sc)?,
        };

        Ok(logits)
    }

    /// Like [`Self::forward`] but accepts pre-computed embeddings instead of
    /// token ids.
    ///
    /// `embeds`: shape `[B, seq_len, hidden_size]`. The caller is responsible
    /// for computing text-token embeddings via
    /// `self.embed_tokens().forward(...)` and then overwriting the rows that
    /// correspond to image-token placeholders with the projector's
    /// `[B, 256, hidden_size]` output.
    ///
    /// This method applies the same `sqrt(hidden_size)` post-embedding scale
    /// that [`Self::forward`] applies — callers must NOT pre-scale.
    ///
    /// The attention masks, decoder-layer stack, final RMSNorm, and tied
    /// `lm_head` are identical to [`Self::forward`].
    pub fn forward_embeds(
        &mut self,
        embeds: &Tensor,
        seqlen_offset: usize,
    ) -> Result<Tensor> {
        let (b_size, seq_len, _hidden) = embeds.dims3()?;
        let mut xs = (embeds * (self.hidden_size as f64).sqrt())?;

        let (attention_mask, sliding_attention_mask) =
            self.create_attention_masks(b_size, seq_len, seqlen_offset)?;

        for layer in self.layers.iter_mut() {
            let mask = if layer.sliding_window.is_some() {
                &sliding_attention_mask
            } else {
                &attention_mask
            };
            xs = layer.forward(&xs, mask.as_ref(), seqlen_offset)?
        }
        let logits = xs
            .narrow(1, seq_len - 1, 1)?
            .apply(&self.norm)?
            .apply(&self.lm_head)?;
        let logits = match self.final_logit_softcapping {
            None => logits,
            Some(sc) => ((logits / sc)?.tanh()? * sc)?,
        };

        Ok(logits)
    }

    pub fn clear_kv_cache(&mut self) {
        for layer in self.layers.iter_mut() {
            layer.clear_kv_cache()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device};
    use candle_nn::{Activation, VarBuilder};
    use std::collections::HashMap;

    /// Build a tiny config so the test runs in milliseconds.
    fn tiny_config() -> Config {
        Config {
            attention_bias: false,
            head_dim: 8,
            hidden_activation: Activation::Gelu,
            hidden_size: 16,
            intermediate_size: 32,
            num_attention_heads: 2,
            num_hidden_layers: 2,
            num_key_value_heads: 1,
            rms_norm_eps: 1e-6,
            rope_theta: 10_000.0,
            rope_local_base_freq: 10_000.0,
            vocab_size: 32,
            final_logit_softcapping: None,
            attn_logit_softcapping: None,
            query_pre_attn_scalar: 8,
            sliding_window: 16,
            sliding_window_pattern: 2,
            max_position_embeddings: 64,
        }
    }

    /// Build a zero-weight VarBuilder for the tiny config.
    ///
    /// We only need to seed the embedding table and norm scales —
    /// `linear_b` with `bias=false` and zero weights will load fine because
    /// the `VarBuilder` synthesizes shapes from the requested dim. We use
    /// `VarBuilder::zeros` which returns a zeros-only builder of the right
    /// dtype.
    fn zero_vb(device: &Device, dtype: DType) -> VarBuilder<'static> {
        VarBuilder::zeros(dtype, device)
    }

    /// `forward_embeds(embed_tokens(input_ids), 0)` must equal
    /// `forward(input_ids, 0)` to FP tolerance — they share every step except
    /// the one we factored out.
    #[test]
    fn test_forward_embeds_matches_forward_for_random_input_ids() {
        let device = Device::Cpu;
        let dtype = DType::F32;
        let cfg = tiny_config();

        // Build two identical models so each gets its own KV cache.
        let mut model_a = Model::new(false, &cfg, zero_vb(&device, dtype)).unwrap();
        let mut model_b = Model::new(false, &cfg, zero_vb(&device, dtype)).unwrap();

        // Random input ids, batch=1, seq=5, all in [0, vocab_size).
        let ids = vec![3u32, 7, 1, 0, 5];
        let input_ids = Tensor::from_vec(ids.clone(), (1, ids.len()), &device).unwrap();

        let logits_a = model_a.forward(&input_ids, 0).unwrap();
        let embeds = model_b.embed_tokens().forward(&input_ids).unwrap();
        let logits_b = model_b.forward_embeds(&embeds, 0).unwrap();

        assert_eq!(logits_a.dims(), logits_b.dims());
        let flat_a = logits_a.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        let flat_b = logits_b.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        assert_eq!(flat_a.len(), flat_b.len());
        for (i, (a, b)) in flat_a.iter().zip(flat_b.iter()).enumerate() {
            assert!(
                (a - b).abs() < 1e-5,
                "logits diverge at {i}: forward={a}, forward_embeds={b}"
            );
        }
    }

    /// `forward_embeds` with a hand-built embeds tensor still produces a
    /// correctly-shaped logits output `[B, 1, vocab_size]`.
    #[test]
    fn test_forward_embeds_shape() {
        let device = Device::Cpu;
        let dtype = DType::F32;
        let cfg = tiny_config();
        let mut model = Model::new(false, &cfg, zero_vb(&device, dtype)).unwrap();

        let seq_len = 7;
        let embeds = Tensor::zeros((1, seq_len, cfg.hidden_size), dtype, &device).unwrap();
        let logits = model.forward_embeds(&embeds, 0).unwrap();
        assert_eq!(logits.dims(), &[1, 1, cfg.vocab_size]);
    }

    // Touch unused HashMap import to silence dead-code warnings on builds
    // that strip dev-deps differently — keep tests dep-free by using
    // VarBuilder::zeros above.
    #[allow(dead_code)]
    fn _hashmap_marker() -> HashMap<String, ()> {
        HashMap::new()
    }
}
