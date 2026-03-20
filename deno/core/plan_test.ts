import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { PlanBudget, PlanMetadata, SerializablePlan } from "./mod.ts";

Deno.test("PlanMetadata new", () => {
  const plan = new PlanMetadata("conv-123", "Implement auth", "Step 1");
  assertEquals(plan.plan_id.length > 0, true);
  assertEquals(plan.status, "draft");
  assertEquals(plan.isRoot(), true);
});

Deno.test("Plan branching", () => {
  const parent = new PlanMetadata("conv-123", "Main", "Plan");
  const branch = parent.createBranch("feature-x", "Feature X", "Branch plan");
  assertEquals(branch.parent_plan_id, parent.plan_id);
  assertEquals(branch.depth, 1);
  assertEquals(branch.isRoot(), false);
});

Deno.test("PlanBudget check - no limits", () => {
  const budget = new PlanBudget();
  const plan = new SerializablePlan("task", [
    { step_number: 1, description: "do thing", estimated_tokens: 9_000_000 },
  ]);
  assertEquals(budget.check(plan), undefined);
});

Deno.test("PlanBudget check - step limit exceeded", () => {
  const budget = new PlanBudget().withMaxSteps(2);
  const steps = [1, 2, 3].map((i) => ({
    step_number: i,
    description: `step ${i}`,
    estimated_tokens: 100,
  }));
  const plan = new SerializablePlan("task", steps);
  const result = budget.check(plan);
  assertEquals(typeof result, "string");
  assertEquals(result!.includes("3 steps"), true);
});

Deno.test("PlanBudget check - token limit exceeded", () => {
  const budget = new PlanBudget().withMaxTokens(500);
  const steps = [1, 2, 3].map((i) => ({
    step_number: i,
    description: `step ${i}`,
    estimated_tokens: 300,
  }));
  const plan = new SerializablePlan("task", steps);
  const result = budget.check(plan);
  assertEquals(typeof result, "string");
  assertEquals(result!.includes("900 tokens"), true);
});

Deno.test("SerializablePlan.parseFromText", () => {
  const text = `Here is my plan:
{"steps":[{"description":"Read the file","tool":"read_file","estimated_tokens":300},{"description":"Write changes","tool":"write_file","estimated_tokens":500}]}
That's the plan.`;
  const plan = SerializablePlan.parseFromText("task", text);
  assertEquals(plan !== undefined, true);
  assertEquals(plan!.steps.length, 2);
  assertEquals(plan!.steps[0].step_number, 1);
  assertEquals(plan!.steps[0].description, "Read the file");
  assertEquals(plan!.steps[0].tool_hint, "read_file");
  assertEquals(plan!.totalEstimatedTokens(), 800);
});

Deno.test("SerializablePlan.parseFromText - empty steps returns undefined", () => {
  assertEquals(SerializablePlan.parseFromText("task", '{"steps":[]}'), undefined);
});

Deno.test("SerializablePlan.parseFromText - no JSON returns undefined", () => {
  assertEquals(SerializablePlan.parseFromText("task", "no json here"), undefined);
});
