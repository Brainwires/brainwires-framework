/**
 * Tests for MarketAllocator, ThreeStateModel, and WaitQueue.
 */

import { assertEquals, assertThrows } from "@std/assert";

import {
  MarketAllocator,
  createBid,
  effectivePriority,
  bidScore as marketBidScoreFn,
  isAllocated,
  calculateUrgency,
  defaultUrgencyContext,
  type UrgencyContext,
} from "./coordination/market.ts";

import {
  ThreeStateModel,
  ApplicationState,
  OperationState,
  DependencyState,
  createOperationLog,
} from "./coordination/three_state.ts";

import {
  WaitQueue,
  resourceKey,
  fileResourceKey,
} from "./coordination/wait_queue.ts";

// ===========================================================================
// MarketAllocator tests
// ===========================================================================

Deno.test("MarketAllocator: register agent and get budget", () => {
  const allocator = new MarketAllocator();
  allocator.registerAgent("agent-1", 100, 1.0);

  const budget = allocator.getBudget("agent-1");
  assertEquals(budget?.totalBudget, 100);
  assertEquals(budget?.available, 100);
});

Deno.test("MarketAllocator: submit bid", () => {
  const allocator = new MarketAllocator();
  allocator.registerAgent("agent-1", 100, 1.0);

  const bid = { ...createBid("agent-1", "resource-a"), basePriority: 8, urgencyMultiplier: 1.5, maxBid: 20, urgencyReason: "user waiting" };
  allocator.submitBid(bid);

  const status = allocator.marketStatus("resource-a");
  assertEquals(status?.pendingBids, 1);
});

Deno.test("MarketAllocator: allocation picks highest score", () => {
  const allocator = new MarketAllocator({ kind: "free" });
  allocator.registerAgent("agent-1", 100, 1.0);
  allocator.registerAgent("agent-2", 100, 1.0);

  allocator.submitBid({ ...createBid("agent-1", "resource-a"), basePriority: 5 });
  allocator.submitBid({ ...createBid("agent-2", "resource-a"), basePriority: 8 });

  const result = allocator.allocate("resource-a");
  assertEquals(result.kind, "allocated");
  if (isAllocated(result)) {
    assertEquals(result.agentId, "agent-2");
  }
});

Deno.test("MarketAllocator: urgency affects allocation", () => {
  const allocator = new MarketAllocator({ kind: "free" });
  allocator.registerAgent("agent-1", 100, 1.0);
  allocator.registerAgent("agent-2", 100, 1.0);

  // Agent 1: priority 8, urgency 1.0 -> effective 8
  allocator.submitBid({ ...createBid("agent-1", "resource-a"), basePriority: 8, urgencyMultiplier: 1.0 });
  // Agent 2: priority 5, urgency 2.5 -> effective 12.5
  allocator.submitBid({ ...createBid("agent-2", "resource-a"), basePriority: 5, urgencyMultiplier: 2.5, urgencyReason: "deadline approaching" });

  const result = allocator.allocate("resource-a");
  if (isAllocated(result)) {
    assertEquals(result.agentId, "agent-2");
  }
});

Deno.test("MarketAllocator: second price auction", () => {
  const allocator = new MarketAllocator({ kind: "second_price" });
  allocator.registerAgent("agent-1", 100, 1.0);
  allocator.registerAgent("agent-2", 100, 1.0);

  allocator.submitBid({ ...createBid("agent-1", "resource-a"), basePriority: 8, maxBid: 30 });
  allocator.submitBid({ ...createBid("agent-2", "resource-a"), basePriority: 5, maxBid: 20 });

  const result = allocator.allocate("resource-a");
  if (isAllocated(result)) {
    assertEquals(result.price, 20);
  }
});

Deno.test("MarketAllocator: insufficient budget rejects bid", () => {
  const allocator = new MarketAllocator({ kind: "first_price" });
  allocator.registerAgent("agent-1", 10, 1.0);

  assertThrows(
    () => allocator.submitBid({ ...createBid("agent-1", "resource-a"), maxBid: 20 }),
    Error,
    "Insufficient budget",
  );
});

Deno.test("MarketAllocator: release resource", () => {
  const allocator = new MarketAllocator({ kind: "free" });
  allocator.registerAgent("agent-1", 100, 1.0);

  allocator.submitBid(createBid("agent-1", "resource-a"));
  allocator.allocate("resource-a");

  const status1 = allocator.marketStatus("resource-a");
  assertEquals(status1?.currentHolder, "agent-1");

  const released = allocator.release("resource-a", "agent-1");
  assertEquals(released, true);

  const status2 = allocator.marketStatus("resource-a");
  assertEquals(status2?.currentHolder, null);
});

Deno.test("MarketAllocator: cannot allocate held resource", () => {
  const allocator = new MarketAllocator({ kind: "free" });
  allocator.registerAgent("agent-1", 100, 1.0);
  allocator.registerAgent("agent-2", 100, 1.0);

  allocator.submitBid(createBid("agent-1", "resource-a"));
  allocator.allocate("resource-a");

  allocator.submitBid(createBid("agent-2", "resource-a"));
  const result = allocator.allocate("resource-a");
  assertEquals(result.kind, "still_held");
});

Deno.test("MarketAllocator: cancel bid", () => {
  const allocator = new MarketAllocator();
  allocator.registerAgent("agent-1", 100, 1.0);

  allocator.submitBid(createBid("agent-1", "resource-a"));
  assertEquals(allocator.marketStatus("resource-a")?.pendingBids, 1);

  const cancelled = allocator.cancelBid("agent-1", "resource-a");
  assertEquals(cancelled, true);
  assertEquals(allocator.marketStatus("resource-a")?.pendingBids, 0);
});

Deno.test("MarketAllocator: market stats", () => {
  const allocator = new MarketAllocator({ kind: "free" });
  allocator.registerAgent("agent-1", 100, 1.0);
  allocator.registerAgent("agent-2", 100, 1.0);

  for (let i = 0; i < 5; i++) {
    allocator.submitBid(createBid("agent-1", `resource-${i}`));
    allocator.allocate(`resource-${i}`);
  }

  const stats = allocator.getStats();
  assertEquals(stats.registeredAgents, 2);
  assertEquals(stats.totalAllocations, 5);
});

Deno.test("MarketAllocator: bid scoring", () => {
  const bid1 = { ...createBid("agent-1", "resource"), basePriority: 8 };
  const bid2 = { ...createBid("agent-2", "resource"), basePriority: 5 };
  assertEquals(marketBidScoreFn(bid1) > marketBidScoreFn(bid2), true);

  const bid3 = { ...createBid("agent-3", "resource"), basePriority: 5, urgencyMultiplier: 2.0, urgencyReason: "urgent" };
  assertEquals(effectivePriority(bid3) > effectivePriority(bid2), true);
});

Deno.test("UrgencyCalculator: default context", () => {
  const ctx = defaultUrgencyContext();
  const urgency = calculateUrgency(ctx);
  assertEquals(Math.abs(urgency - 1.0) < 0.01, true);
});

Deno.test("UrgencyCalculator: user waiting", () => {
  const ctx: UrgencyContext = { ...defaultUrgencyContext(), userWaiting: true };
  const urgency = calculateUrgency(ctx);
  assertEquals(Math.abs(urgency - 2.0) < 0.01, true);
});

Deno.test("UrgencyCalculator: critical path", () => {
  const ctx: UrgencyContext = { ...defaultUrgencyContext(), criticalPath: true };
  const urgency = calculateUrgency(ctx);
  assertEquals(Math.abs(urgency - 1.5) < 0.01, true);
});

Deno.test("UrgencyCalculator: combined factors", () => {
  const ctx: UrgencyContext = { ...defaultUrgencyContext(), userWaiting: true, criticalPath: true };
  const urgency = calculateUrgency(ctx);
  assertEquals(Math.abs(urgency - 3.0) < 0.01, true);
});

// ===========================================================================
// ThreeStateModel tests
// ===========================================================================

Deno.test("ThreeStateModel: creation and snapshot", () => {
  const model = new ThreeStateModel();
  const snapshot = model.snapshot();
  assertEquals(snapshot.files.size, 0);
  assertEquals(snapshot.locks.size, 0);
  assertEquals(snapshot.activeOperations.length, 0);
});

Deno.test("ApplicationState: file tracking", () => {
  const appState = new ApplicationState();
  appState.updateFile("/test/file.rs", "hash123");
  const files = appState.getAllFiles();
  assertEquals(files.get("/test/file.rs")?.contentHash, "hash123");
  assertEquals(files.get("/test/file.rs")?.dirty, true);
});

Deno.test("OperationState: lifecycle", () => {
  const opState = new OperationState();
  const log = createOperationLog("op-1", "agent-1", "build", {});
  const id = opState.startOperation(log);
  assertEquals(id, "op-1");

  const active = opState.getActiveOperations();
  assertEquals(active.length, 1);

  opState.completeOperation(id, true);
  const activeAfter = opState.getActiveOperations();
  assertEquals(activeAfter.length, 0);

  const op = opState.getOperation(id);
  assertEquals(op?.status, "completed");
});

Deno.test("DependencyState: deadlock detection", () => {
  const depState = new DependencyState();
  depState.addDependency("resource-a", "resource-b", {
    dependencyType: "blocked_by",
    strength: "hard",
  });
  depState.setHolder("resource-a", "agent-1");

  const wouldDeadlock = depState.wouldDeadlock("agent-1", ["resource-b"]);
  // Simple case: no deadlock
  assertEquals(wouldDeadlock, false);
});

Deno.test("ThreeStateModel: validate operation conflict detection", () => {
  const model = new ThreeStateModel();

  const log = {
    ...createOperationLog("op-1", "agent-1", "build", {}),
    resourcesNeeded: ["resource-a"],
    resourcesProduced: [],
  };
  model.operationState.startOperation(log);

  const result = model.validateOperation({
    agentId: "agent-2",
    operationType: "build",
    resourcesNeeded: ["resource-a"],
    resourcesProduced: [],
  });
  assertEquals(result.valid, false);
  assertEquals(result.errors.length > 0, true);
});

Deno.test("ThreeStateModel: record state change", () => {
  const model = new ThreeStateModel();
  model.recordStateChange({
    operationId: "op-1",
    applicationChanges: [
      { kind: "file_modified", path: "/test/file.rs", newHash: "newhash" },
      { kind: "resource_created", resourceId: "build-artifact" },
    ],
    newDependencies: [],
  });

  const snapshot = model.snapshot();
  assertEquals(snapshot.files.has("/test/file.rs"), true);
  assertEquals(model.applicationState.resourceExists("build-artifact"), true);
});

Deno.test("DependencyState: execution order", () => {
  const depState = new DependencyState();
  depState.addDependency("op-a", "op-b", {
    dependencyType: "blocked_by",
    strength: "hard",
  });

  const order = depState.getExecutionOrder(["op-a", "op-b"]);
  const posA = order.indexOf("op-a");
  const posB = order.indexOf("op-b");
  assertEquals(posB < posA, true);
});

// ===========================================================================
// WaitQueue tests
// ===========================================================================

Deno.test("WaitQueue: register and position", () => {
  const queue = new WaitQueue();
  const handle1 = queue.register("build:/project", "agent-1", 5, false);
  const handle2 = queue.register("build:/project", "agent-2", 5, false);

  assertEquals(handle1.initialPosition, 0);
  assertEquals(handle2.initialPosition, 1);
  assertEquals(queue.position("build:/project", "agent-1"), 0);
  assertEquals(queue.position("build:/project", "agent-2"), 1);
  assertEquals(queue.queueLength("build:/project"), 2);
});

Deno.test("WaitQueue: priority ordering", () => {
  const queue = new WaitQueue();
  queue.register("build:/project", "agent-1", 5, false);
  const handle2 = queue.register("build:/project", "agent-2", 1, false); // Higher priority
  queue.register("build:/project", "agent-3", 10, false); // Lower priority

  assertEquals(handle2.initialPosition, 0);
  assertEquals(queue.position("build:/project", "agent-2"), 0);
  assertEquals(queue.position("build:/project", "agent-1"), 1);
  assertEquals(queue.position("build:/project", "agent-3"), 2);
});

Deno.test("WaitQueue: cancel", () => {
  const queue = new WaitQueue();
  queue.register("build:/project", "agent-1", 5, false);
  queue.register("build:/project", "agent-2", 5, false);

  assertEquals(queue.cancel("build:/project", "agent-1"), true);
  assertEquals(queue.position("build:/project", "agent-1"), null);
  assertEquals(queue.position("build:/project", "agent-2"), 0);
  assertEquals(queue.queueLength("build:/project"), 1);
});

Deno.test("WaitQueue: notify released", () => {
  const queue = new WaitQueue();
  queue.register("build:/project", "agent-1", 5, false);
  queue.register("build:/project", "agent-2", 5, false);

  const next = queue.notifyReleased("build:/project");
  assertEquals(next, "agent-1");
  assertEquals(queue.position("build:/project", "agent-2"), 0);
  assertEquals(queue.queueLength("build:/project"), 1);
});

Deno.test("WaitQueue: empty queue cleanup", () => {
  const queue = new WaitQueue();
  queue.register("build:/project", "agent-1", 5, false);
  assertEquals(queue.cancel("build:/project", "agent-1"), true);
  assertEquals(queue.queueLength("build:/project"), 0);
  assertEquals(queue.listQueues().length, 0);
});

Deno.test("WaitQueue: wait time estimation", () => {
  const queue = new WaitQueue();
  queue.recordWaitTime("build:/project", 10_000);
  queue.recordWaitTime("build:/project", 20_000);
  queue.recordWaitTime("build:/project", 30_000);

  const estimate = queue.estimateWait("build:/project");
  assertEquals(estimate, 20_000); // Average of 10k, 20k, 30k
});

Deno.test("WaitQueue: isWaiting", () => {
  const queue = new WaitQueue();
  queue.register("build:/project", "agent-1", 5, false);
  assertEquals(queue.isWaiting("agent-1"), true);
  assertEquals(queue.isWaiting("agent-2"), false);
});

Deno.test("WaitQueue: waitingFor", () => {
  const queue = new WaitQueue();
  queue.register("build:/project1", "agent-1", 5, false);
  queue.register("build:/project2", "agent-1", 5, false);

  const waiting = queue.waitingFor("agent-1");
  assertEquals(waiting.length, 2);
  assertEquals(waiting.includes("build:/project1"), true);
  assertEquals(waiting.includes("build:/project2"), true);
});

Deno.test("WaitQueue: peek next", () => {
  const queue = new WaitQueue();
  queue.register("build:/project", "agent-1", 5, true);

  const next = queue.peekNext("build:/project");
  assertEquals(next?.agentId, "agent-1");
  assertEquals(next?.priority, 5);
  assertEquals(next?.autoAcquire, true);
  assertEquals(queue.queueLength("build:/project"), 1); // Still in queue
});

Deno.test("WaitQueue: should auto acquire", () => {
  const queue = new WaitQueue();
  queue.register("build:/project", "agent-1", 5, true);
  queue.register("build:/project", "agent-2", 5, false);

  assertEquals(queue.shouldAutoAcquire("build:/project", "agent-1"), true);
  assertEquals(queue.shouldAutoAcquire("build:/project", "agent-2"), false);
});

Deno.test("WaitQueue: queue status", () => {
  const queue = new WaitQueue();
  queue.register("build:/project", "agent-1", 5, false);
  queue.register("build:/project", "agent-2", 3, true);

  const status = queue.getQueueStatus("build:/project");
  assertEquals(status?.queueLength, 2);
  assertEquals(status?.waiters.length, 2);
  // agent-2 should be first (priority 3 < 5)
  assertEquals(status?.waiters[0].agentId, "agent-2");
  assertEquals(status?.waiters[1].agentId, "agent-1");
});

Deno.test("WaitQueue: event subscription", () => {
  const queue = new WaitQueue();
  const events: string[] = [];
  queue.subscribe((e) => events.push(e.type));

  queue.register("build:/project", "agent-1", 5, false);
  assertEquals(events.includes("registered"), true);
});

Deno.test("resourceKey and fileResourceKey helpers", () => {
  assertEquals(resourceKey("build", "/project"), "build:/project");
  assertEquals(fileResourceKey("/src/main.rs"), "file:/src/main.rs");
});
