import { assertEquals } from "jsr:@std/assert@^1.0.0";
import { allow, reject, type PreHookDecision } from "./executor.ts";

Deno.test("PreHookDecision - allow", () => {
  const decision: PreHookDecision = allow();
  assertEquals(decision.type, "Allow");
});

Deno.test("PreHookDecision - reject", () => {
  const decision: PreHookDecision = reject("Not allowed");
  assertEquals(decision.type, "Reject");
  if (decision.type === "Reject") {
    assertEquals(decision.reason, "Not allowed");
  }
});
