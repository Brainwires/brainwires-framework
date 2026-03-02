/// DoRA (Weight-Decomposed Low-Rank Adaptation) layer.
///
/// Decomposes the weight matrix into direction and magnitude:
///   W = m * (W₀ + BA) / ‖W₀ + BA‖
///
/// Where:
/// - m is a learnable magnitude vector (per output neuron)
/// - W₀ is the frozen base weight
/// - BA is the standard LoRA update
///
/// DoRA consistently outperforms LoRA by learning magnitude separately
/// from direction, closer to how full fine-tuning works.
#[derive(Debug, Clone)]
pub struct DoraLayer {
    /// Input dimension.
    pub in_features: usize,
    /// Output dimension.
    pub out_features: usize,
    /// LoRA rank for the directional component.
    pub rank: usize,
    /// Alpha scaling factor.
    pub alpha: f32,
    /// Dropout rate.
    pub dropout: f32,
}

impl DoraLayer {
    pub fn new(in_features: usize, out_features: usize, rank: usize, alpha: f32) -> Self {
        Self {
            in_features,
            out_features,
            rank,
            alpha,
            dropout: 0.0,
        }
    }

    /// Scaling factor for the directional LoRA component.
    pub fn scaling(&self) -> f32 {
        self.alpha / self.rank as f32
    }

    /// Trainable parameters: LoRA A + LoRA B + magnitude vector.
    pub fn trainable_params(&self) -> usize {
        let lora_params = self.rank * self.in_features + self.out_features * self.rank;
        let magnitude_params = self.out_features; // one scalar per output neuron
        lora_params + magnitude_params
    }

    /// Frozen base parameter count.
    pub fn frozen_params(&self) -> usize {
        self.in_features * self.out_features
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dora_layer() {
        let layer = DoraLayer::new(4096, 4096, 16, 32.0);
        // LoRA params + magnitude vector
        let expected = 16 * 4096 + 4096 * 16 + 4096;
        assert_eq!(layer.trainable_params(), expected);
        assert_eq!(layer.scaling(), 2.0);
    }
}
