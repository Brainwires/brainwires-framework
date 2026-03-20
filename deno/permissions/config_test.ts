/**
 * Tests for config.ts — mirrors Rust tests in config.rs
 */
import { assertEquals, assert } from "jsr:@std/assert";
import {
  parseSize,
  parseDuration,
  parseToolCategory,
  parseGitOperation,
  configToCapabilities,
} from "./config.ts";
import type { PermissionsConfig } from "./config.ts";

Deno.test("parseSize", () => {
  assertEquals(parseSize("1MB"), 1024 * 1024);
  assertEquals(parseSize("512KB"), 512 * 1024);
  assertEquals(parseSize("1GB"), 1024 * 1024 * 1024);
  assertEquals(parseSize("100B"), 100);
  assertEquals(parseSize("100"), 100);
});

Deno.test("parseDuration", () => {
  assertEquals(parseDuration("30m"), 30 * 60);
  assertEquals(parseDuration("1h"), 3600);
  assertEquals(parseDuration("90s"), 90);
  assertEquals(parseDuration("120"), 120);
});

Deno.test("parseToolCategory", () => {
  assertEquals(parseToolCategory("FileRead"), "FileRead");
  assertEquals(parseToolCategory("file_read"), "FileRead");
  assertEquals(parseToolCategory("Git"), "Git");
  assertEquals(parseToolCategory("invalid"), undefined);
});

Deno.test("parseGitOperation", () => {
  assertEquals(parseGitOperation("Status"), "Status");
  assertEquals(parseGitOperation("push"), "Push");
  assertEquals(parseGitOperation("ForcePush"), "ForcePush");
  assertEquals(parseGitOperation("force_push"), "ForcePush");
});

Deno.test("config to capabilities - default", () => {
  const config: PermissionsConfig = {};
  const caps = configToCapabilities(config);
  // Should be standard_dev by default
  assert(caps.allowsTool("read_file"));
  assert(caps.allowsTool("write_file"));
});
