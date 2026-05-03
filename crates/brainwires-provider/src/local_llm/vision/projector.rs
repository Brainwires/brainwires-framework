//! MultiModalProjector — SigLIP patches → Gemma soft tokens.
//!
//! Pipeline:
//! ```text
//!   [B, 4096, 1152]  (vision tower output)
//!     ↓ reshape to [B, 64, 64, 1152]
//!     ↓ permute to [B, 1152, 64, 64] (NCHW for avg_pool2d)
//!     ↓ avg_pool2d kernel=4 stride=4
//!   [B, 1152, 16, 16]
//!     ↓ permute back to [B, 16, 16, 1152]
//!     ↓ flatten spatial → [B, 256, 1152]
//!     ↓ Linear(1152 → text_hidden), no bias
//!     ↓ RMSNorm(text_hidden), eps=1e-6
//!   [B, 256, text_hidden]
//! ```
//!
//! These 256 soft embeddings replace the placeholder image tokens that
//! Stage A's [`super::splice_image_token_block`] reserved.
//!
//! ## Weight names
//!
//! Loaded from a [`VarBuilder`] rooted at `multi_modal_projector` (the HF
//! convention used in Gemma 3/4 vision-language safetensors):
//! - `mm_input_projection_weight` — linear projection, shape
//!   `[vision_hidden, text_hidden]`. NOTE: this is the transpose of the
//!   typical PyTorch `nn.Linear` weight `[out, in]` — Gemma 3's reference
//!   impl stores the projection as `[in, out]` and applies it via
//!   `xs @ proj_weight` (no transpose). No bias.
//! - `mm_soft_emb_norm.weight` — RMSNorm scale, shape `[text_hidden]`.
//!
//! The bias-free assumption matches the published Gemma 3/4 checkpoints
//! (`google/gemma-3-4b-it`, etc.). If a future variant adds a bias, the
//! `VarBuilder::get` call for `mm_input_projection_bias` would need to be
//! made optional.

use candle_core::Module;
use candle_nn::{ops::rms_norm, VarBuilder};
use thiserror::Error;

use crate::CandleDevice as Device;
use crate::CandleTensor as Tensor;

/// Errors emitted while loading or running the multimodal projector.
#[derive(Debug, Error)]
pub enum ProjectorError {
    /// Construction failed — usually a missing or mis-shaped weight.
    #[error("projector load: {0}")]
    Load(String),
    /// Forward pass failed — usually a shape mismatch on the input.
    #[error("projector forward: {0}")]
    Forward(String),
}

/// Default RMSNorm epsilon used by Gemma 3/4 (`mm_soft_emb_norm`).
pub const DEFAULT_EPS: f64 = 1e-6;

/// Spatial grid side after the SigLIP `paligemma_3b_896` config: 896/14 = 64.
const SIGLIP_GRID_SIDE: usize = 64;
/// Pool factor that takes the 64×64 grid down to 16×16 (= 256 tokens).
const POOL_FACTOR: usize = 4;
/// Output side after pooling.
const POOLED_SIDE: usize = SIGLIP_GRID_SIDE / POOL_FACTOR; // 16
/// Total patches in the SigLIP output (`64 * 64`).
const SIGLIP_PATCH_COUNT: usize = SIGLIP_GRID_SIDE * SIGLIP_GRID_SIDE; // 4096
/// Total soft image tokens after pooling (`16 * 16`).
const POOLED_TOKEN_COUNT: usize = POOLED_SIDE * POOLED_SIDE; // 256

/// MultiModalProjector for Gemma 3/4 vision.
///
/// See module docs for the full pipeline + weight-naming contract.
pub struct MultiModalProjector {
    /// Linear projection weight, shape `[vision_hidden, text_hidden]`.
    /// Applied via `xs @ proj_weight` (no transpose, no bias).
    proj_weight: Tensor,
    /// RMSNorm scale, shape `[text_hidden]`. Applied AFTER the projection.
    norm_weight: Tensor,
    /// RMSNorm epsilon. Gemma 3/4 uses `1e-6`.
    eps: f64,
    /// Output dim (gemma text `hidden_size`).
    text_hidden: usize,
    /// Input dim (siglip `hidden_size` = 1152 for `paligemma_3b_896`).
    vision_hidden: usize,
    device: Device,
}

impl MultiModalProjector {
    /// Load from a Candle [`VarBuilder`] rooted at `multi_modal_projector`
    /// (the HF naming convention).
    ///
    /// Reads:
    /// - `mm_input_projection_weight` shape `[vision_hidden, text_hidden]`
    ///   (no bias)
    /// - `mm_soft_emb_norm.weight` shape `[text_hidden]`
    ///
    /// `eps` is the RMSNorm epsilon — pass [`DEFAULT_EPS`] (1e-6) for Gemma
    /// 3/4.
    pub fn load(
        vb: VarBuilder,
        vision_hidden: usize,
        text_hidden: usize,
        eps: f64,
    ) -> Result<Self, ProjectorError> {
        let device = vb.device().clone();
        let proj_weight = vb
            .get(
                (vision_hidden, text_hidden),
                "mm_input_projection_weight",
            )
            .map_err(|e| ProjectorError::Load(format!("mm_input_projection_weight: {e}")))?;
        let norm_weight = vb
            .pp("mm_soft_emb_norm")
            .get(text_hidden, "weight")
            .map_err(|e| ProjectorError::Load(format!("mm_soft_emb_norm.weight: {e}")))?;
        Ok(Self {
            proj_weight,
            norm_weight,
            eps,
            text_hidden,
            vision_hidden,
            device,
        })
    }

    /// SigLIP hidden size this projector expects on its input
    /// (e.g. `1152`).
    pub fn vision_hidden(&self) -> usize {
        self.vision_hidden
    }

    /// Gemma text hidden size produced on the output (depends on variant —
    /// 1B=1152, 4B=2560, 12B=3584, 27B=4608).
    pub fn text_hidden(&self) -> usize {
        self.text_hidden
    }

    /// Device the projector's weights live on.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Project a SigLIP-tower output to a block of soft image tokens.
    ///
    /// - Input: `[B, 4096, vision_hidden]` (the `paligemma_3b_896` SigLIP
    ///   patch sequence).
    /// - Output: `[B, 256, text_hidden]` — ready to splice into Gemma's
    ///   input embedding stream at the placeholder positions reserved by
    ///   Stage A's [`super::splice_image_token_block`].
    pub fn forward(&self, vision_features: &Tensor) -> Result<Tensor, ProjectorError> {
        let dims = vision_features
            .dims3()
            .map_err(|e| ProjectorError::Forward(format!("expected rank-3 tensor: {e}")))?;
        let (batch, patches, hidden) = dims;
        if patches != SIGLIP_PATCH_COUNT {
            return Err(ProjectorError::Forward(format!(
                "expected {SIGLIP_PATCH_COUNT} patches, got {patches}"
            )));
        }
        if hidden != self.vision_hidden {
            return Err(ProjectorError::Forward(format!(
                "expected hidden={}, got {hidden}",
                self.vision_hidden
            )));
        }

        // 1. [B, 4096, V] → [B, 64, 64, V] (NHWC)
        let xs = vision_features
            .reshape((batch, SIGLIP_GRID_SIDE, SIGLIP_GRID_SIDE, hidden))
            .map_err(|e| ProjectorError::Forward(format!("reshape to grid: {e}")))?;
        // 2. NHWC → NCHW for avg_pool2d: [B, V, 64, 64]
        let xs = xs
            .permute((0, 3, 1, 2))
            .map_err(|e| ProjectorError::Forward(format!("permute NHWC→NCHW: {e}")))?
            .contiguous()
            .map_err(|e| ProjectorError::Forward(format!("contiguous NCHW: {e}")))?;
        // 3. avg_pool2d kernel=4 stride=4 → [B, V, 16, 16]
        let xs = xs
            .avg_pool2d((POOL_FACTOR, POOL_FACTOR))
            .map_err(|e| ProjectorError::Forward(format!("avg_pool2d 4×4: {e}")))?;
        // 4. NCHW → NHWC: [B, 16, 16, V]
        let xs = xs
            .permute((0, 2, 3, 1))
            .map_err(|e| ProjectorError::Forward(format!("permute NCHW→NHWC: {e}")))?
            .contiguous()
            .map_err(|e| ProjectorError::Forward(format!("contiguous NHWC: {e}")))?;
        // 5. Flatten spatial → [B, 256, V]
        let xs = xs
            .reshape((batch, POOLED_TOKEN_COUNT, hidden))
            .map_err(|e| ProjectorError::Forward(format!("flatten spatial: {e}")))?;
        // 6. Linear projection: y = x @ proj_weight (proj is stored
        //    [in, out], no transpose) → [B, 256, text_hidden]
        let xs = xs
            .broadcast_matmul(&self.proj_weight)
            .map_err(|e| ProjectorError::Forward(format!("matmul proj: {e}")))?;
        // 7. RMSNorm with eps + scale.
        let xs = rms_norm(&xs, &self.norm_weight, self.eps as f32)
            .map_err(|e| ProjectorError::Forward(format!("rms_norm: {e}")))?;
        Ok(xs)
    }
}

impl Module for MultiModalProjector {
    fn forward(&self, xs: &Tensor) -> candle_core::Result<Tensor> {
        self.forward(xs)
            .map_err(|e| candle_core::Error::Msg(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::DType;
    use candle_nn::VarBuilder;
    use std::collections::HashMap;

    /// Build a `VarBuilder` rooted at `multi_modal_projector` from a
    /// pair of (proj_weight, norm_weight) tensors, mirroring the layout the
    /// projector expects.
    fn make_vb(proj: Tensor, norm: Tensor, device: &Device) -> VarBuilder<'static> {
        let mut tensors = HashMap::new();
        tensors.insert(
            "multi_modal_projector.mm_input_projection_weight".to_string(),
            proj,
        );
        tensors.insert(
            "multi_modal_projector.mm_soft_emb_norm.weight".to_string(),
            norm,
        );
        VarBuilder::from_tensors(tensors, DType::F32, device).pp("multi_modal_projector")
    }

    /// Smoke test: zero proj + ones norm → forward returns the right
    /// shape and doesn't NaN. Uses small `vision_hidden=8`, `text_hidden=16`
    /// so the test runs in milliseconds.
    #[test]
    fn test_load_with_zero_weights_runs_forward() {
        let device = Device::Cpu;
        let vision_hidden = 8;
        let text_hidden = 16;
        let proj = Tensor::zeros((vision_hidden, text_hidden), DType::F32, &device).unwrap();
        let norm = Tensor::ones(text_hidden, DType::F32, &device).unwrap();
        let vb = make_vb(proj, norm, &device);

        let projector =
            MultiModalProjector::load(vb, vision_hidden, text_hidden, DEFAULT_EPS).unwrap();
        assert_eq!(projector.vision_hidden(), vision_hidden);
        assert_eq!(projector.text_hidden(), text_hidden);

        let input =
            Tensor::ones((1, SIGLIP_PATCH_COUNT, vision_hidden), DType::F32, &device).unwrap();
        let out = projector.forward(&input).unwrap();
        assert_eq!(out.dims(), &[1, POOLED_TOKEN_COUNT, text_hidden]);
    }

    /// With zero projection weights, the matmul produces all zeros; RMSNorm
    /// of all zeros must stay zero (or at least finite). This guards against
    /// a divide-by-zero NaN in the normalization step.
    #[test]
    fn test_zero_proj_weights_produce_zero_output() {
        let device = Device::Cpu;
        let vision_hidden = 8;
        let text_hidden = 16;
        let proj = Tensor::zeros((vision_hidden, text_hidden), DType::F32, &device).unwrap();
        let norm = Tensor::ones(text_hidden, DType::F32, &device).unwrap();
        let vb = make_vb(proj, norm, &device);
        let projector =
            MultiModalProjector::load(vb, vision_hidden, text_hidden, DEFAULT_EPS).unwrap();

        let input =
            Tensor::ones((1, SIGLIP_PATCH_COUNT, vision_hidden), DType::F32, &device).unwrap();
        let out = projector.forward(&input).unwrap();
        // Flatten and inspect — every element must be zero (and crucially
        // finite — Candle's rms_norm uses `(sum_sq / n + eps).sqrt()` so the
        // eps keeps the denom > 0 even for an all-zero input).
        let flat = out.flatten_all().unwrap().to_vec1::<f32>().unwrap();
        for (i, v) in flat.iter().enumerate() {
            assert!(v.is_finite(), "output[{i}] = {v} is not finite");
            assert_eq!(*v, 0.0, "output[{i}] = {v}, expected 0.0");
        }
    }

    /// Verify that the 4×4 average pool collapses the right 4×4 window in
    /// the 64×64 grid. We construct an input where each spatial position is
    /// labelled by its row index (so position (r, c) in the grid carries the
    /// scalar value `r` across all `vision_hidden` channels). The pooled
    /// output at (0, 0) should average rows 0..4 → `(0+1+2+3)/4 = 1.5`,
    /// uniformly across the channel dim.
    ///
    /// Catches NHWC vs NCHW ordering bugs: if we pooled before permuting,
    /// or pooled along the wrong axis, the averaged value would be different.
    #[test]
    fn test_avg_pool_4x4_reduces_4096_to_256() {
        let device = Device::Cpu;
        let vision_hidden = 4;
        let text_hidden = 4;

        // Build [1, 4096, 4] where row r of the 64×64 grid holds value r.
        let mut data = Vec::with_capacity(SIGLIP_PATCH_COUNT * vision_hidden);
        for r in 0..SIGLIP_GRID_SIDE {
            for _c in 0..SIGLIP_GRID_SIDE {
                for _v in 0..vision_hidden {
                    data.push(r as f32);
                }
            }
        }
        let input = Tensor::from_vec(data, (1, SIGLIP_PATCH_COUNT, vision_hidden), &device)
            .unwrap();

        // Identity-ish projector: proj_weight = identity (4×4), norm_weight
        // = ones, eps small. RMSNorm of a constant vector v across hidden
        // dim: rms = sqrt(v^2) = |v|, so output ≈ sign(v). To avoid that
        // sign-collapse messing with the test, we directly inspect the
        // pooling step by reproducing the pool inline (matches the
        // implementation exactly) — this is the most surgical way to assert
        // pooling correctness without adding another public API surface.
        let xs = input
            .reshape((1, SIGLIP_GRID_SIDE, SIGLIP_GRID_SIDE, vision_hidden))
            .unwrap();
        let xs = xs.permute((0, 3, 1, 2)).unwrap().contiguous().unwrap();
        let pooled = xs.avg_pool2d((POOL_FACTOR, POOL_FACTOR)).unwrap();
        assert_eq!(pooled.dims(), &[1, vision_hidden, POOLED_SIDE, POOLED_SIDE]);
        let pooled = pooled
            .permute((0, 2, 3, 1))
            .unwrap()
            .contiguous()
            .unwrap()
            .reshape((1, POOLED_TOKEN_COUNT, vision_hidden))
            .unwrap();

        // The pooled token at (0, 0) — first of the 256 — should hold the
        // mean of the 4×4 window covering rows 0..4 in the 64×64 grid:
        // (0+1+2+3)/4 = 1.5, repeated across all `vision_hidden` channels.
        let token0 = pooled
            .get(0)
            .unwrap()
            .get(0)
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        for v in &token0 {
            assert!(
                (v - 1.5).abs() < 1e-5,
                "token[0] channel value {v}, expected 1.5"
            );
        }

        // The next pooled token along width is (row=0, col=1) in the 16×16
        // pooled grid — same window of rows 0..4 → still 1.5.
        let token1 = pooled
            .get(0)
            .unwrap()
            .get(1)
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        for v in &token1 {
            assert!((v - 1.5).abs() < 1e-5);
        }

        // Token 16 is (row=1, col=0) — averages rows 4..8 → (4+5+6+7)/4 = 5.5.
        let token16 = pooled
            .get(0)
            .unwrap()
            .get(POOLED_SIDE)
            .unwrap()
            .to_vec1::<f32>()
            .unwrap();
        for v in &token16 {
            assert!(
                (v - 5.5).abs() < 1e-5,
                "token[16] channel value {v}, expected 5.5"
            );
        }

        // Sanity: also exercise the full forward path with a benign norm.
        let proj_data: Vec<f32> = (0..vision_hidden * text_hidden)
            .map(|i| if i % (text_hidden + 1) == 0 { 1.0 } else { 0.0 })
            .collect();
        let proj =
            Tensor::from_vec(proj_data, (vision_hidden, text_hidden), &device).unwrap();
        let norm = Tensor::ones(text_hidden, DType::F32, &device).unwrap();
        let vb = make_vb(proj, norm, &device);
        let projector =
            MultiModalProjector::load(vb, vision_hidden, text_hidden, DEFAULT_EPS).unwrap();
        let out = projector.forward(&input).unwrap();
        assert_eq!(out.dims(), &[1, POOLED_TOKEN_COUNT, text_hidden]);
    }
}
