//! Google Vertex AI auth -- OAuth2 token acquisition.
//!
//! Feature-gated behind `vertex-ai`.

use std::sync::Arc;

use anyhow::{Context, Result};
use gcp_auth::TokenProvider;
use tokio::sync::OnceCell;

/// Vertex AI streaming endpoint:
/// `POST https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:streamRawPredict`
pub fn vertex_stream_url(region: &str, project_id: &str, model: &str) -> String {
    format!(
        "https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:streamRawPredict",
        region = region,
        project = project_id,
        model = model,
    )
}

/// Vertex AI non-streaming endpoint:
/// `POST https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:rawPredict`
pub fn vertex_raw_predict_url(region: &str, project_id: &str, model: &str) -> String {
    format!(
        "https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:rawPredict",
        region = region,
        project = project_id,
        model = model,
    )
}

/// Google OAuth2 authentication for Vertex AI requests.
///
/// Uses lazy initialization for the token provider so that construction
/// is synchronous (compatible with `ChatProviderFactory::create()`).
pub struct VertexAuth {
    token_provider: OnceCell<Arc<dyn TokenProvider>>,
    project_id: String,
    region: String,
}

impl VertexAuth {
    /// Create a new VertexAuth with lazy token provider initialization.
    ///
    /// The GCP token provider is initialized on first `get_token()` call.
    pub fn new(project_id: String, region: String) -> Self {
        Self {
            token_provider: OnceCell::new(),
            project_id,
            region,
        }
    }

    /// The GCP project ID.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// The GCP region.
    pub fn region(&self) -> &str {
        &self.region
    }

    /// Lazily initialize and return the token provider.
    async fn provider(&self) -> Result<&Arc<dyn TokenProvider>> {
        self.token_provider
            .get_or_try_init(|| async {
                let provider = gcp_auth::provider().await
                    .context("Failed to initialize GCP authentication. Ensure Application Default Credentials are configured.")?;
                Ok(provider)
            })
            .await
    }

    /// Get a Bearer token for Vertex AI requests.
    pub async fn get_token(&self) -> Result<String> {
        let provider = self.provider().await?;
        let scopes = &["https://www.googleapis.com/auth/cloud-platform"];
        let token = provider
            .token(scopes)
            .await
            .context("Failed to get GCP OAuth2 token")?;
        Ok(token.as_str().to_string())
    }

    /// Build the streaming endpoint URL for a given model.
    pub fn stream_url(&self, model: &str) -> String {
        vertex_stream_url(&self.region, &self.project_id, model)
    }

    /// Build the non-streaming endpoint URL for a given model.
    pub fn raw_predict_url(&self, model: &str) -> String {
        vertex_raw_predict_url(&self.region, &self.project_id, model)
    }
}
