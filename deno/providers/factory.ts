/**
 * Chat provider factory — registry-driven protocol dispatch.
 * Creates Provider instances from ProviderConfig by looking up the provider
 * in the registry and dispatching to the appropriate protocol handler.
 * Equivalent to Rust's `chat_factory.rs`.
 */

import type { Provider } from "@brainwires/core";
import type { ProviderConfig } from "./types.ts";
import { lookup } from "./registry.ts";
import { AnthropicChatProvider } from "./anthropic.ts";
import { OpenAiChatProvider } from "./openai.ts";
import { GoogleChatProvider } from "./gemini.ts";
import { OllamaChatProvider } from "./ollama.ts";

/** Pure chat provider factory -- creates provider instances from config.
 *
 * No CLI dependencies (no keyring, no file I/O).
 * The caller is responsible for resolving API keys and base URLs
 * before calling `create()`.
 *
 * Equivalent to Rust's `ChatProviderFactory`. */
export class ChatProviderFactory {
  /** Create a chat provider from a fully-resolved config.
   * All fields (api_key, base_url, model) must already be populated. */
  static create(config: ProviderConfig): Provider {
    const entry = lookup(config.provider);
    if (!entry) {
      throw new Error(
        `Provider type '${config.provider}' is not a chat provider`,
      );
    }

    switch (entry.chat_protocol) {
      case "openai_chat_completions":
        return ChatProviderFactory.createOpenAiCompat(
          config,
          entry.default_base_url,
        );
      case "anthropic_messages":
        return ChatProviderFactory.createAnthropic(config);
      case "gemini_generate_content":
        return ChatProviderFactory.createGemini(config);
      case "ollama_chat":
        return ChatProviderFactory.createOllama(config);
      case "openai_responses":
        // OpenAI Responses API not yet implemented in Deno port;
        // fall through to error for now.
        throw new Error(
          "OpenAI Responses API provider is not yet implemented in the Deno port",
        );
      case "brainwires_relay":
        throw new Error(
          "Brainwires relay provider is not yet implemented in the Deno port",
        );
      default:
        throw new Error(
          `Unsupported chat protocol: ${entry.chat_protocol}`,
        );
    }
  }

  // -----------------------------------------------------------------------
  // Protocol-specific constructors
  // -----------------------------------------------------------------------

  private static createOpenAiCompat(
    config: ProviderConfig,
    defaultBaseUrl: string,
  ): Provider {
    const apiKey = config.api_key;
    if (!apiKey) {
      throw new Error(
        `${config.provider} provider requires an API key`,
      );
    }
    const baseUrl = config.base_url ?? defaultBaseUrl;
    return new OpenAiChatProvider(
      apiKey,
      config.model,
      baseUrl,
      config.provider,
    );
  }

  private static createAnthropic(config: ProviderConfig): Provider {
    const apiKey = config.api_key;
    if (!apiKey) {
      throw new Error(
        `${config.provider} provider requires an API key`,
      );
    }
    return new AnthropicChatProvider(apiKey, config.model, config.provider);
  }

  private static createGemini(config: ProviderConfig): Provider {
    const apiKey = config.api_key;
    if (!apiKey) {
      throw new Error("Google provider requires an API key");
    }
    return new GoogleChatProvider(apiKey, config.model);
  }

  private static createOllama(config: ProviderConfig): Provider {
    return new OllamaChatProvider(config.model, config.base_url);
  }
}
