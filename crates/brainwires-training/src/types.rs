use serde::{Deserialize, Serialize};

/// Unique identifier for a training job.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrainingJobId(pub String);

impl std::fmt::Display for TrainingJobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<S: Into<String>> From<S> for TrainingJobId {
    fn from(s: S) -> Self {
        Self(s.into())
    }
}

/// Unique identifier for an uploaded dataset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatasetId(pub String);

impl std::fmt::Display for DatasetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<S: Into<String>> From<S> for DatasetId {
    fn from(s: S) -> Self {
        Self(s.into())
    }
}

/// Status of a training job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TrainingJobStatus {
    Pending,
    Validating,
    Queued,
    Running {
        progress: TrainingProgress,
    },
    Succeeded {
        model_id: String,
    },
    Failed {
        error: String,
    },
    Cancelled,
}

impl TrainingJobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Succeeded { .. } | Self::Failed { .. } | Self::Cancelled)
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }

    pub fn is_succeeded(&self) -> bool {
        matches!(self, Self::Succeeded { .. })
    }
}

/// Progress information for a running training job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrainingProgress {
    pub epoch: u32,
    pub total_epochs: u32,
    pub step: u64,
    pub total_steps: u64,
    pub train_loss: Option<f64>,
    pub eval_loss: Option<f64>,
    pub learning_rate: Option<f64>,
    pub elapsed_secs: u64,
}

impl TrainingProgress {
    pub fn completion_fraction(&self) -> f64 {
        if self.total_steps == 0 {
            return 0.0;
        }
        self.step as f64 / self.total_steps as f64
    }
}

/// Metrics from a completed training job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub final_train_loss: Option<f64>,
    pub final_eval_loss: Option<f64>,
    pub total_steps: u64,
    pub total_epochs: u32,
    pub total_tokens_trained: Option<u64>,
    pub duration_secs: u64,
    pub estimated_cost_usd: Option<f64>,
}

/// Summary of a training job for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJobSummary {
    pub job_id: TrainingJobId,
    pub provider: String,
    pub base_model: String,
    pub status: TrainingJobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metrics: Option<TrainingMetrics>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_status_terminal() {
        assert!(!TrainingJobStatus::Pending.is_terminal());
        assert!(!TrainingJobStatus::Queued.is_terminal());
        assert!(TrainingJobStatus::Succeeded { model_id: "m".into() }.is_terminal());
        assert!(TrainingJobStatus::Failed { error: "err".into() }.is_terminal());
        assert!(TrainingJobStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_progress_completion() {
        let p = TrainingProgress {
            step: 50,
            total_steps: 100,
            ..Default::default()
        };
        assert!((p.completion_fraction() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_job_id_from_string() {
        let id: TrainingJobId = "ft-abc123".into();
        assert_eq!(id.0, "ft-abc123");
        assert_eq!(id.to_string(), "ft-abc123");
    }
}
