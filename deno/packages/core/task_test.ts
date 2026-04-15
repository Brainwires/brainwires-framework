import { assertEquals } from "@std/assert";
import { Task } from "./mod.ts";

Deno.test("Task lifecycle", () => {
  const task = new Task("task-1", "Test task");
  assertEquals(task.status, "pending");
  task.start();
  assertEquals(task.status, "inprogress");
  task.complete("Done!");
  assertEquals(task.status, "completed");
});

Deno.test("Task failure", () => {
  const task = new Task("task-2", "Failing task");
  task.start();
  task.fail("Error occurred");
  assertEquals(task.status, "failed");
});

Deno.test("Task.newForPlan", () => {
  const task = Task.newForPlan("t-1", "desc", "plan-1");
  assertEquals(task.plan_id, "plan-1");
});

Deno.test("Task.newSubtask", () => {
  const task = Task.newSubtask("t-2", "desc", "parent-1");
  assertEquals(task.parent_id, "parent-1");
  assertEquals(task.isRoot(), false);
});

Deno.test("Task dependencies", () => {
  const task = new Task("t-3", "desc");
  assertEquals(task.hasDependencies(), false);
  task.addDependency("other");
  assertEquals(task.hasDependencies(), true);
});

Deno.test("Task children", () => {
  const task = new Task("t-4", "desc");
  assertEquals(task.hasChildren(), false);
  task.addChild("child-1");
  assertEquals(task.hasChildren(), true);
  // Adding same child is a no-op
  task.addChild("child-1");
  assertEquals(task.children.length, 1);
});
