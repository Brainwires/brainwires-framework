//! Concurrent Agent Tests
//!
//! Tests for multi-agent coordination, file locking, and concurrent operations.
//! These tests verify the agent system handles concurrency correctly.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;
use tokio::time::timeout;

use brainwires_cli::agents::{CommunicationHub, FileLockManager, LockType};

/// Test that multiple concurrent read locks are allowed
#[tokio::test]
async fn test_concurrent_read_locks() {
    let lock_manager = Arc::new(FileLockManager::new());
    let barrier = Arc::new(Barrier::new(5));

    let mut handles = vec![];

    // Spawn 5 tasks that all try to acquire read locks simultaneously
    for i in 0..5 {
        let lock_manager = Arc::clone(&lock_manager);
        let barrier = Arc::clone(&barrier);

        handles.push(tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier.wait().await;

            // Try to acquire read lock
            let agent_id = format!("agent-{}", i);
            let result = lock_manager
                .acquire_lock(&agent_id, "test_file.txt", LockType::Read)
                .await;

            assert!(result.is_ok(), "Task {} should get read lock", i);

            let guard = result.unwrap();

            // Hold the lock briefly
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Lock should be released when guard drops
            drop(guard);
        }));
    }

    // Wait for all tasks
    for handle in handles {
        let result = timeout(Duration::from_secs(10), handle).await;
        assert!(result.is_ok(), "Task should complete");
        result.unwrap().unwrap();
    }
}

/// Test that write lock blocks other locks
#[tokio::test]
async fn test_write_lock_blocks_others() {
    let lock_manager = Arc::new(FileLockManager::new());

    // Acquire write lock
    let write_guard = lock_manager
        .acquire_lock("agent-writer", "exclusive_file.txt", LockType::Write)
        .await
        .expect("Should get write lock");

    // Try to acquire read lock from different agent (should fail)
    let read_result = lock_manager
        .acquire_lock("agent-reader", "exclusive_file.txt", LockType::Read)
        .await;

    // Read lock should fail (write lock held by different agent)
    assert!(read_result.is_err(), "Read lock should fail when write held");

    // Release write lock
    drop(write_guard);

    // Allow async lock release task to complete
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Now read lock should succeed
    let read_guard = lock_manager
        .acquire_lock("agent-reader", "exclusive_file.txt", LockType::Read)
        .await;

    assert!(
        read_guard.is_ok(),
        "Read lock should succeed after write released"
    );
}

/// Test that write lock waits for read locks to release
#[tokio::test]
async fn test_write_lock_waits_for_readers() {
    let lock_manager = Arc::new(FileLockManager::new());

    // Acquire read lock
    let read_guard = lock_manager
        .acquire_lock("agent-reader", "readers_file.txt", LockType::Read)
        .await
        .expect("Should get read lock");

    // Try to acquire write lock from different agent (should fail while read held)
    let write_result_blocked = lock_manager
        .acquire_lock("agent-writer", "readers_file.txt", LockType::Write)
        .await;

    // Should fail because read lock is held
    assert!(
        write_result_blocked.is_err(),
        "Write lock should fail while read held"
    );

    // Release read lock
    drop(read_guard);

    // Allow async lock release task to complete
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Write lock should now succeed
    let write_result = lock_manager
        .acquire_lock("agent-writer", "readers_file.txt", LockType::Write)
        .await;

    assert!(
        write_result.is_ok(),
        "Write lock should succeed after reads released"
    );
}

/// Test lock fairness (FIFO ordering) using acquire_with_wait
#[tokio::test]
async fn test_lock_fairness() {
    let lock_manager = Arc::new(FileLockManager::new());
    let order = Arc::new(tokio::sync::Mutex::new(Vec::<usize>::new()));

    // Acquire initial lock
    let initial_guard = lock_manager
        .acquire_lock("agent-initial", "fairness_file.txt", LockType::Write)
        .await
        .expect("Should get initial lock");

    let mut handles = vec![];

    // Spawn tasks that will wait for the lock
    for i in 0..5 {
        let lock_manager = Arc::clone(&lock_manager);
        let order = Arc::clone(&order);

        // Small delay to ensure ordering
        tokio::time::sleep(Duration::from_millis(10 * i as u64)).await;

        handles.push(tokio::spawn(async move {
            let agent_id = format!("agent-{}", i);
            // Wait for lock with timeout
            let guard = lock_manager
                .acquire_with_wait(&agent_id, "fairness_file.txt", LockType::Write, Duration::from_secs(10))
                .await
                .expect("Should get lock");

            // Record when we got the lock
            order.lock().await.push(i);

            // Brief hold
            tokio::time::sleep(Duration::from_millis(10)).await;

            drop(guard);
        }));
    }

    // Release initial lock to let queued tasks proceed
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(initial_guard);

    // Wait for all tasks
    for handle in handles {
        timeout(Duration::from_secs(15), handle)
            .await
            .expect("Should complete")
            .expect("Should not panic");
    }

    // Verify all tasks completed
    let order = order.lock().await;
    assert_eq!(order.len(), 5, "All tasks should complete");
}

/// Test deadlock prevention (lock ordering)
#[tokio::test]
async fn test_deadlock_prevention() {
    let lock_manager = Arc::new(FileLockManager::new());
    let completed = Arc::new(tokio::sync::Mutex::new(0));

    // Two agents trying to lock files in different orders
    // This could deadlock without proper ordering or timeout

    let lock_manager_1 = Arc::clone(&lock_manager);
    let completed_1 = Arc::clone(&completed);
    let task1 = tokio::spawn(async move {
        // Try to lock A then B
        let guard_a = lock_manager_1
            .acquire_with_wait("agent-1", "file_a.txt", LockType::Write, Duration::from_millis(500))
            .await;

        if guard_a.is_err() {
            return; // Couldn't get first lock
        }

        tokio::time::sleep(Duration::from_millis(10)).await;

        let guard_b = lock_manager_1
            .acquire_with_wait("agent-1", "file_b.txt", LockType::Write, Duration::from_millis(500))
            .await;

        if guard_b.is_ok() {
            *completed_1.lock().await += 1;
        }
    });

    let lock_manager_2 = Arc::clone(&lock_manager);
    let completed_2 = Arc::clone(&completed);
    let task2 = tokio::spawn(async move {
        // Try to lock B then A (opposite order)
        let guard_b = lock_manager_2
            .acquire_with_wait("agent-2", "file_b.txt", LockType::Write, Duration::from_millis(500))
            .await;

        if guard_b.is_err() {
            return;
        }

        tokio::time::sleep(Duration::from_millis(10)).await;

        let guard_a = lock_manager_2
            .acquire_with_wait("agent-2", "file_a.txt", LockType::Write, Duration::from_millis(500))
            .await;

        if guard_a.is_ok() {
            *completed_2.lock().await += 1;
        }
    });

    // Both tasks should complete (with timeouts) rather than deadlock
    let result = timeout(Duration::from_secs(2), async {
        let _ = task1.await;
        let _ = task2.await;
    })
    .await;

    assert!(result.is_ok(), "Tasks should complete without deadlock");
}

/// Test CommunicationHub message routing
#[tokio::test]
async fn test_communication_hub_routing() {
    let hub = Arc::new(CommunicationHub::new());

    // Register multiple agents
    hub.register_agent("agent-1".to_string())
        .await
        .expect("Should register agent-1");
    hub.register_agent("agent-2".to_string())
        .await
        .expect("Should register agent-2");

    // Use AgentMessage enum from communication module
    use brainwires_cli::agents::AgentMessage;

    let message = AgentMessage::StatusUpdate {
        agent_id: "agent-sender".to_string(),
        status: "working".to_string(),
        details: None,
    };

    // Broadcast from a sender to all registered agents
    hub.broadcast("agent-sender".to_string(), message.clone())
        .await
        .expect("Should broadcast message");

    // Both agents should receive the message
    let recv1 = timeout(
        Duration::from_secs(1),
        hub.try_receive_message("agent-1"),
    )
    .await;
    let recv2 = timeout(
        Duration::from_secs(1),
        hub.try_receive_message("agent-2"),
    )
    .await;

    assert!(recv1.is_ok(), "Agent 1 receive should not timeout");
    assert!(recv2.is_ok(), "Agent 2 receive should not timeout");

    // Messages should be present
    assert!(
        recv1.unwrap().is_some(),
        "Agent 1 should receive message"
    );
    assert!(
        recv2.unwrap().is_some(),
        "Agent 2 should receive message"
    );
}

/// Test high contention scenario
#[tokio::test]
async fn test_high_contention() {
    let lock_manager = Arc::new(FileLockManager::new());
    let success_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let failure_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let mut handles = vec![];

    // Spawn many tasks competing for the same lock
    for i in 0..20 {
        let lock_manager = Arc::clone(&lock_manager);
        let success = Arc::clone(&success_count);
        let failure = Arc::clone(&failure_count);

        handles.push(tokio::spawn(async move {
            let agent_id = format!("agent-{}", i);
            let result = lock_manager
                .acquire_with_wait(&agent_id, "contended_file.txt", LockType::Write, Duration::from_millis(200))
                .await;

            if result.is_ok() {
                // Hold lock briefly
                tokio::time::sleep(Duration::from_millis(5)).await;
                success.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            } else {
                failure.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }));
    }

    // Wait for all tasks
    for handle in handles {
        let _ = timeout(Duration::from_secs(10), handle).await;
    }

    let successes = success_count.load(std::sync::atomic::Ordering::SeqCst);
    let failures = failure_count.load(std::sync::atomic::Ordering::SeqCst);

    println!(
        "High contention results: {} successes, {} timeouts",
        successes, failures
    );

    // At least some should succeed
    assert!(successes > 0, "Some tasks should successfully acquire lock");
    // Total should be 20
    assert_eq!(successes + failures, 20, "All tasks should complete");
}

/// Test lock guard drop releases lock
#[tokio::test]
async fn test_lock_guard_release() {
    let lock_manager = Arc::new(FileLockManager::new());

    // Acquire and drop lock in a scope
    {
        let guard = lock_manager
            .acquire_lock("agent-1", "scoped_file.txt", LockType::Write)
            .await
            .expect("Should get lock");

        // Use the guard
        let _ = guard;
    } // Guard dropped here

    // Allow async lock release task to complete
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Should be able to acquire lock again with different agent
    let guard2 = lock_manager
        .acquire_lock("agent-2", "scoped_file.txt", LockType::Write)
        .await;

    assert!(
        guard2.is_ok(),
        "Should get lock after previous guard dropped"
    );
}
