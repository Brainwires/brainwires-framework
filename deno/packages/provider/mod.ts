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
  type AuthScheme,
  type ChatProtocol,
  createProviderConfig,
  defaultModel,
  parseProviderType,
  type ProviderConfig,
  type ProviderType,
  requiresApiKey,
} from "./types.ts";

// Registry
export { lookup, PROVIDER_REGISTRY, type ProviderEntry } from "./registry.ts";

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

// Brainwires Relay — HTTP-based backend that multiplexes upstream models
export {
  BrainwiresRelayProvider,
  DEFAULT_BACKEND_URL,
  DEV_BACKEND_URL,
  getBackendFromApiKey,
  maxOutputTokensForModel,
} from "./brainwires_relay.ts";

// Audio providers moved to @brainwires/provider-speech in v0.11.0.
// Transitional re-export keeps the old import path working through 0.11.x;
// remove this line in 0.12.0 and import from @brainwires/provider-speech.
export * from "@brainwires/provider-speech";

// Factory
export { ChatProviderFactory } from "./factory.ts";

// Rate limiter
export {
  RateLimitedClient,
  type RateLimitedClientOptions,
  RateLimiter,
} from "./rate_limiter.ts";

// Model listing
export {
  type AvailableModel,
  createModelLister,
  inferOpenaiCapabilities,
  isChatCapable,
  type ModelCapability,
  type ModelLister,
} from "./model_lister.ts";
