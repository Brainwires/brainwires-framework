/**
 * @module @brainwires/providers
 *
 * Provider layer for the Brainwires Agent Framework.
 * Contains chat provider implementations that wrap AI APIs with the
 * `Provider` interface from `@brainwires/core`.
 *
 * Equivalent to Rust's `brainwires-providers` crate.
 */

// Types
export {
  createProviderConfig,
  defaultModel,
  parseProviderType,
  requiresApiKey,
  type AuthScheme,
  type ChatProtocol,
  type ProviderConfig,
  type ProviderType,
} from "./types.ts";

// Registry
export {
  lookup,
  PROVIDER_REGISTRY,
  type ProviderEntry,
} from "./registry.ts";

// SSE parsing utilities
export { parseNDJSONStream, parseSSEStream } from "./sse.ts";

// Providers
export { AnthropicChatProvider } from "./anthropic.ts";
export { OpenAiChatProvider } from "./openai.ts";
export { OpenAiResponsesProvider } from "./openai_responses.ts";
export { BedrockProvider } from "./bedrock.ts";
export { VertexAiProvider } from "./vertex.ts";
export { GoogleChatProvider } from "./gemini.ts";
export { OllamaChatProvider } from "./ollama.ts";

// Factory
export { ChatProviderFactory } from "./factory.ts";

// Rate limiter
export { RateLimitedClient, RateLimiter, type RateLimitedClientOptions } from "./rate_limiter.ts";

// Model listing
export {
  createModelLister,
  inferOpenaiCapabilities,
  isChatCapable,
  type AvailableModel,
  type ModelCapability,
  type ModelLister,
} from "./model_lister.ts";
