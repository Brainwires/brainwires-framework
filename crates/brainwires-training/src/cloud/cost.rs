use crate::config::TrainingHyperparams;

/// Per-provider cost estimation for fine-tuning jobs.
pub struct CostEstimator;

/// Cost breakdown for a fine-tuning job.
#[derive(Debug, Clone)]
pub struct CostEstimate {
    /// Provider name.
    pub provider: String,
    /// Estimated cost in USD.
    pub estimated_cost_usd: f64,
    /// Cost per 1M training tokens.
    pub cost_per_million_tokens: f64,
    /// Total estimated tokens.
    pub total_tokens: u64,
    /// Number of epochs.
    pub epochs: u32,
}

impl CostEstimator {
    /// Estimate cost for OpenAI fine-tuning.
    pub fn openai(
        model: &str,
        total_tokens: u64,
        hyperparams: &TrainingHyperparams,
    ) -> CostEstimate {
        // OpenAI pricing (per 1M training tokens, as of 2025)
        let cost_per_million = match model {
            m if m.contains("gpt-4o-mini") => 3.00,
            m if m.contains("gpt-4o") => 25.00,
            m if m.contains("gpt-4") => 30.00,
            m if m.contains("gpt-3.5") => 8.00,
            _ => 10.00, // default estimate
        };

        let total = total_tokens as f64 * hyperparams.epochs as f64;
        let cost = total / 1_000_000.0 * cost_per_million;

        CostEstimate {
            provider: "openai".to_string(),
            estimated_cost_usd: cost,
            cost_per_million_tokens: cost_per_million,
            total_tokens,
            epochs: hyperparams.epochs,
        }
    }

    /// Estimate cost for Together AI fine-tuning.
    pub fn together(
        model: &str,
        total_tokens: u64,
        hyperparams: &TrainingHyperparams,
    ) -> CostEstimate {
        // Together AI pricing (per 1M tokens, approximate)
        let cost_per_million = match model {
            m if m.contains("8B") || m.contains("8b") => 0.50,
            m if m.contains("70B") || m.contains("70b") => 3.00,
            m if m.contains("Mixtral") => 2.00,
            _ => 1.00,
        };

        let total = total_tokens as f64 * hyperparams.epochs as f64;
        let cost = total / 1_000_000.0 * cost_per_million;

        CostEstimate {
            provider: "together".to_string(),
            estimated_cost_usd: cost,
            cost_per_million_tokens: cost_per_million,
            total_tokens,
            epochs: hyperparams.epochs,
        }
    }

    /// Estimate cost for Fireworks AI fine-tuning.
    pub fn fireworks(
        _model: &str,
        total_tokens: u64,
        hyperparams: &TrainingHyperparams,
    ) -> CostEstimate {
        // Fireworks pricing is typically per-GPU-hour, rough token-based estimate
        let cost_per_million = 1.00;
        let total = total_tokens as f64 * hyperparams.epochs as f64;
        let cost = total / 1_000_000.0 * cost_per_million;

        CostEstimate {
            provider: "fireworks".to_string(),
            estimated_cost_usd: cost,
            cost_per_million_tokens: cost_per_million,
            total_tokens,
            epochs: hyperparams.epochs,
        }
    }

    /// Estimate cost for Anyscale fine-tuning.
    pub fn anyscale(
        _model: &str,
        total_tokens: u64,
        hyperparams: &TrainingHyperparams,
    ) -> CostEstimate {
        let cost_per_million = 0.80;
        let total = total_tokens as f64 * hyperparams.epochs as f64;
        let cost = total / 1_000_000.0 * cost_per_million;

        CostEstimate {
            provider: "anyscale".to_string(),
            estimated_cost_usd: cost,
            cost_per_million_tokens: cost_per_million,
            total_tokens,
            epochs: hyperparams.epochs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_cost_estimation() {
        let hyperparams = TrainingHyperparams::default(); // 3 epochs
        let estimate = CostEstimator::openai("gpt-4o-mini-2024-07-18", 1_000_000, &hyperparams);

        assert_eq!(estimate.provider, "openai");
        assert!((estimate.cost_per_million_tokens - 3.0).abs() < f64::EPSILON);
        // 1M tokens * 3 epochs * $3/1M = $9
        assert!((estimate.estimated_cost_usd - 9.0).abs() < 0.01);
    }

    #[test]
    fn test_together_cost_estimation() {
        let hyperparams = TrainingHyperparams::default();
        let estimate = CostEstimator::together("meta-llama/Meta-Llama-3.1-8B-Instruct", 500_000, &hyperparams);

        assert_eq!(estimate.provider, "together");
        // 500K tokens * 3 epochs * $0.50/1M = $0.75
        assert!((estimate.estimated_cost_usd - 0.75).abs() < 0.01);
    }
}
