import { assertEquals, assertThrows } from "@std/assert";

import {
  bidScore,
  ContractNetManager,
  ContractParticipant,
  type TaskAnnouncement,
  type TaskBid,
  defaultTaskRequirements,
} from "./coordination/contract_net.ts";

import {
  CompensationReport,
  isCompensable,
  SagaExecutor,
  type CompensableOperation,
  type OperationResult,
  successResult,
} from "./coordination/saga.ts";

import {
  isCommitSuccess,
  isTokenStale,
  OptimisticController,
} from "./coordination/optimistic.ts";

// ===========================================================================
// Contract-Net Tests
// ===========================================================================

Deno.test("contract-net: bid scoring", () => {
  const bid: TaskBid = {
    agentId: "agent-1",
    taskId: "task-1",
    capabilityScore: 0.8,
    currentLoad: 0.2,
    estimatedDurationMs: 120_000,
    conditions: [],
    submittedAt: Date.now(),
  };

  const score = bidScore(bid);
  assertEquals(score > 0 && score <= 1, true);

  const highCapBid: TaskBid = {
    ...bid,
    agentId: "agent-2",
    capabilityScore: 1.0,
  };
  assertEquals(bidScore(highCapBid) > score, true);
});

Deno.test("contract-net: announce and bid", () => {
  const manager = new ContractNetManager();

  const announcement: TaskAnnouncement = {
    taskId: "",
    description: "Test task",
    requirements: defaultTaskRequirements(),
    bidDeadline: Date.now() + 60_000,
    announcer: "manager",
    announcedAt: Date.now(),
  };
  const taskId = manager.announceTask(announcement);
  assertEquals(taskId.length > 0, true);

  const bid: TaskBid = {
    agentId: "agent-1",
    taskId,
    capabilityScore: 0.9,
    currentLoad: 0.1,
    estimatedDurationMs: 60_000,
    conditions: [],
    submittedAt: Date.now(),
  };
  manager.receiveBid(bid);

  const bids = manager.getBids(taskId);
  assertEquals(bids.length, 1);
  assertEquals(bids[0].agentId, "agent-1");
});

Deno.test("contract-net: award task", () => {
  const manager = new ContractNetManager();

  const announcement: TaskAnnouncement = {
    taskId: "task-1",
    description: "Test task",
    requirements: defaultTaskRequirements(),
    bidDeadline: Date.now() + 60_000,
    announcer: "manager",
    announcedAt: Date.now(),
  };
  manager.announceTask(announcement);

  manager.receiveBid({
    agentId: "agent-1",
    taskId: "task-1",
    capabilityScore: 0.7,
    currentLoad: 0.0,
    estimatedDurationMs: 60_000,
    conditions: [],
    submittedAt: Date.now(),
  });
  manager.receiveBid({
    agentId: "agent-2",
    taskId: "task-1",
    capabilityScore: 0.9,
    currentLoad: 0.0,
    estimatedDurationMs: 60_000,
    conditions: [],
    submittedAt: Date.now(),
  });

  const winner = manager.awardTask("task-1");
  assertEquals(winner, "agent-2"); // Higher capability wins
});

Deno.test("contract-net: task lifecycle", () => {
  const manager = new ContractNetManager();

  const announcement: TaskAnnouncement = {
    taskId: "task-1",
    description: "Test task",
    requirements: defaultTaskRequirements(),
    bidDeadline: Date.now() + 60_000,
    announcer: "manager",
    announcedAt: Date.now(),
  };
  manager.announceTask(announcement);
  assertEquals(manager.getTaskStatus("task-1"), "open_for_bids");

  manager.receiveBid({
    agentId: "agent-1",
    taskId: "task-1",
    capabilityScore: 0.9,
    currentLoad: 0.0,
    estimatedDurationMs: 60_000,
    conditions: [],
    submittedAt: Date.now(),
  });

  manager.awardTask("task-1");
  assertEquals(manager.getTaskStatus("task-1"), "awarded");

  manager.acceptAward("task-1", "agent-1");
  assertEquals(manager.getTaskStatus("task-1"), "in_progress");

  manager.completeTask("task-1", "agent-1", true, "Done");
  assertEquals(manager.getTaskStatus("task-1"), "completed");
});

Deno.test("contract-net: bid after deadline", () => {
  const manager = new ContractNetManager();

  const announcement: TaskAnnouncement = {
    taskId: "task-1",
    description: "Test task",
    requirements: defaultTaskRequirements(),
    bidDeadline: Date.now() - 1000, // Already past
    announcer: "manager",
    announcedAt: Date.now(),
  };
  manager.announceTask(announcement);

  assertThrows(() =>
    manager.receiveBid({
      agentId: "agent-1",
      taskId: "task-1",
      capabilityScore: 0.9,
      currentLoad: 0.0,
      estimatedDurationMs: 60_000,
      conditions: [],
      submittedAt: Date.now(),
    })
  );
});

Deno.test("contract-net: participant", () => {
  const participant = new ContractParticipant(
    "agent-1",
    ["rust", "git"],
    2,
  );

  const announcement: TaskAnnouncement = {
    taskId: "task-1",
    description: "Test task",
    requirements: {
      ...defaultTaskRequirements(),
      capabilities: ["rust"],
      complexity: 5,
    },
    bidDeadline: Date.now() + 60_000,
    announcer: "manager",
    announcedAt: Date.now(),
  };

  assertEquals(participant.shouldBid(announcement), true);

  const bid = participant.generateBid(announcement);
  assertEquals(bid.agentId, "agent-1");
  assertEquals(bid.capabilityScore, 1.0);

  participant.acceptTask("task-1");
  assertEquals(participant.currentTaskCount(), 1);

  participant.completeTask("task-1");
  assertEquals(participant.currentTaskCount(), 0);
});

// ===========================================================================
// Saga Tests
// ===========================================================================

class NoOpOp implements CompensableOperation {
  constructor(
    private desc: string,
    private opType: "generic" | "file_write" = "generic",
  ) {}
  // deno-lint-ignore require-await
  async execute(): Promise<OperationResult> {
    return successResult(this.desc);
  }
  async compensate(_result: OperationResult): Promise<void> {}
  description(): string {
    return this.desc;
  }
  operationType() {
    // deno-lint-ignore no-explicit-any
    return this.opType as any;
  }
}

// deno-lint-ignore require-await
Deno.test("saga: basic execution", async () => {
  const saga = new SagaExecutor("test-agent", "test saga");
  assertEquals(saga.status, "running");
  assertEquals(saga.operationCount(), 0);
});

Deno.test("saga: execute and complete", async () => {
  const saga = new SagaExecutor("test-agent", "test saga");
  const result = await saga.executeStep(new NoOpOp("test op"));
  assertEquals(result.success, true);

  saga.complete();
  assertEquals(saga.status, "completed");
});

Deno.test("saga: compensation", async () => {
  const saga = new SagaExecutor("test-agent", "test saga");
  await saga.executeStep(new NoOpOp("compensable op", "file_write"));

  saga.fail();
  const report = await saga.compensateAll();

  assertEquals(saga.status, "compensated");
  assertEquals(report.operations.length, 1);
});

Deno.test("saga: operation type compensable", () => {
  assertEquals(isCompensable("file_write"), true);
  assertEquals(isCompensable("file_edit"), true);
  assertEquals(isCompensable("git_stage"), true);
  assertEquals(isCompensable("git_commit"), true);

  assertEquals(isCompensable("build"), false);
  assertEquals(isCompensable("test"), false);
  assertEquals(isCompensable("generic"), false);
});

Deno.test("saga: compensation report", () => {
  const report = new CompensationReport("test-saga");
  report.addSuccess("op1");
  report.addFailure("op2", "error");
  report.addSkipped("op3", "non-compensable");

  assertEquals(report.allSuccessful(), false);
  assertEquals(report.summary().includes("1 successful"), true);
  assertEquals(report.summary().includes("1 failed"), true);
  assertEquals(report.summary().includes("1 skipped"), true);
});

// ===========================================================================
// Optimistic Concurrency Tests
// ===========================================================================

Deno.test("optimistic: commit success", () => {
  const controller = new OptimisticController();
  const token = controller.beginOptimistic("agent-1", "file.txt");
  assertEquals(token.baseVersion, 0);

  const version = controller.commitOptimistic(token, "hash123");
  assertEquals(version, 1);
});

Deno.test("optimistic: commit conflict", () => {
  const controller = new OptimisticController();
  const token1 = controller.beginOptimistic("agent-1", "file.txt");
  const token2 = controller.beginOptimistic("agent-2", "file.txt");

  controller.commitOptimistic(token1, "hash1");

  try {
    controller.commitOptimistic(token2, "hash2");
    throw new Error("Should have thrown");
  // deno-lint-ignore no-explicit-any
  } catch (e: any) {
    assertEquals(e.expectedVersion, 0);
    assertEquals(e.actualVersion, 1);
    assertEquals(e.holderAgent, "agent-1");
  }
});

Deno.test("optimistic: version tracking", () => {
  const controller = new OptimisticController();

  const token1 = controller.beginOptimistic("agent-1", "file.txt");
  controller.commitOptimistic(token1, "hash1");

  const token2 = controller.beginOptimistic("agent-1", "file.txt");
  assertEquals(token2.baseVersion, 1);
  controller.commitOptimistic(token2, "hash2");

  const version = controller.getVersion("file.txt");
  assertEquals(version?.version, 2);
  assertEquals(version?.contentHash, "hash2");
});

Deno.test("optimistic: last writer wins", () => {
  const controller = new OptimisticController({
    kind: "last_writer_wins",
  });

  const token1 = controller.beginOptimistic("agent-1", "file.txt");
  const token2 = controller.beginOptimistic("agent-2", "file.txt");

  controller.commitOptimistic(token1, "hash1");

  const result = controller.commitOrResolve(token2, "hash2");
  assertEquals(isCommitSuccess(result), true);
});

Deno.test("optimistic: first writer wins", () => {
  const controller = new OptimisticController({
    kind: "first_writer_wins",
  });

  const token1 = controller.beginOptimistic("agent-1", "file.txt");
  const token2 = controller.beginOptimistic("agent-2", "file.txt");

  controller.commitOptimistic(token1, "hash1");

  const result = controller.commitOrResolve(token2, "hash2");
  assertEquals(result.kind, "rejected");
});

Deno.test("optimistic: has changed", () => {
  const controller = new OptimisticController();

  assertEquals(controller.hasChanged("file.txt", 0), false);

  const token = controller.beginOptimistic("agent-1", "file.txt");
  controller.commitOptimistic(token, "hash1");

  assertEquals(controller.hasChanged("file.txt", 0), true);
  assertEquals(controller.hasChanged("file.txt", 1), false);
});

Deno.test("optimistic: conflict history", () => {
  const controller = new OptimisticController();

  const token1 = controller.beginOptimistic("agent-1", "file.txt");
  const token2 = controller.beginOptimistic("agent-2", "file.txt");

  controller.commitOptimistic(token1, "hash1");
  controller.commitOrResolve(token2, "hash2");

  const history = controller.getConflictHistory();
  assertEquals(history.length, 1);
  assertEquals(history[0].conflict.conflictingAgent, "agent-2");
});

Deno.test("optimistic: stats", () => {
  const controller = new OptimisticController();

  for (let i = 0; i < 5; i++) {
    const token = controller.beginOptimistic("agent-1", `file${i}.txt`);
    controller.commitOptimistic(token, `hash${i}`);
  }

  const stats = controller.getStats();
  assertEquals(stats.totalResources, 5);
  assertEquals(stats.totalConflicts, 0);
});

Deno.test("optimistic: token staleness", () => {
  const token = {
    resourceId: "test",
    baseVersion: 0,
    baseHash: "",
    agentId: "agent-1",
    createdAt: Date.now() - 120_000,
  };

  assertEquals(isTokenStale(token, 60_000), true);
  assertEquals(isTokenStale(token, 180_000), false);
});
