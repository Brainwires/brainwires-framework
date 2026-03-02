use crate::dataset::{Dataset, InstructDataset};
use crate::types::TrainingExample;

/// Split configuration for train/eval datasets.
#[derive(Debug, Clone)]
pub struct SplitConfig {
    /// Fraction of data for training (0.0 - 1.0).
    pub train_ratio: f32,
    /// Random seed for reproducible splits.
    pub seed: u64,
    /// Whether to shuffle before splitting.
    pub shuffle: bool,
}

impl Default for SplitConfig {
    fn default() -> Self {
        Self {
            train_ratio: 0.9,
            seed: 42,
            shuffle: true,
        }
    }
}

/// Split result containing train and eval datasets.
pub struct SplitResult {
    pub train: InstructDataset,
    pub eval: InstructDataset,
}

/// Split a dataset into train/eval sets.
pub fn train_eval_split(
    examples: &[TrainingExample],
    config: &SplitConfig,
) -> SplitResult {
    let mut dataset = InstructDataset::new(examples.to_vec());

    if config.shuffle {
        dataset.shuffle(config.seed);
    }

    let (train, eval) = dataset.split(config.train_ratio);

    SplitResult {
        train: InstructDataset::new(train),
        eval: InstructDataset::new(eval),
    }
}

/// Sort examples by token count (ascending) for curriculum learning.
pub fn curriculum_order(examples: &mut [TrainingExample]) {
    examples.sort_by_key(|e| e.estimated_tokens());
}

/// Sort examples by token count (descending) for anti-curriculum.
pub fn anti_curriculum_order(examples: &mut [TrainingExample]) {
    examples.sort_by(|a, b| b.estimated_tokens().cmp(&a.estimated_tokens()));
}

/// Sample `n` examples uniformly (with seed for reproducibility).
pub fn sample_n(examples: &[TrainingExample], n: usize, seed: u64) -> Vec<TrainingExample> {
    if n >= examples.len() {
        return examples.to_vec();
    }

    // Fisher-Yates partial shuffle
    let mut indices: Vec<usize> = (0..examples.len()).collect();
    let mut state = seed;
    for i in 0..n {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = i + ((state >> 33) as usize % (examples.len() - i));
        indices.swap(i, j);
    }

    indices[..n].iter().map(|&i| examples[i].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TrainingMessage;

    fn sample_examples(n: usize) -> Vec<TrainingExample> {
        (0..n)
            .map(|i| {
                TrainingExample::with_id(
                    format!("ex-{i}"),
                    vec![
                        TrainingMessage::user(format!("Q{}: {}", i, "x".repeat(i * 10))),
                        TrainingMessage::assistant(format!("A{}", i)),
                    ],
                )
            })
            .collect()
    }

    #[test]
    fn test_train_eval_split() {
        let examples = sample_examples(100);
        let result = train_eval_split(&examples, &SplitConfig::default());
        assert_eq!(result.train.len(), 90);
        assert_eq!(result.eval.len(), 10);
    }

    #[test]
    fn test_curriculum_order() {
        let mut examples = sample_examples(10);
        curriculum_order(&mut examples);
        for i in 1..examples.len() {
            assert!(examples[i].estimated_tokens() >= examples[i - 1].estimated_tokens());
        }
    }

    #[test]
    fn test_sample_n() {
        let examples = sample_examples(100);
        let sampled = sample_n(&examples, 10, 42);
        assert_eq!(sampled.len(), 10);

        // Deterministic
        let sampled2 = sample_n(&examples, 10, 42);
        for (a, b) in sampled.iter().zip(sampled2.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn test_sample_n_larger_than_dataset() {
        let examples = sample_examples(5);
        let sampled = sample_n(&examples, 100, 42);
        assert_eq!(sampled.len(), 5);
    }
}
