# Architectural Principles for Production Agentic Systems

## The Meta-Principle

> "The more autonomous the system, the more deterministic the surrounding infrastructure must be."

Autonomy inside. Determinism outside.

This is the master constraint from which all other principles follow. As the agent's decision-making
scope expands, the infrastructure surrounding it must become *more* constrained, *more* observable,
and *more* controllable — not less. The common failure mode is the reverse: as autonomy expands,
infrastructure discipline relaxes because "the agent handles it."

---

## Principle 1: Constrain Autonomy With Explicit Control Planes

### The Principle
The LLM is a decision component inside a deterministic control plane, not the control plane itself.

The control plane owns:
- **State machine**: defined states with allowed transitions
- **Step budget**: maximum iterations enforced by the orchestrator
- **Tool whitelist**: per-state allowed tool set (not global access)
- **Termination rules**: completion conditions defined in code

The LLM decides *what to do next within the allowed action space*. The control plane decides
*whether that action is permitted* and *when the execution stops*.

### Why This Matters
An LLM making decisions without an explicit control plane is an LLM defining its own execution policy.
That policy is stochastic, not reproducible, and cannot be audited.

### Brainwires Implementation
- `TaskAgentConfig.max_iterations` in `src/agents/task_agent.rs:69` — step budget
- `TaskAgentStatus` enum (Idle/Working/WaitingForLock/Completed/Failed) — state machine
- `PermissionMode` (Auto/Ask/Reject) — tool access control at execution time
- `FileLockManager.acquire_lock()` — resource access gating

---

## Principle 2: Strict, Typed, Idempotent Tool Contracts

### The Principle
Never let the model directly perform irreversible side effects. Every tool invocation must pass through:
1. **Schema validation** — structural correctness (fail fast before execution)
2. **Semantic validation** — intent consistency with current state
3. **Capability check** — is this agent permitted this action in this context?
4. **Idempotency key** — can this call be safely retried without side effects?
5. **Side-effect transparency** — what will this tool change, and is that reversible?

Treat every tool invocation as an untrusted remote procedure call.

### Why This Matters
API-Bank benchmark (arXiv:2304.08244) demonstrates that schema adherence is probabilistic even with
structured output modes — failure rates of 12–30% depending on API complexity. External validation
must be deterministic even when model output is not.

### Brainwires Implementation
- `ToolExecutor` in `src/tools/executor.rs` — validation + permission checking before execution
- `PolicyEngine` + `PolicyRequest` in `brainwires-permissions/src/policy.rs` — declarative rules
- `AuditLogger` — every tool execution logged with outcome
- `FileLockManager` — exclusive write locks prevent concurrent corruption

**Gap:** Idempotency keys and side-effect staging for write operations are not yet implemented.

---

## Principle 3: Separate Planning From Execution

### The Principle
Planning and execution are fundamentally different activities and must be separated:

**Planning phase** produces a plan that is:
- **Serializable** — can be stored, inspected, and transmitted
- **Inspectable** — a human or validator can read and evaluate it before execution
- **Interruptible** — can be paused, modified, or cancelled before any side effects
- **Cost-evaluable** — estimated cost can be checked against budget before execution begins

**Execution phase** is:
- **Deterministic** — same plan = same actions (given the same tool outputs)
- **Logged** — every action recorded in the execution graph
- **Replayable** — execution can be reconstructed from the log

### Why This Matters
Mixing planning and execution means plans are never inspectable and costs are never predictable.
The model makes irreversible decisions before anyone has had a chance to evaluate whether they
are correct.

### Brainwires Implementation
- MDAP decomposition phase (`brainwires-mdap/src/decomposition/`) — produces subtask plan
- Validation loop (in `attempt_validated_completion()`) — execution gate before completion
- `ValidationConfig.working_set_files` — tracks what was actually modified during execution
- `brainwires-core/src/plan.rs` — `Plan` type with steps and dependencies

---

## Principle 4: Version Prompts as Artifacts

### The Principle
A prompt is a program. A change to a prompt is a code change. Apply all code change disciplines:
- Version control with semantic identifiers
- Regression evaluation before deployment
- Staged rollout for high-traffic prompts
- Automatic snapshot: every run records the exact prompt that produced it

Every run must carry: model version + temperature + prompt hash + tool registry hash. Together,
these form a "build ID" that uniquely identifies the behavior configuration.

### Why This Matters
A wording change can break tool calling syntax, change termination semantics, or alter error recovery
behavior in ways that are completely invisible until failures surface in production.

### Brainwires Implementation
- System prompts compiled as Rust constants in `src/agents/system_prompts.rs` — compile-time
  stability, no runtime drift
- `TaskAgentConfig.temperature` and `TaskAgentConfig.system_prompt` — tracked per agent run
- **Gap:** Prompt versioning with semantic IDs and CI regression evaluation not yet implemented

---

## Principle 5: Build an Execution Graph, Not Just Logs

### The Principle
Every agent run produces an execution DAG:
- Nodes: each step (think, tool call, validation, retrieval)
- Edges: causal dependencies between steps
- Annotations: token usage, latency, tool arguments, validator results at every node

Replay capability is mandatory: given an execution graph + frozen tool outputs + frozen model
version, the run must be fully reproducible.

### Why This Matters
Graph-of-Thoughts (arXiv:2308.09687) shows that execution as a DAG enables aggregation, merging,
and refinement operations impossible in linear execution. For debugging: bugs that manifest at step
15 of 20 require causal tracing back through the graph to find the root cause at step 3.

### Reference Structure

```
ExecutionGraph {
  run_id: UUID,
  model_version: String,
  prompt_hash: String,
  tool_registry_hash: String,
  start_time: DateTime,
  nodes: Vec<StepNode>,
  total_tokens: TokenUsage,
  total_cost_usd: f64,
}

StepNode {
  id: u32,
  step_type: Think | ToolCall | Validation | Retrieval | Completion,
  parent_ids: Vec<u32>,
  input_tokens: u32,
  output_tokens: u32,
  latency_ms: u64,
  tool_name: Option<String>,
  tool_args: Option<Value>,
  result: Option<String>,
  error: Option<String>,
}
```

### Brainwires Implementation
- `MdapMetrics` + `SubtaskMetric` in `brainwires-mdap/src/metrics.rs` — per-subtask tracking
- `AuditEvent` in `brainwires-permissions/src/audit.rs` — tool execution logging
- **Gap:** Full execution DAG per non-MDAP run and replay capability not yet implemented

---

## Principle 6: Design Memory With Authority Hierarchy

### The Principle
Memory tiers must have explicit authority levels and governance:

| Tier | Scope | Authority | Storage | Write Access |
|------|-------|-----------|---------|--------------|
| Ephemeral | Per-step | Lowest | Context window only | Any agent |
| Session | Per-run | Medium | DB (temporary) | Any agent in session |
| Canonical | Global | Highest | Persistent DB | Authorized sources only |

Governance requirements:
- **TTL policies**: non-canonical memories expire automatically
- **Retrieval confidence thresholds**: minimum similarity score before injection
- **Poison detection**: flag conflicting facts for human review
- **Canonical override rule**: confirmed canonical facts always beat retrieved session memories

### Why This Matters
Memory without authority hierarchy collapses into a single flat namespace where any stored fact can
override any other. Model-generated content cannot be trusted at canonical authority level — it may
contain errors that propagate into future reasoning.

### Brainwires Implementation
- `TieredMemory` (hot/warm/cold) in `brainwires-storage` — tier structure
- `MessageStore` with LanceDB persistence — warm/cold storage
- `WorkingSet` in `brainwires-core/src/working_set.rs` — ephemeral file tracking
- **Gap:** Canonical authority layer, poison detection, and TTL policies not fully implemented

---

## Principle 7: Enforce Cost, Latency, Token Budgets at Runtime

### The Principle
Budget enforcement must be automatic and non-optional. Budget parameters must be:
- **Configurable per workflow** (a code review task has different budget than full refactor)
- **Enforced by the orchestrator** (not dependent on the model staying within budget)
- **Applied at multiple levels** (per-call, per-run, per-model, per-project)

Budget axes:
- Maximum steps per run
- Maximum tokens per run (input + output combined)
- Maximum reflection/critique loops
- Timeout ceiling (wall-clock time)
- Cost ceiling per run (USD)

When a budget ceiling is reached: return partial results, not silence.

### Why This Matters
FrugalGPT (arXiv:2305.05176) demonstrates 98% cost reduction through model cascading with
appropriate routing. Without explicit budget enforcement, single runaway tasks can consume
resources intended for hundreds of normal tasks.

### Brainwires Implementation
- `TaskAgentConfig.max_iterations` — step budget
- `TaskAgentConfig.max_tokens` — per-call token limit (4096 default)
- `MdapConfig.max_samples_per_subtask` — MDAP sampling budget
- `MdapMetrics.actual_cost_usd` — cost tracking
- **Gap:** Total per-run token budget, cost ceiling, and timeout ceiling not yet implemented

---

## Principle 8: Deterministic Arbitration in Multi-Agent Systems

### The Principle
Multi-agent systems require distributed systems engineering, not emergent collaboration:

Requirements:
- **Role authority hierarchy**: each role has defined scope; agents cannot act outside it
- **Single orchestrator**: one orchestrator spawns and coordinates workers; no peer-to-peer recursion
- **Conflict resolution rules**: when agents disagree, resolution is by defined rule (first-write-wins,
  higher-authority-wins, human-escalation) — not by negotiation
- **Shared structured state**: agents communicate via typed messages, not free-form conversation
- **Termination contracts**: every agent interaction has a defined exit condition before it starts

### Why This Matters
AutoGen (arXiv:2308.08155) and MetaGPT (arXiv:2308.00352) both demonstrate that emergent multi-agent
collaboration works in demo settings but requires explicit coordination contracts in production to
avoid contradictions, deadlocks, and exponential token growth.

### Brainwires Implementation
- `CommunicationHub` in `src/agents/` — typed `AgentMessage` enum, broadcast + receive
- `FileLockManager` in `src/agents/` — deterministic read/write conflict resolution
- `OrchestratorAgent` in `src/agents/orchestrator.rs` — single-orchestrator pattern
- `AgentPool` in `src/agents/pool.rs` — lifecycle management for worker agents

---

## Principle 9: Test Behavior, Not Output

### The Principle
Success metric: goal completion under uncertainty, not output string equality.

Testing methodology:
- **N-run Monte Carlo evaluation**: success = P(goal completion | task distribution) > threshold
- **Tool sequence validation**: verify the sequence of tool calls, not just the final output
- **Adversarial prompt suites**: test with malformed inputs, ambiguous instructions, injection attempts
- **Long-horizon stability**: test tasks requiring 15+ steps to surface accumulation failures
- **Regression scoring**: track success rates across model versions and prompt changes

Treat threshold as a product requirement. P(success) = 0.95 for a code review task is a different
requirement than P(success) = 0.99 for a database migration task.

### Why This Matters
HELM (arXiv:2211.09110) established multi-dimensional distributional evaluation as the standard.
Binary pass/fail is insufficient for stochastic systems: you need confidence intervals, not booleans.

### Brainwires Implementation
- `test-results/` manual test archive with star ratings (42 tests, 95% success rate)
- `ValidationLoop` as automated behavioral gate before task completion
- **Gap:** Automated Monte Carlo evaluation framework not yet implemented

---

## Principle 10: Fail Fast and Loud

### The Principle
When something is wrong, abort clearly. Don't push through.

Failure modes that require immediate abort:
- Schema-invalid tool call (after max retries)
- File operation on path outside working directory
- Tool call requesting capability not in whitelist
- Validation failure after max retry count
- Cost or step budget exceeded

On abort: surface the exact failure with context (which step, which tool, what the args were, what
the validator said). Return partial results with clear failure indication rather than silence.

### Why This Matters
Pushing through invalid states produces silent corruption that's harder to debug than an explicit
failure. A model that "tries again with slightly different args" after a permission denied error is
not recovering — it's obscuring the root cause.

### Brainwires Implementation
- `TaskAgentStatus::Failed(error)` — explicit failure with message
- `attempt_validated_completion()` in `task_agent.rs` — abort on validation failure, inject feedback
- `PolicyEngine` — deny on permission violation, don't retry

---

## Principle 11: Keep Humans in the Control Loop Strategically

### The Principle
Don't try to eliminate human involvement — strategically place it where it adds maximum value:
- **Gate irreversible actions**: file deletes, external API calls, schema migrations always require approval
- **Escalate uncertainty**: when confidence is below threshold, surface to human rather than guess
- **Provide intervention hooks**: running agent tasks should be interruptible by human override
- **Autonomy degrades gracefully**: when something goes wrong, fall back to more human supervision,
  not less

### Why This Matters
The trust failure mode is not "human is too involved" — it's "human discovers a serious failure and
no intervention was possible." A single irreversible failure with no human gate destroys more trust
than requiring approval for 100 routine operations.

### Brainwires Implementation
- `PermissionMode::Ask` — human approval required for all tool calls
- `PermissionMode::Reject` — no tool calls allowed (fully supervised)
- `approval_tx` in `ToolExecutor` — approval channel for interactive approval requests
- `PolicyEngine` rules can require human approval for specific tool/file combinations

---

## Principle 12: Design for Replayability

### The Principle
Every run must be fully reproducible from its execution record. Requirements:

- **Deterministic seeds**: random number generation seeded from run ID
- **Snapshotted prompts**: exact prompt text (not version reference) stored with run
- **Snapshotted tool I/O**: exact tool call arguments and results stored
- **Frozen model version**: model ID (including minor version) recorded
- **Frozen tool registry**: hash of registered tools recorded

Given this execution record + mocked tool outputs, the exact same decisions should be produced.

### Why This Matters
Bugs in production multi-step agents are rarely reproducible without replay capability. The "it worked
twice but failed on the third run" problem is only debuggable with full execution records.

### Brainwires Implementation
- `AuditLogger` records tool executions with arguments and outcomes
- `MdapMetrics.execution_id` identifies runs
- **Gap:** Full replay capability (frozen model version, deterministic seeding) not yet implemented

---

## Principle 13: Treat LLMs as Policy Engines, Not Truth Engines

### The Principle
Assume the model can be wrong at any step:
- Hallucinate parameters or facts
- Misuse tools (correct schema, wrong semantic intent)
- Self-justify errors in reflection ("I was right because...")
- Generate plausible-sounding but incorrect plans

Therefore:
- **External validation**: never accept model output as correct without external verification
- **Structured verification**: use deterministic tools (build, test, lint, file existence) to verify claims
- **Confidence scoring**: maintain uncertainty estimates and act conservatively at low confidence
- **Double-check high-risk decisions**: any action with high blast radius requires external validation
  before execution

### Why This Matters
Reflexion (arXiv:2303.11366) shows that self-reflection improves performance statistically but
fails systematically when the model has fundamental errors (same bias = same error in reflection).
External grounding is necessary for reliability.

### Brainwires Implementation
- `ValidationLoop` runs `verify_build`, `check_duplicates`, `check_syntax` externally
- `check_working_set_files()` in validation: confirms files actually exist on disk (Bug #5 fix)
- `ThreeStateModel` in `brainwires-agents` — external state validation against declared model

---

## Principle 14: Design for Gradual Capability Expansion

### The Principle
Start narrow. Constrain. Measure. Expand carefully.

Expansion sequence:
1. Single tool → prove reliability → add second tool
2. Single file type → prove reliability → add second file type
3. Single reversible action type → prove reliability → add irreversible action with approval gate
4. Single-agent → prove reliability → add second agent with explicit coordination contract

At each stage: catalog failure modes, establish success rate baseline, set threshold for expansion.

### Why This Matters
Breadth kills reliability. Every new tool, every new action type, every new agent adds exponential
interaction surface. Production systems that try to be comprehensive from day one have no baseline
to measure quality against.

### Brainwires Implementation
- `ToolRegistry.with_builtins()` — curated default tool set, not everything
- `PermissionMode` — constrains tool access per deployment
- `ValidationConfig.checks` — configurable validation checklist per task type

---

## Principle 15: Treat Agentic Systems as Runtime Systems

### The Principle
The LLM is one component. Engineer the surrounding system.

A production agentic system is:
- A distributed runtime (multiple agents, concurrent execution)
- A state management system (working set, memory tiers, task graph)
- A cost control system (token budgets, model routing)
- A security system (permission scoping, audit logging, sandboxing)
- An observability system (execution graphs, metrics, tracing)
- A reliability system (validation, retry logic, failure classification)

None of these are AI problems. They are software engineering problems that happen to have an AI
component inside them.

---

## Reference Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Control Plane                                │
│  ┌─────────────┐    ┌──────────────┐    ┌─────────────────────┐    │
│  │ State Machine│    │  Step Budget │    │  Permission Engine  │    │
│  │ (Orchestrator│    │  (max_iter)  │    │  (PolicyEngine)     │    │
│  └──────┬───────┘    └──────┬───────┘    └──────────┬──────────┘    │
│         │                  │                        │               │
│         └──────────────────┴────────────────────────┘               │
│                            │                                        │
│                            ▼                                        │
│                  ┌─────────────────┐                                │
│                  │   LLM Policy    │  ← model call                  │
│                  │  (TaskAgent)    │                                │
│                  └────────┬────────┘                                │
│                           │ proposed action                         │
│                           ▼                                        │
│                  ┌─────────────────┐                                │
│                  │  Tool Validator │  ← schema + semantic + perms   │
│                  └────────┬────────┘                                │
│                           │ validated action                        │
│                           ▼                                        │
│          ┌────────────────────────────────┐                        │
│          │           Executor             │                        │
│          │  (FileLockManager + ToolExec)  │                        │
│          └────────────┬───────────────────┘                        │
│                       │                                            │
│           ┌───────────┴───────────┐                               │
│           ▼                       ▼                               │
│  ┌─────────────────┐   ┌──────────────────────┐                  │
│  │ Execution Graph  │   │    Memory Manager    │                  │
│  │ + Cost Engine   │   │    (TieredMemory)    │                  │
│  │ (MdapMetrics)   │   │                      │                  │
│  └─────────────────┘   └──────────────────────┘                  │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                    Audit Logger                              │  │
│  │              (brainwires-permissions)                        │  │
│  └─────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

The LLM occupies one box — "LLM Policy." Everything else is deterministic infrastructure.
