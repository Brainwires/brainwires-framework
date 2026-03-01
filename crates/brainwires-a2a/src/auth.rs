//! Authentication configuration for A2A communication.
//!
//! The A2A protocol supports multiple authentication schemes. An agent's
//! [`AgentCard`](crate::AgentCard) can advertise required auth, and clients
//! use [`AuthConfig`] to supply credentials when connecting.

use serde::{Deserialize, Serialize};

/// Supported authentication schemes for A2A endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthScheme {
    /// API key passed as a header or query parameter.
    ApiKey,

    /// OAuth 2.0 flow.
    OAuth2,

    /// JSON Web Token authentication.
    Jwt,

    /// Bearer token authentication.
    Bearer,

    /// No authentication required.
    None,
}

/// Authentication configuration for connecting to an A2A agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    /// The authentication scheme to use.
    pub scheme: AuthScheme,

    /// Credentials or token value. The interpretation depends on `scheme`:
    /// - `ApiKey`: the raw API key string
    /// - `OAuth2`: an access token (obtain via OAuth flow externally)
    /// - `Jwt`: a signed JWT string
    /// - `Bearer`: a bearer token string
    /// - `None`: ignored
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            scheme: AuthScheme::None,
            credentials: None,
        }
    }
}
