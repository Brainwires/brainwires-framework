import { assertEquals } from "@std/assert";
import { canOverride, requiresSanitization } from "./mod.ts";

Deno.test("requiresSanitization only for external", () => {
  assertEquals(requiresSanitization("system_prompt"), false);
  assertEquals(requiresSanitization("user_input"), false);
  assertEquals(requiresSanitization("agent_reasoning"), false);
  assertEquals(requiresSanitization("external_content"), true);
});

Deno.test("canOverride respects trust order", () => {
  assertEquals(canOverride("system_prompt", "user_input"), true);
  assertEquals(canOverride("system_prompt", "external_content"), true);
  assertEquals(canOverride("external_content", "system_prompt"), false);
  assertEquals(canOverride("user_input", "user_input"), false);
});
