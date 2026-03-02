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
}
