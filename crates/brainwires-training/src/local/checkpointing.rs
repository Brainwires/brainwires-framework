use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

/// Checkpoint metadata stored alongside model weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMeta {
    /// Current epoch number.
    pub epoch: u32,
    /// Global training step count.
    pub step: u64,
    /// Training loss at this checkpoint.
    pub train_loss: f64,
    /// Evaluation loss at this checkpoint (if available).
    pub eval_loss: Option<f64>,
    /// Current learning rate.
    pub learning_rate: f64,
    /// Timestamp when this checkpoint was saved.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Manages saving and loading training checkpoints.
pub struct CheckpointManager {
    /// Base directory for checkpoints.
    output_dir: PathBuf,
    /// Maximum number of checkpoints to keep.
    max_checkpoints: usize,
    /// Save a checkpoint every N steps.
    save_every_steps: u64,
}

impl CheckpointManager {
    /// Create a new checkpoint manager writing to the given directory.
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
            max_checkpoints: 3,
            save_every_steps: 500,
        }
    }

    /// Set the maximum number of checkpoints to retain.
    pub fn with_max_checkpoints(mut self, max: usize) -> Self {
        self.max_checkpoints = max;
        self
    }

    /// Set the checkpoint save interval in training steps.
    pub fn with_save_every_steps(mut self, steps: u64) -> Self {
        self.save_every_steps = steps;
        self
    }

    /// Whether we should save a checkpoint at this step.
    pub fn should_save(&self, step: u64) -> bool {
        step > 0 && step.is_multiple_of(self.save_every_steps)
    }

    /// Get the path for a checkpoint at a given step.
    pub fn checkpoint_path(&self, step: u64) -> PathBuf {
        self.output_dir.join(format!("checkpoint-{}", step))
    }

    /// Save checkpoint metadata.
    pub fn save_meta(&self, step: u64, meta: &CheckpointMeta) -> std::io::Result<()> {
        let dir = self.checkpoint_path(step);
        std::fs::create_dir_all(&dir)?;

        let meta_path = dir.join("checkpoint_meta.json");
        let json = serde_json::to_string_pretty(meta)
            .map_err(std::io::Error::other)?;
        std::fs::write(&meta_path, json)?;

        info!("Saved checkpoint at step {} to {:?}", step, dir);
        self.cleanup_old_checkpoints()?;
        Ok(())
    }

    /// Load checkpoint metadata from a directory.
    pub fn load_meta(checkpoint_dir: &Path) -> std::io::Result<CheckpointMeta> {
        let meta_path = checkpoint_dir.join("checkpoint_meta.json");
        let json = std::fs::read_to_string(&meta_path)?;
        let meta: CheckpointMeta = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(meta)
    }

    /// Find the latest checkpoint in the output directory.
    pub fn latest_checkpoint(&self) -> Option<PathBuf> {
        let mut checkpoints = self.list_checkpoints();
        checkpoints.sort_by_key(|(step, _)| std::cmp::Reverse(*step));
        checkpoints.into_iter().next().map(|(_, path)| path)
    }

    /// List all checkpoints with their step numbers.
    pub fn list_checkpoints(&self) -> Vec<(u64, PathBuf)> {
        let Ok(entries) = std::fs::read_dir(&self.output_dir) else {
            return Vec::new();
        };

        entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.starts_with("checkpoint-") {
                    let step_str = name.strip_prefix("checkpoint-")?;
                    let step: u64 = step_str.parse().ok()?;
                    Some((step, e.path()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Remove old checkpoints, keeping only the most recent `max_checkpoints`.
    fn cleanup_old_checkpoints(&self) -> std::io::Result<()> {
        let mut checkpoints = self.list_checkpoints();
        checkpoints.sort_by_key(|(step, _)| *step);

        while checkpoints.len() > self.max_checkpoints {
            if let Some((step, path)) = checkpoints.first() {
                debug!("Removing old checkpoint: step {}", step);
                std::fs::remove_dir_all(path)?;
                checkpoints.remove(0);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_save() {
        let mgr = CheckpointManager::new("/tmp/test").with_save_every_steps(100);
        assert!(!mgr.should_save(0));
        assert!(!mgr.should_save(50));
        assert!(mgr.should_save(100));
        assert!(mgr.should_save(200));
    }

    #[test]
    fn test_checkpoint_path() {
        let mgr = CheckpointManager::new("/tmp/training");
        assert_eq!(
            mgr.checkpoint_path(500),
            PathBuf::from("/tmp/training/checkpoint-500")
        );
    }
}
