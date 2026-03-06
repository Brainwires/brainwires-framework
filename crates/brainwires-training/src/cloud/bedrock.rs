use async_trait::async_trait;
use reqwest::Client;
use tracing::debug;

use brainwires_datasets::DataFormat;

use crate::error::TrainingError;
use crate::types::{TrainingJobId, TrainingJobStatus, TrainingJobSummary, DatasetId};
use super::{CloudFineTuneConfig, FineTuneProvider};

/// AWS Bedrock fine-tuning provider.
///
/// Supports Claude Haiku fine-tuning and other Bedrock foundation models.
/// Requires AWS credentials (access key + secret or IAM role).
///
/// **Status**: Not yet implemented. Requires AWS SDK integration for SigV4 signing.
pub struct BedrockFineTune {
    region: String,
    #[allow(dead_code)]
    client: Client,
    #[allow(dead_code)]
    access_key_id: Option<String>,
    #[allow(dead_code)]
    secret_access_key: Option<String>,
}

impl BedrockFineTune {
    /// Create a new AWS Bedrock fine-tune provider.
    pub fn new(region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            client: Client::new(),
            access_key_id: None,
            secret_access_key: None,
        }
    }

    /// Set explicit AWS credentials.
    pub fn with_credentials(
        mut self,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        self.access_key_id = Some(access_key_id.into());
        self.secret_access_key = Some(secret_access_key.into());
        self
    }

    #[allow(dead_code)]
    fn base_url(&self) -> String {
        format!("https://bedrock.{}.amazonaws.com", self.region)
    }

    fn not_implemented(&self, feature: &str) -> TrainingError {
        TrainingError::NotImplemented {
            provider: "AWS Bedrock".to_string(),
            feature: format!("{} (requires AWS SDK for SigV4 request signing)", feature),
        }
    }
}

#[async_trait]
impl FineTuneProvider for BedrockFineTune {
    fn name(&self) -> &str {
        "bedrock"
    }

    fn supported_base_models(&self) -> Vec<String> {
        vec![
            "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
            "meta.llama3-1-8b-instruct-v1:0".to_string(),
            "amazon.titan-text-lite-v1".to_string(),
        ]
    }

    fn supports_dpo(&self) -> bool {
        false
    }

    async fn upload_dataset(&self, data: &[u8], _format: DataFormat) -> Result<DatasetId, TrainingError> {
        debug!("Bedrock fine-tuning requires data in S3. Dataset size: {} bytes", data.len());
        Err(self.not_implemented("dataset upload (data must be in S3)"))
    }

    async fn create_job(&self, config: CloudFineTuneConfig) -> Result<TrainingJobId, TrainingError> {
        debug!("Creating Bedrock fine-tuning job for: {}", config.base_model);
        Err(self.not_implemented("job creation"))
    }

    async fn get_job_status(&self, job_id: &TrainingJobId) -> Result<TrainingJobStatus, TrainingError> {
        debug!("Checking Bedrock job status: {}", job_id);
        Err(self.not_implemented("job status"))
    }

    async fn cancel_job(&self, job_id: &TrainingJobId) -> Result<(), TrainingError> {
        debug!("Cancelling Bedrock job: {}", job_id);
        Err(self.not_implemented("job cancellation"))
    }

    async fn list_jobs(&self) -> Result<Vec<TrainingJobSummary>, TrainingError> {
        Err(self.not_implemented("job listing"))
    }

    async fn delete_model(&self, model_id: &str) -> Result<(), TrainingError> {
        debug!("Deleting Bedrock model: {}", model_id);
        Err(self.not_implemented("model deletion"))
    }
}
