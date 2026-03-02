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
pub struct BedrockFineTune {
    region: String,
    #[allow(dead_code)]
    client: Client,
    // AWS credentials are resolved via environment or IAM
    access_key_id: Option<String>,
    secret_access_key: Option<String>,
}

impl BedrockFineTune {
    pub fn new(region: impl Into<String>) -> Self {
        Self {
            region: region.into(),
            client: Client::new(),
            access_key_id: None,
            secret_access_key: None,
        }
    }

    pub fn with_credentials(
        mut self,
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
    ) -> Self {
        self.access_key_id = Some(access_key_id.into());
        self.secret_access_key = Some(secret_access_key.into());
        self
    }

    fn base_url(&self) -> String {
        format!("https://bedrock.{}.amazonaws.com", self.region)
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
        // Bedrock uses S3 URIs for training data, not direct upload.
        // This would need to upload to S3 first, then reference the S3 URI.
        Err(TrainingError::Provider(
            "Bedrock requires dataset upload to S3 first. Use upload_to_s3() then pass the S3 URI as DatasetId.".to_string(),
        ))
    }

    async fn create_job(&self, config: CloudFineTuneConfig) -> Result<TrainingJobId, TrainingError> {
        debug!("Creating Bedrock fine-tuning job for: {}", config.base_model);

        // Bedrock uses CreateModelCustomizationJob API
        // The training data must already be in S3
        let _url = format!("{}/model-customization-jobs", self.base_url());

        Err(TrainingError::Provider(
            "Bedrock fine-tuning requires AWS SDK integration. Use AWS SDK directly or configure credentials.".to_string(),
        ))
    }

    async fn get_job_status(&self, job_id: &TrainingJobId) -> Result<TrainingJobStatus, TrainingError> {
        debug!("Checking Bedrock job status: {}", job_id);

        Err(TrainingError::Provider(
            "Bedrock job status requires AWS SDK integration.".to_string(),
        ))
    }

    async fn cancel_job(&self, job_id: &TrainingJobId) -> Result<(), TrainingError> {
        debug!("Cancelling Bedrock job: {}", job_id);
        Err(TrainingError::Provider("Bedrock cancellation requires AWS SDK.".to_string()))
    }

    async fn list_jobs(&self) -> Result<Vec<TrainingJobSummary>, TrainingError> {
        Err(TrainingError::Provider("Bedrock job listing requires AWS SDK.".to_string()))
    }

    async fn delete_model(&self, model_id: &str) -> Result<(), TrainingError> {
        debug!("Deleting Bedrock model: {}", model_id);
        Err(TrainingError::Provider("Bedrock model deletion requires AWS SDK.".to_string()))
    }
}
