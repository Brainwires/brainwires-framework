/**
 * Agent card types: AgentCard, AgentInterface, AgentCapabilities,
 * AgentSkill, AgentProvider, AgentExtension, SecurityScheme,
 * SecurityRequirement, OAuthFlows, AgentCardSignature.
 *
 * Serialization rules:
 * - `SecurityScheme` is a wrapper OneOf (exactly one field set)
 * - `OAuthFlows` is a wrapper OneOf (exactly one field set)
 */

/** Self-describing manifest for an agent. */
export interface AgentCard {
  /** Human-readable agent name. */
  name: string;
  /** Human-readable description. */
  description: string;
  /** Agent version string. */
  version: string;
  /** Ordered list of supported interfaces (first is preferred). Required. */
  supportedInterfaces: AgentInterface[];
  /** Agent capabilities. */
  capabilities: AgentCapabilities;
  /** Agent skills. */
  skills: AgentSkill[];
  /** Default input media types. */
  defaultInputModes: string[];
  /** Default output media types. */
  defaultOutputModes: string[];
  /** Service provider information. */
  provider?: AgentProvider;
  /** Security scheme definitions. */
  securitySchemes?: Record<string, SecurityScheme>;
  /** Security requirements. */
  securityRequirements?: SecurityRequirement[];
  /** URL to additional documentation. */
  documentationUrl?: string;
  /** Icon URL. */
  iconUrl?: string;
  /** JWS signatures for the agent card. */
  signatures?: AgentCardSignature[];
}

/** Declares a protocol binding interface for the agent. */
export interface AgentInterface {
  /** URL where this interface is available. */
  url: string;
  /** Protocol binding: `JSONRPC`, `GRPC`, `HTTP+JSON`. */
  protocolBinding: string;
  /** Optional tenant identifier. */
  tenant?: string;
  /** A2A protocol version. */
  protocolVersion: string;
}

/** Agent capabilities. */
export interface AgentCapabilities {
  /** Supports streaming responses. */
  streaming?: boolean;
  /** Supports push notifications. */
  pushNotifications?: boolean;
  /** Supports extended agent card. */
  extendedAgentCard?: boolean;
  /** Protocol extensions supported. */
  extensions?: AgentExtension[];
}

/** A protocol extension declaration. */
export interface AgentExtension {
  /** Unique URI identifying the extension. */
  uri: string;
  /** Human-readable description. */
  description?: string;
  /** Whether the client must comply. */
  required: boolean;
  /** Extension-specific parameters. */
  params?: Record<string, unknown>;
}

/** An agent's specific capability or function. */
export interface AgentSkill {
  /** Unique skill identifier. */
  id: string;
  /** Human-readable skill name. */
  name: string;
  /** Detailed description. */
  description: string;
  /** Keywords describing capabilities. */
  tags: string[];
  /** Example prompts/scenarios. */
  examples?: string[];
  /** Override input modes for this skill. */
  inputModes?: string[];
  /** Override output modes for this skill. */
  outputModes?: string[];
  /** Security requirements for this skill. */
  securityRequirements?: SecurityRequirement[];
}

/** Agent service provider. */
export interface AgentProvider {
  /** Provider website or documentation URL. */
  url: string;
  /** Organization name. */
  organization: string;
}

/** JWS signature for an AgentCard (RFC 7515). */
export interface AgentCardSignature {
  /** Base64url-encoded protected JWS header. */
  protected: string;
  /** Base64url-encoded computed signature. */
  signature: string;
  /** Unprotected header values. */
  header?: Record<string, unknown>;
}

/** Security requirements map: scheme name -> required scopes. */
export interface SecurityRequirement {
  /** Map of security scheme names to their required scopes. */
  schemes: Record<string, string[]>;
}

/**
 * Security scheme (wrapper OneOf).
 * Exactly one field should be set.
 */
export interface SecurityScheme {
  /** API key security scheme. */
  apiKeySecurityScheme?: ApiKeySecurityScheme;
  /** HTTP auth security scheme. */
  httpAuthSecurityScheme?: HttpAuthSecurityScheme;
  /** OAuth2 security scheme. */
  oauth2SecurityScheme?: OAuth2SecurityScheme;
  /** OpenID Connect security scheme. */
  openIdConnectSecurityScheme?: OpenIdConnectSecurityScheme;
  /** Mutual TLS security scheme. */
  mtlsSecurityScheme?: MutualTlsSecurityScheme;
}

export interface ApiKeySecurityScheme {
  /** Parameter name. */
  name: string;
  /** Location: `query`, `header`, or `cookie`. */
  in: string;
  /** Description. */
  description?: string;
}

export interface HttpAuthSecurityScheme {
  /** Auth scheme name (e.g. `Bearer`). */
  scheme: string;
  /** Format hint (e.g. `JWT`). */
  bearerFormat?: string;
  /** Description. */
  description?: string;
}

export interface OAuth2SecurityScheme {
  /** OAuth2 flow configuration. */
  flows: OAuthFlows;
  /** Description. */
  description?: string;
  /** OAuth2 metadata URL (RFC 8414). */
  oauth2MetadataUrl?: string;
}

export interface OpenIdConnectSecurityScheme {
  /** OIDC discovery URL. */
  openIdConnectUrl: string;
  /** Description. */
  description?: string;
}

export interface MutualTlsSecurityScheme {
  /** Description. */
  description?: string;
}

/**
 * OAuth 2.0 flow configuration (wrapper OneOf).
 * Exactly one field should be set.
 */
export interface OAuthFlows {
  /** Authorization code flow. */
  authorizationCode?: AuthorizationCodeOAuthFlow;
  /** Client credentials flow. */
  clientCredentials?: ClientCredentialsOAuthFlow;
  /** Implicit flow (deprecated). */
  implicit?: ImplicitOAuthFlow;
  /** Password flow (deprecated). */
  password?: PasswordOAuthFlow;
  /** Device code flow. */
  deviceCode?: DeviceCodeOAuthFlow;
}

export interface AuthorizationCodeOAuthFlow {
  /** Authorization URL. */
  authorizationUrl: string;
  /** Token URL. */
  tokenUrl: string;
  /** Refresh URL. */
  refreshUrl?: string;
  /** Available scopes. */
  scopes: Record<string, string>;
  /** Whether PKCE is required. */
  pkceRequired?: boolean;
}

export interface ClientCredentialsOAuthFlow {
  /** Token URL. */
  tokenUrl: string;
  /** Refresh URL. */
  refreshUrl?: string;
  /** Available scopes. */
  scopes: Record<string, string>;
}

export interface ImplicitOAuthFlow {
  /** Authorization URL. */
  authorizationUrl?: string;
  /** Refresh URL. */
  refreshUrl?: string;
  /** Available scopes. */
  scopes: Record<string, string>;
}

export interface PasswordOAuthFlow {
  /** Token URL. */
  tokenUrl?: string;
  /** Refresh URL. */
  refreshUrl?: string;
  /** Available scopes. */
  scopes: Record<string, string>;
}

export interface DeviceCodeOAuthFlow {
  /** Device authorization URL. */
  deviceAuthorizationUrl: string;
  /** Token URL. */
  tokenUrl: string;
  /** Refresh URL. */
  refreshUrl?: string;
  /** Available scopes. */
  scopes: Record<string, string>;
}
