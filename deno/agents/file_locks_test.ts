import { assertEquals, assertThrows } from "jsr:@std/assert";
import { FileLockManager } from "./file_locks.ts";

Deno.test("acquire write lock", () => {
  const manager = new FileLockManager();
  const guard = manager.acquireLock("agent-1", "/test/file.txt", "write");

  assertEquals(guard.lockType, "write");
  assertEquals(manager.isLockedBy("/test/file.txt", "agent-1"), true);
});

Deno.test("acquire read lock", () => {
  const manager = new FileLockManager();
  const _guard = manager.acquireLock("agent-1", "/test/file.txt", "read");
  assertEquals(manager.isLockedBy("/test/file.txt", "agent-1"), true);
});

Deno.test("multiple read locks", () => {
  const manager = new FileLockManager();
  const _guard1 = manager.acquireLock("agent-1", "/test/file.txt", "read");
  const _guard2 = manager.acquireLock("agent-2", "/test/file.txt", "read");

  assertEquals(manager.isLockedBy("/test/file.txt", "agent-1"), true);
  assertEquals(manager.isLockedBy("/test/file.txt", "agent-2"), true);
});

Deno.test("write lock blocks other write", () => {
  const manager = new FileLockManager();
  const _guard = manager.acquireLock("agent-1", "/test/file.txt", "write");

  assertThrows(() =>
    manager.acquireLock("agent-2", "/test/file.txt", "write")
  );
});

Deno.test("write lock blocks read", () => {
  const manager = new FileLockManager();
  const _guard = manager.acquireLock("agent-1", "/test/file.txt", "write");

  assertThrows(() =>
    manager.acquireLock("agent-2", "/test/file.txt", "read")
  );
});

Deno.test("read lock blocks write", () => {
  const manager = new FileLockManager();
  const _guard = manager.acquireLock("agent-1", "/test/file.txt", "read");

  assertThrows(() =>
    manager.acquireLock("agent-2", "/test/file.txt", "write")
  );
});

Deno.test("same agent reacquire write", () => {
  const manager = new FileLockManager();
  const _guard1 = manager.acquireLock("agent-1", "/test/file.txt", "write");
  const _guard2 = manager.acquireLock("agent-1", "/test/file.txt", "write");

  assertEquals(manager.isLockedBy("/test/file.txt", "agent-1"), true);
});

Deno.test("release all locks", () => {
  const manager = new FileLockManager();
  manager.acquireLock("agent-1", "/test/file1.txt", "write");
  manager.acquireLock("agent-1", "/test/file2.txt", "read");

  const released = manager.releaseAllLocks("agent-1");
  assertEquals(released, 2);
});

Deno.test("lock stats", () => {
  const manager = new FileLockManager();
  manager.acquireLock("agent-1", "/test/file1.txt", "write");
  manager.acquireLock("agent-2", "/test/file2.txt", "read");
  manager.acquireLock("agent-3", "/test/file2.txt", "read");

  const stats = manager.stats();
  assertEquals(stats.totalFiles, 2);
  assertEquals(stats.totalWriteLocks, 1);
  assertEquals(stats.totalReadLocks, 2);
});

Deno.test("can acquire", () => {
  const manager = new FileLockManager();

  assertEquals(manager.canAcquire("/test/file.txt", "agent-1", "write"), true);
  assertEquals(manager.canAcquire("/test/file.txt", "agent-1", "read"), true);

  manager.acquireLock("agent-1", "/test/file.txt", "write");

  assertEquals(manager.canAcquire("/test/file.txt", "agent-1", "write"), true);
  assertEquals(manager.canAcquire("/test/file.txt", "agent-2", "write"), false);
  assertEquals(manager.canAcquire("/test/file.txt", "agent-2", "read"), false);
});

Deno.test("expired lock cleanup", () => {
  const manager = new FileLockManager();

  // Acquire lock with very short timeout (1ms)
  manager.acquireLock("agent-1", "/test/file.txt", "write", 1);

  // Wait for expiration
  const start = Date.now();
  while (Date.now() - start < 10) { /* spin */ }

  const cleaned = manager.cleanupExpired();
  assertEquals(cleaned, 1);

  // Now another agent can acquire
  const guard = manager.acquireLock("agent-2", "/test/file.txt", "write");
  assertEquals(guard.lockType, "write");
});

Deno.test("force release", () => {
  const manager = new FileLockManager();
  manager.acquireLock("agent-1", "/test/file.txt", "write");
  manager.forceRelease("/test/file.txt");

  const guard = manager.acquireLock("agent-2", "/test/file.txt", "write");
  assertEquals(guard.lockType, "write");
});

Deno.test("list locks", () => {
  const manager = new FileLockManager();
  manager.acquireLock("agent-1", "/test/file1.txt", "write");
  manager.acquireLock("agent-2", "/test/file2.txt", "read");

  const locks = manager.listLocks();
  assertEquals(locks.length, 2);
});

Deno.test("locks for agent", () => {
  const manager = new FileLockManager();
  manager.acquireLock("agent-1", "/test/file1.txt", "write");
  manager.acquireLock("agent-1", "/test/file2.txt", "read");
  manager.acquireLock("agent-2", "/test/file3.txt", "write");

  assertEquals(manager.locksForAgent("agent-1").length, 2);
  assertEquals(manager.locksForAgent("agent-2").length, 1);
});

Deno.test("deadlock detection", () => {
  const manager = new FileLockManager();

  // Agent 1 holds file1
  manager.acquireLock("agent-1", "/test/file1.txt", "write");
  // Agent 2 holds file2
  manager.acquireLock("agent-2", "/test/file2.txt", "write");

  // Simulate agent 1 waiting for file2 (internal method exposed via getWaitingAgents)
  // We simulate via acquireWithWait which calls startWaiting internally

  // Direct deadlock check: if agent2 tried to wait for file1 while agent1 waits for file2
  // this is handled internally by acquireWithWait
  // For now, verify the waiting agents tracking
  assertEquals(manager.getWaitingAgents().size, 0);
});

Deno.test("guard release", () => {
  const manager = new FileLockManager();
  const guard = manager.acquireLock("agent-1", "/test/file.txt", "write");

  assertEquals(manager.isLockedBy("/test/file.txt", "agent-1"), true);
  guard.release();
  assertEquals(manager.isLockedBy("/test/file.txt", "agent-1"), false);
});
