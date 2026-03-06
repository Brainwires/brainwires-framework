use async_trait::async_trait;
use reqwest::Client;
use tracing::debug;

use brainwires_datasets::DataFormat;

use crate::error::TrainingError;
use crate::types::{TrainingJobId, TrainingJobStatus, TrainingJobSummary, DatasetId};
use super::{CloudFineTuneConfig, FineTuneProvider};

/// Google Vertex AI fine-tuning provider.
///
/// Supports Gemini model tuning (enterprise only).
/// Requires GCP service account credentials.
///
/// **Status**: Not yet implemented. Requires GCP OAuth2/service account auth.
pub struct VertexFineTune {
    project_id: String,
    location: String,
    #[allow(dead_code)]
    client: Client,
    #[allow(dead_code)]
    access_token: Option<String>,
}

impl VertexFineTune {
    /// Create a new Google Vertex AI fine-tune provider.
    pub fn new(project_id: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            location: location.into(),
            client: Client::new(),
            access_token: None,
        }
    }

    /// Set an explicit access token.
    pub fn with_access_token(mut self, token: impl Into<String>) -> Self {
        self.access_token = Some(token.into());
        self
    }

    #[allow(dead_code)]
    fn base_url(&self) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}",
            self.location, self.project_id, self.location
        )
    }

    fn not_implemented(&self, feature: &str) -> TrainingError {
        TrainingError::NotImplemented {
            provider: "Google Vertex AI".to_string(),
            feature: format!("{} (requires GCP OAuth2/service account authentication)", feature),
        }
    }
}

#[async_trait]
impl FineTuneProvider for VertexFineTune {
    fn name(&self) -> &str {
        "vertex"
    }

    fn supported_base_models(&self) -> Vec<String> {
        vec![
            "gemini-1.5-flash-002".to_string(),
            "gemini-1.5-pro-002".to_string(),
        ]
    }

    fn supports_dpo(&self) -> bool {
        false // Vertex uses RLHF, not DPO
    }

    async fn upload_dataset(&self, data: &[u8], _format: DataFormat) -> Result<DatasetId, TrainingError> {
        debug!(
            "Vertex AI fine-tuning requires data in GCS. Dataset size: {} bytes",
            data.len()
        );
        Err(self.not_implemented("dataset upload (data must be in GCS)"))
    }

    async fn create_job(&self, config: CloudFineTuneConfig) -> Result<TrainingJobId, TrainingError> {
        debug!("Creating Vertex AI tuning job for: {}", config.base_model);
        Err(self.not_implemented("job creation"))
    }

    async fn get_job_status(&self, job_id: &TrainingJobId) -> Result<TrainingJobStatus, TrainingError> {
        debug!("Checking Vertex AI job status: {}", job_id);
        Err(self.not_implemented("job status"))
    }

    async fn cancel_job(&self, job_id: &TrainingJobId) -> Result<(), TrainingError> {
        debug!("Cancelling Vertex AI job: {}", job_id);
        Err(self.not_implemented("job cancellation"))
    }

    async fn list_jobs(&self) -> Result<Vec<TrainingJobSummary>, TrainingError> {
        Err(self.not_implemented("job listing"))
    }

    async fn delete_model(&self, model_id: &str) -> Result<(), TrainingError> {
        debug!("Deleting Vertex AI model: {}", model_id);
        Err(self.not_implemented("model deletion"))
    }
}
