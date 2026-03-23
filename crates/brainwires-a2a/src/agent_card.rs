//! Agent card types: AgentCard, AgentCapabilities, AgentSkill, security schemes, OAuth flows.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Self-describing manifest for an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    /// Human-readable agent name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Agent version string.
    pub version: String,
    /// Ordered list of supported interfaces (first is preferred).
    #[serde(rename = "supportedInterfaces")]
    pub supported_interfaces: Vec<AgentInterface>,
    /// Agent capabilities.
    pub capabilities: AgentCapabilities,
    /// Agent skills.
    pub skills: Vec<AgentSkill>,
    /// Default input media types.
    #[serde(rename = "defaultInputModes")]
    pub default_input_modes: Vec<String>,
    /// Default output media types.
    #[serde(rename = "defaultOutputModes")]
    pub default_output_modes: Vec<String>,
    /// Service provider information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<AgentProvider>,
    /// Security scheme definitions.
    #[serde(rename = "securitySchemes", skip_serializing_if = "Option::is_none")]
    pub security_schemes: Option<HashMap<String, SecurityScheme>>,
    /// Security requirements.
    #[serde(
        rename = "securityRequirements",
        skip_serializing_if = "Option::is_none"
    )]
    pub security_requirements: Option<Vec<SecurityRequirement>>,
    /// URL to additional documentation.
    #[serde(rename = "documentationUrl", skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    /// Icon URL.
    #[serde(rename = "iconUrl", skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    /// JWS signatures for the agent card.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Vec<AgentCardSignature>>,
}

/// Declares a protocol binding interface for the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInterface {
    /// URL where this interface is available.
    pub url: String,
    /// Protocol binding: `JSONRPC`, `GRPC`, `HTTP+JSON`.
    #[serde(rename = "protocolBinding")]
    pub protocol_binding: String,
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// A2A protocol version.
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
}

/// Agent capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Supports streaming responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
    /// Supports push notifications.
    #[serde(rename = "pushNotifications", skip_serializing_if = "Option::is_none")]
    pub push_notifications: Option<bool>,
    /// Supports extended agent card.
    #[serde(rename = "extendedAgentCard", skip_serializing_if = "Option::is_none")]
    pub extended_agent_card: Option<bool>,
    /// Protocol extensions supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<AgentExtension>>,
}

/// A protocol extension declaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExtension {
    /// Unique URI identifying the extension.
    pub uri: String,
    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the client must comply.
    #[serde(default)]
    pub required: bool,
    /// Extension-specific parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// An agent's specific capability or function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    /// Unique skill identifier.
    pub id: String,
    /// Human-readable skill name.
    pub name: String,
    /// Detailed description.
    pub description: String,
    /// Keywords describing capabilities.
    pub tags: Vec<String>,
    /// Example prompts/scenarios.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,
    /// Override input modes for this skill.
    #[serde(rename = "inputModes", skip_serializing_if = "Option::is_none")]
    pub input_modes: Option<Vec<String>>,
    /// Override output modes for this skill.
    #[serde(rename = "outputModes", skip_serializing_if = "Option::is_none")]
    pub output_modes: Option<Vec<String>>,
    /// Security requirements for this skill.
    #[serde(
        rename = "securityRequirements",
        skip_serializing_if = "Option::is_none"
    )]
    pub security_requirements: Option<Vec<SecurityRequirement>>,
}

/// Agent service provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProvider {
    /// Provider website or documentation URL.
    pub url: String,
    /// Organization name.
    pub organization: String,
}

/// JWS signature for an AgentCard (RFC 7515).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCardSignature {
    /// Base64url-encoded protected JWS header.
    pub protected: String,
    /// Base64url-encoded computed signature.
    pub signature: String,
    /// Unprotected header values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<HashMap<String, serde_json::Value>>,
}

/// Security requirements map: scheme name -> required scopes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityRequirement {
    /// Map of security scheme names to their required scopes.
    pub schemes: HashMap<String, Vec<String>>,
}

/// Security scheme (wrapper-based oneOf).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityScheme {
    /// API key authentication.
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "apiKeySecurityScheme"
    )]
    pub api_key: Option<ApiKeySecurityScheme>,
    /// HTTP authentication (Bearer, Basic, etc).
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "httpAuthSecurityScheme"
    )]
    pub http_auth: Option<HttpAuthSecurityScheme>,
    /// OAuth 2.0 authentication.
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "oauth2SecurityScheme"
    )]
    pub oauth2: Option<OAuth2SecurityScheme>,
    /// OpenID Connect authentication.
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "openIdConnectSecurityScheme"
    )]
    pub open_id_connect: Option<OpenIdConnectSecurityScheme>,
    /// Mutual TLS authentication.
    #[serde(skip_serializing_if = "Option::is_none", rename = "mtlsSecurityScheme")]
    pub mtls: Option<MutualTlsSecurityScheme>,
}

/// API key security scheme details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiKeySecurityScheme {
    /// Parameter name.
    pub name: String,
    /// Location: `query`, `header`, or `cookie`.
    #[serde(rename = "in")]
    pub location: String,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// HTTP authentication security scheme details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpAuthSecurityScheme {
    /// Auth scheme name (e.g. `Bearer`).
    pub scheme: String,
    /// Format hint (e.g. `JWT`).
    #[serde(rename = "bearerFormat", skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OAuth 2.0 security scheme details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuth2SecurityScheme {
    /// OAuth2 flow configuration.
    pub flows: OAuthFlows,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// OAuth2 metadata URL (RFC 8414).
    #[serde(rename = "oauth2MetadataUrl", skip_serializing_if = "Option::is_none")]
    pub oauth2_metadata_url: Option<String>,
}

/// OpenID Connect security scheme details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenIdConnectSecurityScheme {
    /// OIDC discovery URL.
    #[serde(rename = "openIdConnectUrl")]
    pub open_id_connect_url: String,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Mutual TLS security scheme details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutualTlsSecurityScheme {
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OAuth 2.0 flow configuration (wrapper-based oneOf).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuthFlows {
    /// Authorization Code flow.
    #[serde(skip_serializing_if = "Option::is_none", rename = "authorizationCode")]
    pub authorization_code: Option<AuthorizationCodeOAuthFlow>,
    /// Client Credentials flow.
    #[serde(skip_serializing_if = "Option::is_none", rename = "clientCredentials")]
    pub client_credentials: Option<ClientCredentialsOAuthFlow>,
    /// Implicit flow (deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<ImplicitOAuthFlow>,
    /// Password flow (deprecated).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<PasswordOAuthFlow>,
    /// Device Code flow (RFC 8628).
    #[serde(skip_serializing_if = "Option::is_none", rename = "deviceCode")]
    pub device_code: Option<DeviceCodeOAuthFlow>,
}

/// Authorization Code OAuth flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthorizationCodeOAuthFlow {
    /// Authorization URL.
    #[serde(rename = "authorizationUrl")]
    pub authorization_url: String,
    /// Token URL.
    #[serde(rename = "tokenUrl")]
    pub token_url: String,
    /// Refresh URL.
    #[serde(rename = "refreshUrl", skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    /// Available scopes.
    pub scopes: HashMap<String, String>,
    /// Whether PKCE is required.
    #[serde(rename = "pkceRequired", skip_serializing_if = "Option::is_none")]
    pub pkce_required: Option<bool>,
}

/// Client Credentials OAuth flow.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClientCredentialsOAuthFlow {
    /// Token URL.
    #[serde(rename = "tokenUrl")]
    pub token_url: String,
    /// Refresh URL.
    #[serde(rename = "refreshUrl", skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    /// Available scopes.
    pub scopes: HashMap<String, String>,
}

/// Implicit OAuth flow (deprecated).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImplicitOAuthFlow {
    /// Authorization URL.
    #[serde(rename = "authorizationUrl", skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    /// Refresh URL.
    #[serde(rename = "refreshUrl", skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    /// Available scopes.
    #[serde(default)]
    pub scopes: HashMap<String, String>,
}

/// Password OAuth flow (deprecated).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PasswordOAuthFlow {
    /// Token URL.
    #[serde(rename = "tokenUrl", skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    /// Refresh URL.
    #[serde(rename = "refreshUrl", skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    /// Available scopes.
    #[serde(default)]
    pub scopes: HashMap<String, String>,
}

/// Device Code OAuth flow (RFC 8628).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceCodeOAuthFlow {
    /// Device authorization URL.
    #[serde(rename = "deviceAuthorizationUrl")]
    pub device_authorization_url: String,
    /// Token URL.
    #[serde(rename = "tokenUrl")]
    pub token_url: String,
    /// Refresh URL.
    #[serde(rename = "refreshUrl", skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    /// Available scopes.
    pub scopes: HashMap<String, String>,
}
