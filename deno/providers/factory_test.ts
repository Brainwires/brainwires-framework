import { assertEquals, assertThrows } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { ChatProviderFactory } from "./factory.ts";
import type { ProviderConfig } from "./types.ts";

Deno.test("ChatProviderFactory.create - ollama no key required", () => {
  const config: ProviderConfig = {
    provider: "ollama",
    model: "llama3.1",
  };
  const provider = ChatProviderFactory.create(config);
  assertEquals(provider.name, "ollama");
});

Deno.test("ChatProviderFactory.create - anthropic requires key", () => {
  const config: ProviderConfig = {
    provider: "anthropic",
    model: "claude-3",
  };
  assertThrows(
    () => ChatProviderFactory.create(config),
    Error,
    "requires an API key",
  );
});

Deno.test("ChatProviderFactory.create - anthropic with key", () => {
  const config: ProviderConfig = {
    provider: "anthropic",
    model: "claude-3-sonnet",
    api_key: "sk-ant-test",
  };
  const provider = ChatProviderFactory.create(config);
  assertEquals(provider.name, "anthropic");
});

Deno.test("ChatProviderFactory.create - openai with key", () => {
  const config: ProviderConfig = {
    provider: "openai",
    model: "gpt-4",
    api_key: "sk-test",
  };
  const provider = ChatProviderFactory.create(config);
  assertEquals(provider.name, "openai");
});

Deno.test("ChatProviderFactory.create - groq with key", () => {
  const config: ProviderConfig = {
    provider: "groq",
    model: "llama-3.3-70b-versatile",
    api_key: "gsk_test",
  };
  const provider = ChatProviderFactory.create(config);
  assertEquals(provider.name, "groq");
});

Deno.test("ChatProviderFactory.create - together with key", () => {
  const config: ProviderConfig = {
    provider: "together",
    model: "meta-llama/Llama-3.1-8B-Instruct",
    api_key: "tok_test",
  };
  const provider = ChatProviderFactory.create(config);
  assertEquals(provider.name, "together");
});

Deno.test("ChatProviderFactory.create - fireworks with key", () => {
  const config: ProviderConfig = {
    provider: "fireworks",
    model: "llama-v3p1-8b-instruct",
    api_key: "fw_test",
  };
  const provider = ChatProviderFactory.create(config);
  assertEquals(provider.name, "fireworks");
});

Deno.test("ChatProviderFactory.create - google with key", () => {
  const config: ProviderConfig = {
    provider: "google",
    model: "gemini-pro",
    api_key: "test-key",
  };
  const provider = ChatProviderFactory.create(config);
  assertEquals(provider.name, "google");
});

Deno.test("ChatProviderFactory.create - google requires key", () => {
  const config: ProviderConfig = {
    provider: "google",
    model: "gemini-pro",
  };
  assertThrows(
    () => ChatProviderFactory.create(config),
    Error,
    "requires an API key",
  );
});

Deno.test("ChatProviderFactory.create - openai requires key", () => {
  const config: ProviderConfig = {
    provider: "openai",
    model: "gpt-4",
  };
  assertThrows(
    () => ChatProviderFactory.create(config),
    Error,
    "requires an API key",
  );
});

Deno.test("ChatProviderFactory.create - custom provider rejected", () => {
  const config: ProviderConfig = {
    provider: "custom",
    model: "custom-model",
    api_key: "key",
  };
  assertThrows(
    () => ChatProviderFactory.create(config),
    Error,
    "is not a chat provider",
  );
});

Deno.test("ChatProviderFactory.create - ollama with custom url", () => {
  const config: ProviderConfig = {
    provider: "ollama",
    model: "llama3.1",
    base_url: "http://custom:8080",
  };
  const provider = ChatProviderFactory.create(config);
  assertEquals(provider.name, "ollama");
});
