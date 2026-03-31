# ADR 0002: Multi-Agent Architecture

## Status

Accepted

## Context

Complex coding tasks often require:
- Breaking down large problems into smaller subtasks
- Parallel execution of independent work
- Coordination to prevent conflicts (e.g., concurrent file edits)
- Recovery from partial failures

A single-agent approach struggles with:
- Long-running tasks blocking user interaction
- Complex tasks exceeding context limits
- No parallelism for independent subtasks
- All-or-nothing failure modes

## Options Considered

### 1. Single Agent with Sequential Execution

**Pros:**
- Simple implementation
- No coordination overhead
- Easy to debug

**Cons:**
- No parallelism
- Single point of failure
- Context limits constrain task complexity

### 2. Multi-Agent with Message Passing (Chosen)

**Pros:**
- Parallel execution of independent tasks
- Fault isolation per agent
- Scales to complex problems
- Clear separation of concerns

**Cons:**
- Coordination complexity
- Potential for deadlocks
- Communication overhead

### 3. Thread Pool / Task Queue

**Pros:**
- Proven pattern
- Good resource utilization

**Cons:**
- No semantic understanding of task relationships
- Difficult to handle dependencies
- No agent-level isolation

## Decision

Implement a **multi-agent architecture** with:

1. **Orchestrator Agent**: Decomposes tasks, spawns workers, aggregates results
2. **Task Agents**: Execute specific subtasks autonomously
3. **Communication Hub**: Pub/sub message passing for coordination
4. **File Lock Manager**: Prevents concurrent file conflicts

## Architecture

```
┌─────────────────┐
│   Orchestrator  │
│     Agent       │
└────────┬────────┘
         │ spawns
    ┌────┴────┐
    v         v
┌───────┐ ┌───────┐
│Task   │ │Task   │
│Agent 1│ │Agent 2│
└───┬───┘ └───┬───┘
    │         │
    v         v
┌─────────────────┐
│ Communication   │
│      Hub        │
└─────────────────┘
    │         │
    v         v
┌─────────────────┐
│  File Lock      │
│  Manager        │
└─────────────────┘
```

## Key Patterns

### Communication Hub
- Typed `AgentMessage` enum
- Register/unregister agents
- Point-to-point and broadcast messaging
- Async receive with timeout

### File Locking
- Read locks (shared): Multiple agents can read
- Write locks (exclusive): Only one agent can write
- Lock guards with automatic release on drop
- Deadlock detection via wait graph analysis

### Validation Loop
- Pre-completion checks: file existence, duplicates, syntax, build
- Configurable retry count
- Prevents "success without output" bugs

## Consequences

### Positive
- Complex tasks can be parallelized
- Agent failures are isolated
- Clear debugging via message traces
- Enables MDAP (voting) for reliability

### Negative
- Higher implementation complexity
- Potential for subtle coordination bugs
- Overhead for simple tasks

### Mitigations
- Thorough testing of concurrent scenarios
- Simple single-agent mode for basic tasks
- Timeouts to prevent indefinite waits

## References

- `src/agents/` - Agent implementations
- `src/agents/communication.rs` - Communication hub
- `src/agents/file_locks.rs` - File lock manager
- `src/agents/validation_loop.rs` - Validation system
