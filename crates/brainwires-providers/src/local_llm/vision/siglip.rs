//! SigLIP vision tower for Gemma 3/4.
//!
//! Wraps [`candle_transformers::models::siglip::VisionModel`] configured with
//! the [`VisionConfig::paligemma_3b_896`] preset (896×896 input → 64×64 = 4096
//! patches × 1152 hidden, 27 layers, patch size 14, SigLIP-So400m). The model
//! itself returns the unpooled patch tensor `[B, 4096, 1152]`; Stage C's
//! `MultiModalProjector` reduces patches to 256 image tokens via a 4×4
//! spatial pool followed by a linear projection into Gemma's text embedding
//! space.
//!
//! Single-crop only. Pan-and-Scan tiling — where the image is split into
//! multiple 896×896 crops, each producing its own block of image tokens — is
//! a follow-up.
//!
//! ## API contract
//!
//! - Input: pixel tensor of shape `[B, 3, 896, 896]`, f32, normalized with
//!   SigLIP mean/std (see [`super::preprocess`]).
//! - Output: patch tensor of shape `[B, 4096, 1152]` (post-layernorm, no
//!   classification head — `use_head = false`).
//!
//! The Candle [`VisionModel`] implements [`Module`], so under the hood we
//! call `pixel_values.apply(&inner)`. We don't use the
//! `MultiheadAttentionPoolingHead` (`use_head = true`) because that head
//! pools patches down to a single feature vector for zero-shot
//! classification — the vision-language path needs the full patch sequence.

use candle_core::Module;
use candle_nn::VarBuilder;
use candle_transformers::models::siglip::{VisionConfig, VisionModel};
use thiserror::Error;

use crate::CandleDevice as Device;
use crate::CandleTensor as Tensor;

/// Errors emitted while building or running the SigLIP vision tower.
#[derive(Debug, Error)]
pub enum SiglipError {
    /// Construction failed — usually a missing or mis-shaped weight in the
    /// `VarBuilder`.
    #[error("siglip load: {0}")]
    Load(String),
    /// Forward pass failed — usually a shape mismatch on `pixel_values`.
    #[error("siglip forward: {0}")]
    Forward(String),
}

/// SigLIP vision tower, single-crop, `paligemma_3b_896` config.
///
/// See module docs for the I/O contract.
pub struct SiglipVisionTower {
    inner: VisionModel,
    config: VisionConfig,
    device: Device,
}

impl SiglipVisionTower {
    /// Build a fresh tower from a Candle [`VarBuilder`]. The `VarBuilder`
    /// must be rooted at the SigLIP weights — typically a sub-prefix in a
    /// Gemma safetensors file like `vision_tower.vision_model` (the inner
    /// `VisionTransformer` looks up `embeddings.*`, `encoder.layers.*`,
    /// `post_layernorm.*` directly under the supplied prefix).
    ///
    /// Uses [`VisionConfig::paligemma_3b_896`] and `use_head = false` (the
    /// classification pooling head would collapse the 4096 patches to a
    /// single vector — we need the full patch sequence for the projector).
    pub fn load(vb: VarBuilder, device: Device) -> Result<Self, SiglipError> {
        let config = VisionConfig::paligemma_3b_896();
        let inner =
            VisionModel::new(&config, false, vb).map_err(|e| SiglipError::Load(e.to_string()))?;
        Ok(Self {
            inner,
            config,
            device,
        })
    }

    /// Reference to the underlying [`VisionConfig`].
    pub fn config(&self) -> &VisionConfig {
        &self.config
    }

    /// Reference to the device this tower's weights live on.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Number of patches the encoder produces per image.
    /// For `paligemma_3b_896` this is `(896 / 14)^2 = 4096`.
    pub fn num_patches(&self) -> usize {
        self.config.num_patches()
    }

    /// Hidden size of each patch embedding.
    /// For `paligemma_3b_896` this is `1152`.
    pub fn hidden_size(&self) -> usize {
        self.config.hidden_size
    }

    /// Encode a preprocessed image tensor of shape `[B, 3, 896, 896]`.
    ///
    /// Returns `[B, num_patches, hidden_size]`. For `paligemma_3b_896` that
    /// is `[B, 4096, 1152]`. The output is the post-layernorm patch
    /// sequence — no classification head, no pooling.
    pub fn forward(&self, pixel_values: &Tensor) -> Result<Tensor, SiglipError> {
        // VisionModel implements Module; the forward routes through the
        // inner VisionTransformer (embeddings → encoder → post_layernorm).
        self.inner
            .forward(pixel_values)
            .map_err(|e| SiglipError::Forward(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `paligemma_3b_896` preset must match the constants Gemma 3/4's
    /// vision tower expects. If upstream Candle ever changes these numbers,
    /// the projector + token-splice math downstream will silently
    /// miscompute — this guard catches that on the next CI run.
    #[test]
    fn test_default_config_dimensions() {
        let cfg = VisionConfig::paligemma_3b_896();

        // SigLIP-So400m: 27 layers × 1152 hidden × 16 heads × 4304 MLP.
        assert_eq!(cfg.hidden_size, 1152);
        assert_eq!(cfg.intermediate_size, 4304);
        assert_eq!(cfg.num_hidden_layers, 27);
        assert_eq!(cfg.num_attention_heads, 16);
        assert_eq!(cfg.num_channels, 3);

        // 896×896 input, 14×14 patches → 64×64 = 4096 patches.
        assert_eq!(cfg.image_size, 896);
        assert_eq!(cfg.patch_size, 14);
        assert_eq!(cfg.num_patches(), 4096);

        // SigLIP convention: GeluPytorchTanh activation, 1e-6 layer-norm eps.
        assert_eq!(cfg.layer_norm_eps, 1e-6);
    }

    // Forward-shape validation (zero-weights smoke test + wrong-shape
    // rejection) is deferred to Stage E's integration test, where real
    // weights are loaded. Building a 400M-parameter SigLIP-So400m via
    // `VarBuilder::zeros` would allocate ~1.6 GB just to validate that
    // tensor arithmetic doesn't panic — not worth it for a unit test.
}
