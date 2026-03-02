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
    pub round: u32,
    pub dataset_size: usize,
    pub training_loss: f64,
    pub validation_loss: Option<f64>,
    pub eval_score: Option<f64>,
    pub cost: f64,
    pub duration_secs: f64,
}

/// Report for a complete training loop execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingLoopReport {
    pub rounds: Vec<TrainingRoundResult>,
    pub total_duration_secs: f64,
    pub total_cost: f64,
    pub converged: bool,
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
