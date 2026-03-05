//! Google Vertex AI auth -- OAuth2 token acquisition.
//!
//! Feature-gated behind `vertex-ai`.

use anyhow::{Context, Result};

/// Vertex AI endpoint pattern:
/// `POST https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:streamRawPredict`
pub fn vertex_endpoint_url(region: &str, project_id: &str, model: &str) -> String {
    format!(
        "https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:streamRawPredict",
        region = region,
        project = project_id,
        model = model,
    )
}

/// Google OAuth2 authentication for Vertex AI requests.
pub struct VertexAuth {
    auth_manager: gcp_auth::AuthenticationManager,
    project_id: String,
    region: String,
}

impl VertexAuth {
    /// Create from default application credentials.
    pub async fn from_default(project_id: String, region: String) -> Result<Self> {
        let auth_manager = gcp_auth::AuthenticationManager::new().await
            .context("Failed to initialize GCP authentication")?;
        Ok(Self { auth_manager, project_id, region })
    }

    /// The GCP project ID.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// The GCP region.
    pub fn region(&self) -> &str {
        &self.region
    }

    /// Get a Bearer token for Vertex AI requests.
    pub async fn get_token(&self) -> Result<String> {
        let scopes = &["https://www.googleapis.com/auth/cloud-platform"];
        let token = self.auth_manager
            .get_token(scopes)
            .await
            .context("Failed to get GCP OAuth2 token")?;
        Ok(token.as_str().to_string())
    }

    /// Build the full endpoint URL for a given model.
    pub fn endpoint_url(&self, model: &str) -> String {
        vertex_endpoint_url(&self.region, &self.project_id, model)
    }
}
