/**
 * Tests for policy.ts — mirrors Rust tests in policy.rs
 */
import { assertEquals, assert } from "jsr:@std/assert";
import {
  PolicyEngine,
  createPolicy,
  PolicyActions,
  policyMatches,
  policyRequestForTool,
  policyRequestForFile,
  policyRequestForNetwork,
  policyRequestForGit,
  createPolicyRequest,
  isDecisionAllowed,
  isDecisionRequiresApproval,
  type PolicyCondition,
} from "./policy.ts";

Deno.test("policy matching - tool condition", () => {
  const policy = createPolicy("test", {
    conditions: [{ type: "tool", name: "write_file" }],
    action: PolicyActions.Deny,
  });
  assert(policyMatches(policy, policyRequestForTool("write_file")));
  assert(!policyMatches(policy, policyRequestForTool("read_file")));
});

Deno.test("file path condition", () => {
  const policy = createPolicy("test", {
    conditions: [{ type: "file_path", pattern: "**/.env*" }],
    action: PolicyActions.Deny,
  });
  assert(policyMatches(policy, policyRequestForFile(".env", "read_file")));
  assert(policyMatches(policy, policyRequestForFile(".env.local", "read_file")));
  assert(!policyMatches(policy, policyRequestForFile("src/main.rs", "read_file")));
});

Deno.test("domain condition", () => {
  const cond: PolicyCondition = { type: "domain", pattern: "*.github.com" };
  const policy = createPolicy("test", { conditions: [cond], action: PolicyActions.Deny });
  assert(policyMatches(policy, policyRequestForNetwork("api.github.com")));
  assert(policyMatches(policy, policyRequestForNetwork("github.com")));
  assert(!policyMatches(policy, policyRequestForNetwork("evil.com")));
});

Deno.test("compound AND conditions", () => {
  const cond: PolicyCondition = {
    type: "and",
    conditions: [
      { type: "tool_category", category: "FileWrite" },
      { type: "file_path", pattern: "**/test/**" },
    ],
  };
  const policy = createPolicy("test", { conditions: [cond], action: PolicyActions.Deny });

  const request = {
    ...policyRequestForFile("src/test/file.rs", "write_file"),
    tool_category: "FileWrite" as const,
  };
  assert(policyMatches(policy, request));

  const request2 = {
    ...policyRequestForFile("src/main.rs", "write_file"),
    tool_category: "FileWrite" as const,
  };
  assert(!policyMatches(policy, request2));
});

Deno.test("policy engine evaluation", () => {
  const engine = new PolicyEngine();

  engine.addPolicy(createPolicy("deny_secrets", {
    conditions: [{ type: "file_path", pattern: "**/.env*" }],
    action: PolicyActions.Deny,
    priority: 100,
  }));

  engine.addPolicy(createPolicy("allow_read", {
    conditions: [{ type: "tool_category", category: "FileRead" }],
    action: PolicyActions.Allow,
    priority: 10,
  }));

  // Should be denied (higher priority)
  const decision = engine.evaluate(policyRequestForFile(".env", "read_file"));
  assert(!isDecisionAllowed(decision));
  assertEquals(decision.matched_policy, "deny_secrets");

  // Should be allowed
  const request2 = { ...policyRequestForFile("src/main.rs", "read_file"), tool_category: "FileRead" as const };
  const decision2 = engine.evaluate(request2);
  assert(isDecisionAllowed(decision2));
});

Deno.test("trust level condition", () => {
  const policy = createPolicy("require_trust", {
    conditions: [{ type: "min_trust_level", level: 2 }],
    action: PolicyActions.Allow,
  });
  assert(!policyMatches(policy, createPolicyRequest({ trust_level: 1 })));
  assert(policyMatches(policy, createPolicyRequest({ trust_level: 3 })));
});

Deno.test("git operation condition", () => {
  const policy = createPolicy("approve_reset", {
    conditions: [{ type: "git_op", operation: "Reset" }],
    action: PolicyActions.RequireApproval,
  });
  assert(policyMatches(policy, policyRequestForGit("Reset")));
  assert(!policyMatches(policy, policyRequestForGit("Commit")));
});

Deno.test("default policies", () => {
  const engine = PolicyEngine.withDefaults();

  const decision = engine.evaluate(policyRequestForFile(".env", "read_file"));
  assert(!isDecisionAllowed(decision));

  const decision2 = engine.evaluate(policyRequestForGit("Reset"));
  assert(isDecisionRequiresApproval(decision2));
});

Deno.test("NOT condition", () => {
  const cond: PolicyCondition = { type: "not", condition: { type: "tool", name: "read_file" } };
  const policy = createPolicy("test", { conditions: [cond], action: PolicyActions.Deny });
  assert(policyMatches(policy, policyRequestForTool("write_file")));
  assert(!policyMatches(policy, policyRequestForTool("read_file")));
});

Deno.test("OR condition", () => {
  const cond: PolicyCondition = {
    type: "or",
    conditions: [
      { type: "tool", name: "write_file" },
      { type: "tool", name: "delete_file" },
    ],
  };
  const policy = createPolicy("test", { conditions: [cond], action: PolicyActions.Deny });
  assert(policyMatches(policy, policyRequestForTool("write_file")));
  assert(policyMatches(policy, policyRequestForTool("delete_file")));
  assert(!policyMatches(policy, policyRequestForTool("read_file")));
});
