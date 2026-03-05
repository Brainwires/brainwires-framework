//! Autonomous training loop — orchestrates model training cycles.

use serde::{Deserialize, Serialize};

/// Configuration for the autonomous training loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoopConfig {
    /// Maximum training rounds.
    pub max_rounds: u32,
    /// Maximum cost per round in USD.
    pub max_cost_per_round: f64,
    /// Minimum improvement threshold to continue training.
    pub min_improvement: f64,
    /// Dataset size limit per round.
    pub max_dataset_size: usize,
    /// Whether to auto-evaluate after each round.
    pub auto_evaluate: bool,
}

impl Default for TrainingLoopConfig {
    fn default() -> Self {
        Self {
            max_rounds: 5,
            max_cost_per_round: 10.0,
            min_improvement: 0.01,
            max_dataset_size: 10_000,
            auto_evaluate: true,
        }
    }
}

/// Result of a single training round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRoundResult {
    /// Round number (1-based).
    pub round: u32,
    /// Number of examples in the training dataset.
    pub dataset_size: usize,
    /// Final training loss for this round.
    pub training_loss: f64,
    /// Validation loss, if a validation set was used.
    pub validation_loss: Option<f64>,
    /// Evaluation score from the eval suite, if available.
    pub eval_score: Option<f64>,
    /// Cost of this round in USD.
    pub cost: f64,
    /// Duration of this round in seconds.
    pub duration_secs: f64,
}

/// Report for a complete training loop execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoopReport {
    /// Results from each training round.
    pub rounds: Vec<TrainingRoundResult>,
    /// Total duration of the training loop in seconds.
    pub total_duration_secs: f64,
    /// Total cost across all rounds in USD.
    pub total_cost: f64,
    /// Whether training converged before reaching the max round limit.
    pub converged: bool,
    /// Final evaluation score from the last round, if available.
    pub final_eval_score: Option<f64>,
}

/// Orchestrates autonomous training cycles with evaluation checkpoints.
///
/// The actual training and evaluation implementations are provided by
/// `brainwires-training` and `brainwires-eval` respectively. This struct
/// manages the loop logic, convergence detection, and reporting.
pub struct AutonomousTrainingLoop {
    config: TrainingLoopConfig,
}

impl AutonomousTrainingLoop {
    /// Create a new autonomous training loop with the given configuration.
    pub fn new(config: TrainingLoopConfig) -> Self {
        Self { config }
    }

    /// Get the configuration.
    pub fn config(&self) -> &TrainingLoopConfig {
        &self.config
    }

    /// Check if training should continue based on improvement between rounds.
    pub fn should_continue(
        &self,
        current_round: u32,
        prev_score: Option<f64>,
        current_score: Option<f64>,
    ) -> bool {
        if current_round >= self.config.max_rounds {
            return false;
        }

        match (prev_score, current_score) {
            (Some(prev), Some(curr)) => {
                let improvement = curr - prev;
                improvement >= self.config.min_improvement
            }
            _ => true, // Continue if we don't have scores yet
        }
    }

    /// Generate a report from collected round results.
    pub fn generate_report(
        &self,
        rounds: Vec<TrainingRoundResult>,
        total_duration_secs: f64,
        converged: bool,
    ) -> TrainingLoopReport {
        let total_cost: f64 = rounds.iter().map(|r| r.cost).sum();
        let final_eval_score = rounds.last().and_then(|r| r.eval_score);

        TrainingLoopReport {
            rounds,
            total_duration_secs,
            total_cost,
            converged,
            final_eval_score,
        }
    }
}
