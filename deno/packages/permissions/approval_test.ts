/**
 * Tests for approval.ts — mirrors Rust tests in approval.rs
 */
import { assertEquals, assert } from "@std/assert";
import {
  approvalActionDescription,
  approvalActionCategory,
  approvalActionSeverity,
  isApprovalResponseApproved,
  isApprovalResponseSessionPersistent,
  type ApprovalAction,
} from "./approval.ts";

Deno.test("approval action description", () => {
  const action: ApprovalAction = { type: "write_file", path: "/tmp/test.txt" };
  assert(approvalActionDescription(action).includes("/tmp/test.txt"));
  assertEquals(approvalActionCategory(action), "File Write");
});

Deno.test("approval response is_approved", () => {
  assert(isApprovalResponseApproved("approve"));
  assert(isApprovalResponseApproved("approve_for_session"));
  assert(!isApprovalResponseApproved("deny"));
  assert(!isApprovalResponseApproved("deny_for_session"));
});

Deno.test("approval response is_session_persistent", () => {
  assert(!isApprovalResponseSessionPersistent("approve"));
  assert(isApprovalResponseSessionPersistent("approve_for_session"));
  assert(!isApprovalResponseSessionPersistent("deny"));
  assert(isApprovalResponseSessionPersistent("deny_for_session"));
});

Deno.test("command truncation", () => {
  const longCommand = "a".repeat(100);
  const action: ApprovalAction = { type: "execute_command", command: longCommand };
  const desc = approvalActionDescription(action);
  assert(desc.length < 70);
  assert(desc.endsWith("..."));
});

Deno.test("severity levels", () => {
  assertEquals(approvalActionSeverity({ type: "delete_file", path: "x" }), "high");
  assertEquals(approvalActionSeverity({ type: "write_file", path: "x" }), "medium");
  assertEquals(approvalActionSeverity({ type: "create_directory", path: "x" }), "low");
});
