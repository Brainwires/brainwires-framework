/**
 * Tests for types.ts — mirrors Rust tests in types.rs
 */
import { assertEquals, assert, assertNotEquals } from "@std/assert";
import {
  AgentCapabilities,
  PathPattern,
  parseCapabilityProfile,
} from "./types.ts";

// ── PathPattern ─────────────────────────────────────────────────────

Deno.test("PathPattern - matches .env files", () => {
  const pattern = new PathPattern("**/.env*");
  assert(pattern.matches(".env"));
  assert(pattern.matches(".env.local"));
  assert(pattern.matches("config/.env"));
});

Deno.test("PathPattern - matches src/**/*.rs", () => {
  const pattern = new PathPattern("src/**/*.rs");
  assert(pattern.matches("src/main.rs"));
  assert(pattern.matches("src/lib/mod.rs"));
});

Deno.test("PathPattern - **/* matches root and nested files", () => {
  const pattern = new PathPattern("**/*");
  assert(pattern.matches("index.html"), "**/* should match root files");
  assert(pattern.matches("src/main.rs"), "**/* should match nested files");
});

// ── Tool Categorization ─────────────────────────────────────────────

Deno.test("categorizeTool - basic categories", () => {
  assertEquals(AgentCapabilities.categorizeTool("read_file"), "FileRead");
  assertEquals(AgentCapabilities.categorizeTool("write_file"), "FileWrite");
  assertEquals(AgentCapabilities.categorizeTool("git_status"), "Git");
  assertEquals(AgentCapabilities.categorizeTool("git_force_push"), "GitDestructive");
  assertEquals(AgentCapabilities.categorizeTool("execute_command"), "Bash");
});

// ── allows_tool ─────────────────────────────────────────────────────

Deno.test("allowsTool - default capabilities", () => {
  const caps = new AgentCapabilities();
  assert(caps.allowsTool("read_file"));
  assert(caps.allowsTool("search_code"));
  assert(!caps.allowsTool("write_file"));
  assert(!caps.allowsTool("execute_command"));
});

Deno.test("allowsTool - denied tools override category", () => {
  const caps = new AgentCapabilities();
  caps.tools.denied_tools.add("read_file");
  assert(!caps.allowsTool("read_file"));
  assert(caps.allowsTool("list_directory")); // other FileRead tools still work
});

// ── Domain matching ─────────────────────────────────────────────────

Deno.test("allowsDomain - wildcard matching", () => {
  const caps = new AgentCapabilities({
    network: {
      allowed_domains: ["github.com", "*.github.com"],
      denied_domains: [],
      allow_all: false,
      rate_limit: undefined,
      allow_api_calls: false,
      max_response_size: undefined,
    },
  });
  assert(caps.allowsDomain("github.com"));
  assert(caps.allowsDomain("api.github.com"));
  assert(caps.allowsDomain("raw.github.com"));
  assert(!caps.allowsDomain("gitlab.com"));
});

// ── Git operations ──────────────────────────────────────────────────

Deno.test("allowsGitOp - default allows read-only ops", () => {
  const caps = new AgentCapabilities();
  assert(caps.allowsGitOp("Status"));
  assert(caps.allowsGitOp("Diff"));
  assert(!caps.allowsGitOp("Push"));
  assert(!caps.allowsGitOp("ForcePush"));
});

// ── Profiles ────────────────────────────────────────────────────────

Deno.test("read_only profile", () => {
  const caps = AgentCapabilities.readOnly();
  assert(caps.allowsTool("read_file"));
  assert(caps.allowsTool("search_code"));
  assert(!caps.allowsTool("write_file"));
  assert(!caps.allowsTool("execute_command"));
  assert(!caps.allowsDomain("github.com"));
  assert(!caps.canSpawnAgent(0, 0));
});

Deno.test("standard_dev profile", () => {
  const caps = AgentCapabilities.standardDev();
  assert(caps.allowsTool("read_file"));
  assert(caps.allowsTool("write_file"));
  assert(caps.allowsTool("git_status"));
  assert(!caps.allowsTool("execute_code"));
  assert(caps.requiresApproval("delete_file"));
  assert(caps.requiresApproval("execute_command"));
  assert(caps.allowsDomain("github.com"));
  assert(caps.allowsDomain("api.github.com"));
  assert(!caps.allowsDomain("malware.com"));
  assert(caps.canSpawnAgent(0, 0));
  assert(caps.canSpawnAgent(2, 1));
  assert(!caps.canSpawnAgent(3, 0));
  assert(!caps.canSpawnAgent(0, 2));
});

Deno.test("full_access profile", () => {
  const caps = AgentCapabilities.fullAccess();
  assert(caps.allowsTool("read_file"));
  assert(caps.allowsTool("write_file"));
  assert(caps.allowsTool("execute_code"));
  assert(caps.allowsTool("execute_command"));
  assert(caps.allowsDomain("any-domain.com"));
  assert(caps.canSpawnAgent(9, 4));
});

// ── derive_child ────────────────────────────────────────────────────

Deno.test("deriveChild - reduces depth", () => {
  const parent = AgentCapabilities.standardDev();
  const child = parent.deriveChild();
  assertEquals(child.spawning.max_depth, parent.spawning.max_depth - 1);
  assert(!child.spawning.can_elevate);
  assertNotEquals(child.capability_id, parent.capability_id);
});

// ── intersect ───────────────────────────────────────────────────────

Deno.test("intersect - takes most restrictive", () => {
  const full = AgentCapabilities.fullAccess();
  const readOnly = AgentCapabilities.readOnly();
  const intersected = full.intersect(readOnly);
  assert(intersected.allowsTool("read_file"));
  assert(!intersected.allowsTool("write_file"));
  assert(!intersected.canSpawnAgent(0, 0));
});

// ── Profile parsing ─────────────────────────────────────────────────

Deno.test("parseCapabilityProfile", () => {
  assertEquals(parseCapabilityProfile("read_only"), "read_only");
  assertEquals(parseCapabilityProfile("standard_dev"), "standard_dev");
  assertEquals(parseCapabilityProfile("full_access"), "full_access");
  assertEquals(parseCapabilityProfile("invalid"), undefined);
});
