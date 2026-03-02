//! Burn-native modules for LoRA fine-tuning.
//!
//! These are actual `burn::module::Module` implementations that run on GPU via WGPU.

use burn::prelude::*;
use burn::module::{Module, Param};
use burn::nn;
use burn::tensor::activation;

/// LoRA adapter module in Burn.
///
/// Wraps a frozen base linear layer with trainable low-rank A/B matrices.
/// Forward: y = W_frozen @ x + (B @ A @ x) * scaling
#[derive(Module, Debug)]
pub struct LoraLinear<B: Backend> {
    /// Frozen base weight (not updated during training).
    base: nn::Linear<B>,
    /// Down-projection: (rank × in_features).
    lora_a: nn::Linear<B>,
    /// Up-projection: (out_features × rank).
    lora_b: nn::Linear<B>,
    /// Scaling factor: alpha / rank.
    #[module(skip)]
    scaling: f32,
    /// Whether the LoRA adapter is active.
    #[module(skip)]
    active: bool,
}

/// Configuration for creating a LoRA linear layer.
#[derive(Config, Debug)]
pub struct LoraLinearConfig {
    /// Input dimension.
    pub in_features: usize,
    /// Output dimension.
    pub out_features: usize,
    /// LoRA rank (bottleneck dimension).
    #[config(default = "16")]
    pub rank: usize,
    /// Alpha scaling factor.
    #[config(default = "32.0")]
    pub alpha: f32,
}

impl LoraLinearConfig {
    /// Initialize LoRA linear layer.
    ///
    /// Base weights are initialized from a normal distribution (would be loaded from model).
    /// LoRA A is initialized with Kaiming uniform, B is initialized to zero
    /// (so initial LoRA contribution is zero).
    pub fn init<B: Backend>(&self, device: &B::Device) -> LoraLinear<B> {
        let base = nn::LinearConfig::new(self.in_features, self.out_features)
            .with_bias(false)
            .init(device);

        // A: (in_features → rank) — Kaiming init
        let lora_a = nn::LinearConfig::new(self.in_features, self.rank)
            .with_bias(false)
            .init(device);

        // B: (rank → out_features) — zero init so LoRA starts as identity
        let lora_b_config = nn::LinearConfig::new(self.rank, self.out_features)
            .with_bias(false);
        let mut lora_b = lora_b_config.init(device);
        // Zero-initialize B so the LoRA contribution starts at zero
        lora_b.weight = lora_b.weight.map(|w| w.zeros_like());

        LoraLinear {
            base,
            lora_a,
            lora_b,
            scaling: self.alpha / self.rank as f32,
            active: true,
        }
    }
}

impl<B: Backend> LoraLinear<B> {
    /// Forward pass: base + LoRA adapter.
    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        let base_out = self.base.forward(input.clone());

        if !self.active {
            return base_out;
        }

        // LoRA path: input → A → B, scaled
        let lora_out = self.lora_b.forward(self.lora_a.forward(input));
        let lora_scaled = lora_out.mul_scalar(self.scaling);

        base_out + lora_scaled
    }

    /// Freeze the base layer (already frozen by design, but explicit).
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Number of trainable parameters (A + B only).
    pub fn trainable_param_count(&self) -> usize {
        let a_shape = self.lora_a.weight.val().shape();
        let a_params = a_shape.dims[0] * a_shape.dims[1];
        let b_shape = self.lora_b.weight.val().shape();
        let b_params = b_shape.dims[0] * b_shape.dims[1];
        a_params + b_params
    }
}

/// RMS Layer Normalization (used in LLaMA-style models).
#[derive(Module, Debug)]
pub struct RmsNorm<B: Backend> {
    /// Learnable scale parameter.
    weight: Param<Tensor<B, 1>>,
    /// Epsilon for numerical stability.
    #[module(skip)]
    eps: f64,
}

/// Configuration for RMS normalization.
#[derive(Config, Debug)]
pub struct RmsNormConfig {
    pub hidden_size: usize,
    #[config(default = "1e-5")]
    pub eps: f64,
}

impl RmsNormConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> RmsNorm<B> {
        let weight = Tensor::ones([self.hidden_size], device);
        RmsNorm {
            weight: Param::from_tensor(weight),
            eps: self.eps,
        }
    }
}

impl<B: Backend> RmsNorm<B> {
    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let variance = x.clone().powf_scalar(2.0).mean_dim(1);
        let rms = (variance + self.eps).sqrt();
        let normed = x / rms;
        normed * self.weight.val().unsqueeze()
    }
}

/// SwiGLU feed-forward network (LLaMA-style).
#[derive(Module, Debug)]
pub struct SwiGluFfn<B: Backend> {
    gate_proj: nn::Linear<B>,
    up_proj: nn::Linear<B>,
    down_proj: nn::Linear<B>,
}

/// Configuration for SwiGLU FFN.
#[derive(Config, Debug)]
pub struct SwiGluFfnConfig {
    pub hidden_size: usize,
    pub intermediate_size: usize,
}

impl SwiGluFfnConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> SwiGluFfn<B> {
        SwiGluFfn {
            gate_proj: nn::LinearConfig::new(self.hidden_size, self.intermediate_size)
                .with_bias(false)
                .init(device),
            up_proj: nn::LinearConfig::new(self.hidden_size, self.intermediate_size)
                .with_bias(false)
                .init(device),
            down_proj: nn::LinearConfig::new(self.intermediate_size, self.hidden_size)
                .with_bias(false)
                .init(device),
        }
    }
}

impl<B: Backend> SwiGluFfn<B> {
    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        let gate = activation::silu(self.gate_proj.forward(x.clone()));
        let up = self.up_proj.forward(x);
        self.down_proj.forward(gate * up)
    }
}

/// Simple cross-entropy loss for language modeling.
pub fn cross_entropy_loss<B: Backend>(
    logits: Tensor<B, 2>, // [batch * seq_len, vocab_size]
    targets: Tensor<B, 1, Int>, // [batch * seq_len]
) -> Tensor<B, 1> {
    let log_softmax = activation::log_softmax(logits, 1);
    let batch_size = targets.dims()[0];

    // Gather the log probabilities at target indices
    let targets_2d = targets.reshape([batch_size, 1]);
    let gathered = log_softmax.gather(1, targets_2d);

    // Negative log likelihood
    gathered.neg().mean()
}

/// Training step output.
#[derive(Debug)]
pub struct TrainStepOutput<B: Backend> {
    pub loss: Tensor<B, 1>,
    pub num_tokens: usize,
}

// ────────────────────────────────────────────────────────────────────────────
// Phase 5: DoRA, DPO/ORPO tensor losses, Transformer block
// ────────────────────────────────────────────────────────────────────────────

/// DoRA (Weight-Decomposed Low-Rank Adaptation) module in Burn.
///
/// Decomposes weight update into direction and magnitude:
///   W' = m * (W₀ + B·A) / ‖W₀ + B·A‖_col
///
/// Where m is a learnable per-output-neuron magnitude vector.
#[derive(Module, Debug)]
pub struct DoraLinear<B: Backend> {
    /// Frozen base weight.
    base: nn::Linear<B>,
    /// Down-projection: (in_features → rank).
    lora_a: nn::Linear<B>,
    /// Up-projection: (rank → out_features).
    lora_b: nn::Linear<B>,
    /// Learnable magnitude vector (one scalar per output neuron).
    magnitude: Param<Tensor<B, 1>>,
    /// Scaling factor: alpha / rank.
    #[module(skip)]
    scaling: f32,
}

/// Configuration for DoRA linear layer.
#[derive(Config, Debug)]
pub struct DoraLinearConfig {
    pub in_features: usize,
    pub out_features: usize,
    #[config(default = "16")]
    pub rank: usize,
    #[config(default = "32.0")]
    pub alpha: f32,
}

impl DoraLinearConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> DoraLinear<B> {
        let base = nn::LinearConfig::new(self.in_features, self.out_features)
            .with_bias(false)
            .init(device);

        let lora_a = nn::LinearConfig::new(self.in_features, self.rank)
            .with_bias(false)
            .init(device);

        let lora_b_config = nn::LinearConfig::new(self.rank, self.out_features)
            .with_bias(false);
        let mut lora_b = lora_b_config.init(device);
        lora_b.weight = lora_b.weight.map(|w| w.zeros_like());

        // Initialize magnitude from base weight column norms.
        // Burn stores Linear weights as [in_features, out_features], so each column
        // corresponds to one output neuron. Sum across dim 0 (input dim) for per-output norms.
        let base_w = base.weight.val();
        let col_norms = base_w.clone().powf_scalar(2.0).sum_dim(0).sqrt().squeeze(0);
        let magnitude = Param::from_tensor(col_norms);

        DoraLinear {
            base,
            lora_a,
            lora_b,
            magnitude,
            scaling: self.alpha / self.rank as f32,
        }
    }
}

impl<B: Backend> DoraLinear<B> {
    /// Forward pass with direction-magnitude decomposition.
    pub fn forward(&self, input: Tensor<B, 2>) -> Tensor<B, 2> {
        // Burn stores Linear weights as [in_features, out_features].
        // LoRA update: A_w @ B_w in Burn convention (opposite of PyTorch's B @ A).
        let lora_a_w = self.lora_a.weight.val(); // [in_features, rank]
        let lora_b_w = self.lora_b.weight.val(); // [rank, out_features]
        let lora_update = lora_a_w.matmul(lora_b_w).mul_scalar(self.scaling); // [in, out]
        let updated_w = self.base.weight.val() + lora_update; // [in, out]

        // Per-output-neuron norms: sum across dim 0 (input dim) since columns = outputs
        let col_norms = updated_w.clone().powf_scalar(2.0).sum_dim(0).sqrt(); // [1, out]
        let eps: f32 = 1e-8;
        let col_norms_safe = col_norms + eps;

        // Normalize: direction = W' / ‖W'‖
        let direction = updated_w / col_norms_safe; // [in, out]

        // Apply magnitude: W_final = m * direction
        let m = self.magnitude.val().unsqueeze_dim(0); // [1, out]
        let final_w = direction * m; // [in, out]

        // Forward: y = input @ W (Burn convention, no transpose needed)
        input.matmul(final_w)
    }

    /// Total trainable parameters: LoRA A + LoRA B + magnitude vector.
    pub fn trainable_param_count(&self) -> usize {
        let a_shape = self.lora_a.weight.val().shape();
        let b_shape = self.lora_b.weight.val().shape();
        let m_shape = self.magnitude.val().shape();
        a_shape.dims[0] * a_shape.dims[1]
            + b_shape.dims[0] * b_shape.dims[1]
            + m_shape.dims[0]
    }
}

/// DPO loss computed on Burn tensors.
///
/// L_DPO = -log σ(β * (log π(y_w|x)/π_ref(y_w|x) - log π(y_l|x)/π_ref(y_l|x)))
pub fn dpo_loss<B: Backend>(
    chosen_logps: Tensor<B, 1>,      // Log-prob of chosen under policy
    rejected_logps: Tensor<B, 1>,    // Log-prob of rejected under policy
    ref_chosen_logps: Tensor<B, 1>,  // Log-prob of chosen under reference
    ref_rejected_logps: Tensor<B, 1>, // Log-prob of rejected under reference
    beta: f32,
) -> Tensor<B, 1> {
    let chosen_rewards = (chosen_logps - ref_chosen_logps).mul_scalar(beta);
    let rejected_rewards = (rejected_logps - ref_rejected_logps).mul_scalar(beta);
    let logits = chosen_rewards - rejected_rewards;

    // -log σ(logits) = log(1 + exp(-logits)) = softplus(-logits)
    let neg_logits = logits.neg();
    // softplus: log(1 + exp(x)) — numerically stable
    let loss = (neg_logits.clone().exp() + 1.0).log();
    loss.mean()
}

/// ORPO alignment loss computed on Burn tensors.
///
/// L_OR = -log σ(log(odds(chosen) / odds(rejected)))
/// where odds(p) = p / (1-p)
pub fn orpo_alignment_loss<B: Backend>(
    chosen_probs: Tensor<B, 1>,   // Average token probability for chosen
    rejected_probs: Tensor<B, 1>, // Average token probability for rejected
) -> Tensor<B, 1> {
    let eps: f32 = 1e-10;
    let one_minus_eps: f32 = 1.0 - eps;

    // Clamp probabilities to avoid log(0)
    let chosen_clamped = chosen_probs.clamp(eps, one_minus_eps);
    let rejected_clamped = rejected_probs.clamp(eps, one_minus_eps);

    // odds = p / (1-p)
    let chosen_odds = chosen_clamped.clone() / (chosen_clamped.neg() + 1.0);
    let rejected_odds = rejected_clamped.clone() / (rejected_clamped.neg() + 1.0);

    // log odds ratio
    let log_odds_ratio = (chosen_odds / rejected_odds).log();

    // -log σ(log_odds_ratio) = softplus(-log_odds_ratio)
    let neg_lor = log_odds_ratio.neg();
    let loss = (neg_lor.exp() + 1.0).log();
    loss.mean()
}

/// Full ORPO loss: SFT + lambda * alignment.
pub fn orpo_loss<B: Backend>(
    sft_loss: Tensor<B, 1>,
    chosen_probs: Tensor<B, 1>,
    rejected_probs: Tensor<B, 1>,
    lambda: f32,
) -> Tensor<B, 1> {
    let align = orpo_alignment_loss(chosen_probs, rejected_probs);
    sft_loss + align.mul_scalar(lambda)
}

/// Minimal transformer block as a Burn Module.
///
/// Components: RMSNorm → Attention (simplified) → Residual → RMSNorm → SwiGLU FFN → Residual
#[derive(Module, Debug)]
pub struct BurnTransformerBlock<B: Backend> {
    pre_norm: RmsNorm<B>,
    /// Simplified multi-head attention (Q/K/V projections + output).
    q_proj: nn::Linear<B>,
    k_proj: nn::Linear<B>,
    v_proj: nn::Linear<B>,
    o_proj: nn::Linear<B>,
    post_norm: RmsNorm<B>,
    ffn: SwiGluFfn<B>,
    #[module(skip)]
    num_heads: usize,
    #[module(skip)]
    head_dim: usize,
}

/// Configuration for a transformer block.
#[derive(Config, Debug)]
pub struct BurnTransformerBlockConfig {
    pub hidden_size: usize,
    pub num_heads: usize,
    pub intermediate_size: usize,
}

impl BurnTransformerBlockConfig {
    pub fn init<B: Backend>(&self, device: &B::Device) -> BurnTransformerBlock<B> {
        let head_dim = self.hidden_size / self.num_heads;

        BurnTransformerBlock {
            pre_norm: RmsNormConfig::new(self.hidden_size).init(device),
            q_proj: nn::LinearConfig::new(self.hidden_size, self.hidden_size)
                .with_bias(false)
                .init(device),
            k_proj: nn::LinearConfig::new(self.hidden_size, self.hidden_size)
                .with_bias(false)
                .init(device),
            v_proj: nn::LinearConfig::new(self.hidden_size, self.hidden_size)
                .with_bias(false)
                .init(device),
            o_proj: nn::LinearConfig::new(self.hidden_size, self.hidden_size)
                .with_bias(false)
                .init(device),
            post_norm: RmsNormConfig::new(self.hidden_size).init(device),
            ffn: SwiGluFfnConfig::new(self.hidden_size, self.intermediate_size).init(device),
            num_heads: self.num_heads,
            head_dim,
        }
    }
}

impl<B: Backend> BurnTransformerBlock<B> {
    /// Forward pass through the transformer block.
    ///
    /// Input: [batch_size, hidden_size] (single position, no sequence dim for simplicity).
    pub fn forward(&self, x: Tensor<B, 2>) -> Tensor<B, 2> {
        // Pre-norm + attention + residual
        let normed = self.pre_norm.forward(x.clone());
        let q = self.q_proj.forward(normed.clone());
        let k = self.k_proj.forward(normed.clone());
        let v = self.v_proj.forward(normed);

        // Simplified attention: softmax(Q·K^T / sqrt(d)) · V
        let scale = (self.head_dim as f32).sqrt();
        let attn_weights = q.matmul(k.transpose()).div_scalar(scale);
        let attn_weights = activation::softmax(attn_weights, 1);
        let attn_out = attn_weights.matmul(v);
        let attn_proj = self.o_proj.forward(attn_out);

        let h = x + attn_proj; // residual

        // Post-norm + FFN + residual
        let normed2 = self.post_norm.forward(h.clone());
        let ffn_out = self.ffn.forward(normed2);

        h + ffn_out // residual
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;

    type TestBackend = NdArray;

    #[test]
    fn test_lora_linear_forward() {
        let device = Default::default();
        let config = LoraLinearConfig::new(64, 128);
        let layer = config.init::<TestBackend>(&device);

        let input = Tensor::<TestBackend, 2>::random(
            [4, 64],
            burn::tensor::Distribution::Normal(0.0, 1.0),
            &device,
        );
        let output = layer.forward(input);

        assert_eq!(output.dims(), [4, 128]);
    }

    #[test]
    fn test_lora_linear_zero_init() {
        let device = Default::default();
        let config = LoraLinearConfig::new(32, 32);
        let layer = config.init::<TestBackend>(&device);

        // With B zero-initialized, LoRA contribution should be zero
        // so output should equal base output
        let input = Tensor::<TestBackend, 2>::random(
            [2, 32],
            burn::tensor::Distribution::Normal(0.0, 1.0),
            &device,
        );

        let base_out = layer.base.forward(input.clone());
        let full_out = layer.forward(input);

        let diff = (full_out - base_out).abs().sum().into_scalar();
        assert!(diff < 1e-5, "LoRA should contribute zero initially, diff={}", diff);
    }

    #[test]
    fn test_lora_inactive() {
        let device = Default::default();
        let config = LoraLinearConfig::new(32, 32);
        let mut layer = config.init::<TestBackend>(&device);
        layer.set_active(false);

        let input = Tensor::<TestBackend, 2>::random(
            [2, 32],
            burn::tensor::Distribution::Normal(0.0, 1.0),
            &device,
        );
        let base_out = layer.base.forward(input.clone());
        let full_out = layer.forward(input);

        let diff = (full_out - base_out).abs().sum().into_scalar();
        assert!(diff < 1e-6, "Inactive LoRA should not contribute");
    }

    #[test]
    fn test_rms_norm() {
        let device = Default::default();
        let norm = RmsNormConfig::new(64).init::<TestBackend>(&device);

        let input = Tensor::<TestBackend, 2>::random(
            [4, 64],
            burn::tensor::Distribution::Normal(0.0, 1.0),
            &device,
        );
        let output = norm.forward(input);

        assert_eq!(output.dims(), [4, 64]);
    }

    #[test]
    fn test_swiglu_ffn() {
        let device = Default::default();
        let ffn = SwiGluFfnConfig::new(64, 128).init::<TestBackend>(&device);

        let input = Tensor::<TestBackend, 2>::random(
            [4, 64],
            burn::tensor::Distribution::Normal(0.0, 1.0),
            &device,
        );
        let output = ffn.forward(input);

        assert_eq!(output.dims(), [4, 64]);
    }

    #[test]
    fn test_trainable_params() {
        let device = Default::default();
        let config = LoraLinearConfig::new(4096, 4096).with_rank(16);
        let layer = config.init::<TestBackend>(&device);

        let params = layer.trainable_param_count();
        assert_eq!(params, 16 * 4096 + 4096 * 16); // A + B
    }

    // ── Phase 5 tests ──

    #[test]
    fn test_dora_forward() {
        let device = Default::default();
        let config = DoraLinearConfig::new(64, 128).with_rank(8);
        let layer = config.init::<TestBackend>(&device);

        let input = Tensor::<TestBackend, 2>::random(
            [4, 64],
            burn::tensor::Distribution::Normal(0.0, 1.0),
            &device,
        );
        let output = layer.forward(input);
        assert_eq!(output.dims(), [4, 128]);
    }

    #[test]
    fn test_dora_trainable_params() {
        let device = Default::default();
        let config = DoraLinearConfig::new(256, 256).with_rank(16);
        let layer = config.init::<TestBackend>(&device);

        let params = layer.trainable_param_count();
        // A: 16*256 + B: 256*16 + magnitude: 256
        assert_eq!(params, 16 * 256 + 256 * 16 + 256);
    }

    #[test]
    fn test_dpo_loss_tensor() {
        let device = Default::default();
        let chosen = Tensor::<TestBackend, 1>::from_floats([-1.0, -0.5, -0.8], &device);
        let rejected = Tensor::<TestBackend, 1>::from_floats([-3.0, -2.5, -2.8], &device);
        let ref_chosen = Tensor::<TestBackend, 1>::from_floats([-1.5, -1.0, -1.2], &device);
        let ref_rejected = Tensor::<TestBackend, 1>::from_floats([-1.5, -1.0, -1.2], &device);

        let loss = dpo_loss(chosen, rejected, ref_chosen, ref_rejected, 0.1);
        let val: f32 = loss.into_scalar();
        assert!(val > 0.0, "DPO loss should be positive, got {}", val);
        assert!(val < 5.0, "DPO loss should be reasonable, got {}", val);
    }

    #[test]
    fn test_dpo_loss_equal_logps() {
        let device = Default::default();
        // When chosen and rejected are equal, loss should be log(2)
        let logps = Tensor::<TestBackend, 1>::from_floats([-2.0], &device);
        let loss = dpo_loss(logps.clone(), logps.clone(), logps.clone(), logps, 0.1);
        let val: f32 = loss.into_scalar();
        assert!((val - (2.0f32).ln()).abs() < 0.01, "Expected ~ln(2), got {}", val);
    }

    #[test]
    fn test_orpo_alignment_loss() {
        let device = Default::default();
        let chosen = Tensor::<TestBackend, 1>::from_floats([0.8, 0.7], &device);
        let rejected = Tensor::<TestBackend, 1>::from_floats([0.3, 0.2], &device);

        let loss = orpo_alignment_loss(chosen, rejected);
        let val: f32 = loss.into_scalar();
        assert!(val > 0.0, "ORPO alignment loss should be positive");
    }

    #[test]
    fn test_orpo_full_loss() {
        let device = Default::default();
        let sft = Tensor::<TestBackend, 1>::from_floats([2.0], &device);
        let chosen = Tensor::<TestBackend, 1>::from_floats([0.7], &device);
        let rejected = Tensor::<TestBackend, 1>::from_floats([0.3], &device);

        let total = orpo_loss(sft, chosen, rejected, 0.5);
        let val: f32 = total.into_scalar();
        assert!(val > 2.0, "Total should be > SFT loss, got {}", val);
    }

    #[test]
    fn test_transformer_block() {
        let device = Default::default();
        let config = BurnTransformerBlockConfig::new(64, 4, 128);
        let block = config.init::<TestBackend>(&device);

        let input = Tensor::<TestBackend, 2>::random(
            [8, 64],
            burn::tensor::Distribution::Normal(0.0, 0.1),
            &device,
        );
        let output = block.forward(input);
        assert_eq!(output.dims(), [8, 64], "Transformer block should preserve shape");
    }
}
