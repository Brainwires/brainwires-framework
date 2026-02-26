# Production Anti-Patterns in Agentic Systems

## Overview

The meta-pattern underlying all other anti-patterns: **treating agentic systems as prompt problems instead
of runtime systems**. Teams that succeed in production treat the LLM as a decision component inside a
deterministic infrastructure envelope. Teams that fail treat the LLM as the system itself and try to
engineer everything through prompt tuning.

This document catalogs 11 anti-patterns observed repeatedly across production agent system failures. Each
entry includes the smell (how to recognize it), what breaks, the root cause, and the fix.

---

## Anti-Pattern 1: "Let the Model Figure It Out" Architecture

### The Smell
- Single mega-prompt containing all instructions, context, and task specification
- No explicit state machine governing allowed transitions
- Unlimited tool access (agent has every tool registered)
- No step budget or token ceiling
- Agent decides its own completion criteria

### What Happens
- The model defines its own execution policy, which changes across runs
- Irrelevant tool calls consume budget while producing no progress
- Hallucinated capabilities ("I'll use the deploy_to_production tool") cause silent failures
- Infinite loops (model keeps calling `search` hoping to find an answer that doesn't exist)
- 40-step task completion when 5 steps would suffice

### Root Cause
LLMs are excellent at generating plausible next actions. They are poor at maintaining a consistent
execution policy across many steps when that policy is implicit in the prompt. The model is being asked to
be both the decision engine and the control plane simultaneously.

### The Fix
Explicit orchestration layers:
- Finite state machine with defined states and allowed transitions
- Tool whitelist per state (you can only call `write_file` from the `EXECUTING` state, not `PLANNING`)
- Step budget enforced by the orchestrator, not the model
- Completion condition defined in code, not in the model's judgment

**Brainwires reference:** `TaskAgentConfig.max_iterations` + `TaskAgentStatus` enum in
`src/agents/task_agent.rs:22-48` enforce this pattern.

---

## Anti-Pattern 2: Prompts as Configuration Instead of Code

### The Smell
- System prompts stored as string literals in application configuration
- No version control for prompt changes
- "Quick fix" prompt tweaks deployed directly to production
- No evaluation run before prompt promotion
- Prompts have no semantic identifier or hash

### What Happens
- A one-word change to a prompt breaks tool calling syntax the model expected
- New model versions change behavior with the same prompt — no one notices for weeks
- Multiple developers make conflicting prompt changes that interact
- "It was working yesterday" bugs with no diff to examine
- Prompt regressions silently reduce task success rates

### Root Cause
Prompts define executable behavior: they specify allowed tools, error recovery, output format, and
termination semantics. They are programs. A change to a prompt changes program behavior. Version control,
regression testing, and staged deployment apply.

### The Fix
- Version prompts with semantic identifiers (not just string hashes)
- Snapshot the prompt with every run — the exact prompt used for run X must be reproducible
- Run evaluation suite before promoting prompt changes (same methodology as testing code changes)
- Track: model version + temperature + prompt version + tool registry version as a single "build ID"
- Require CI approval for prompt changes that affect core execution paths

**Brainwires reference:** System prompts compiled as Rust constants in `src/agents/system_prompts.rs`
enforce compile-time stability; temperature and model version tracked in `TaskAgentConfig`.

---

## Anti-Pattern 3: No Deterministic Guardrails Around Tool Use

### The Smell
- Tool call JSON schema validation is "best effort"
- Retry loop: try → fail → try again with same args
- Tool registry exposes all available tools to all agents in all contexts
- No idempotency on write operations
- Trusting that schema-valid means semantically valid

### What Happens
- Silent corruption: a structurally valid tool call writes garbage to a file because the semantic
  intent was misread
- Partial writes: a write operation partially completes, leaving the file in a corrupt state
- Wrong tool, correct schema: model calls `delete_file` with correct JSON but meant `read_file`
- Retry amplification: retrying a non-idempotent write operation multiple times creates duplicate data

### Root Cause
The type system mismatch between LLM text generation and typed function signatures is never resolved
by schema validation alone. Schema validation answers "is this structurally valid JSON?" not "is this
semantically the right call to make?"

### The Fix
Strict validation pipeline:
1. **JSON schema validation** — structural correctness (can fail fast before execution)
2. **Semantic validator** — does the intent make sense given current state?
3. **Capability check** — is this agent allowed to make this call in this state?
4. **Idempotency key** — can this call be safely retried?
5. **Side-effect staging** — stage irreversible operations before committing

**Brainwires reference:** `ToolExecutor` in `src/tools/executor.rs` with `PolicyEngine` + `AuditLogger`
from `brainwires-permissions` implements permission checking before execution.

---

## Anti-Pattern 4: Memory Without Lifecycle Management

### The Smell
- "Store everything in the vector DB" — every message, every tool result, every intermediate state
- No TTL on stored memories
- No distinction between facts of different authority levels
- Embedding every message at input time regardless of relevance
- No mechanism for canonical facts to override retrieved memories

### What Happens
- Context window fills with obsolete, superseded, or irrelevant memories
- Retrieval returns thematically similar but task-irrelevant content
- A wrong fact stored in session memory overrides the correct fact in long-term memory
- Model compounds errors: reads its own wrong output from memory and treats it as authoritative input
- Vector DB query performance degrades as size grows unboundedly

### Root Cause
Memory without governance is a liability, not an asset. The amount of stored information is inversely
related to retrieval precision when no lifecycle management exists.

### The Fix
Memory tiers with explicit lifecycle:
- **Ephemeral** — per-step, never stored, used only in current context window
- **Session-bound** — persists for current run, cleared on completion or timeout
- **Canonical** — persists across sessions, requires explicit write authority, cannot be overwritten
  by model output
- TTL policies on non-canonical memories
- Retrieval confidence thresholds (minimum similarity score before injection)
- Canonical override: a confirmed fact always beats a retrieved memory

**Brainwires reference:** `TieredMemory` (hot/warm/cold) in `brainwires-storage` implements tiered
persistence. Canonical authority and poison detection are current gaps (see `06-research-to-production-mapping.md`).

---

## Anti-Pattern 5: Testing Like CRUD Software

### The Smell
- Test suite consists entirely of unit tests with mocked model responses
- Single-run verification: "I ran it once and it worked"
- Output snapshot tests: "the output should equal this exact string"
- No adversarial prompts in the test suite
- Success is binary pass/fail rather than success rate over N runs

### What Happens
- Tests pass in CI with mocked responses, fail in production with real model variance
- Edge cases that appear 1-in-20 runs never surface in single-run testing
- Output snapshot tests break every time the model is updated (and the fix is to update the snapshot)
- Long-horizon task failures (problems at step 15 of 20) never surface in short tests
- "Works in dev" syndrome: optimistic prompt, clean inputs, no adversarial content

### Root Cause
Classical unit testing assumes deterministic, repeatable functions. Agentic systems are stochastic
multi-step processes. The testing methodology must match the system's stochastic, behavioral nature.

### The Fix
- **Monte Carlo evaluation**: run N trials (N ≥ 30 for statistical significance), measure P(success)
- **Behavioral metrics**: goal completion rate, tool sequence correctness, iteration efficiency
- **Adversarial prompt suite**: malformed inputs, ambiguous instructions, prompt injection attempts
- **Long-horizon stability tests**: run 20+ step tasks and check for loops, drift, and early exits
- **Tool sequence diffing**: verify the sequence of tool calls matches expected behavior patterns

**Brainwires reference:** `test-results/` manual test archive with star ratings; automated Monte Carlo
framework is a documented gap in the implementation (see `06-research-to-production-mapping.md`).

---

## Anti-Pattern 6: Infinite Autonomy Without Step Budgets

### The Smell
- No `max_iterations` or equivalent parameter
- No token budget for the entire workflow (only per-call limits)
- No loop detection in the execution history
- No cost ceiling per workflow
- "Let the agent run until it's done"

### What Happens
- 100-step execution chains for tasks that should take 10 steps (Bug #3 in Brainwires test results)
- Recursive critique loops: agent calls a reviewer, reviewer calls another reviewer, ad infinitum
- Cost spikes: a single task consumes the monthly API budget
- Memory overflow: conversation history grows to fill and exceed context limits
- The agent "tries harder" by repeating the same failing approach with minor variations

### Root Cause
Without explicit budgets, agents have no forcing function to find efficient solutions. LLMs are very
good at generating plausible "one more thing to try" continuations.

### The Fix
Hard execution ceilings at multiple levels:
- **Step budget**: `max_iterations` enforced by the orchestrator (not the model)
- **Token budget**: total tokens consumed across the entire workflow
- **Reflection budget**: maximum number of self-reflection or critique loops
- **Loop detection**: same tool + similar args > N times triggers forced exit
- **Cost-aware planning**: estimate cost at planning time, refuse over-budget plans
- **Early exit**: if partial results are available and budget is nearly exhausted, return them

**Brainwires reference:** `TaskAgentConfig.max_iterations` (default: 100); `MdapConfig.max_samples_per_subtask`
in `crates/brainwires-framework/crates/brainwires-mdap/src/`; iteration limit check at
`src/agents/task_agent.rs:261`.

---

## Anti-Pattern 7: Multi-Agent Without Coordination Contracts

### The Smell
- "Let's add a reviewer agent" without specifying what the reviewer has authority to change
- "Critic agent + worker agent" conversation loops with no termination condition
- Agents communicate in free-form natural language with no structured message schema
- No single orchestrator; agents directly call each other peer-to-peer
- No defined role authority hierarchy

### What Happens
- Contradictions: Critic agent reverts what Worker agent just built; they loop forever
- Deadlocks: Agent A waits for Agent B; Agent B waits for Agent A
- Exponential token growth: each agent adds context, the message chain grows quadratically
- Emergent conflict: two agents with overlapping scopes produce inconsistent results
- No audit trail: which agent made which decision for what reason?

### Root Cause
Multi-agent systems are distributed systems. Distributed systems require explicit coordination
protocols, not emergent negotiation. "Let agents figure it out" produces the same failure modes
as distributed systems without consensus protocols.

### The Fix
- **Single orchestrator pattern**: one orchestrator spawns and coordinates worker agents
- **Role authority hierarchy**: each role has defined scope; agents cannot act outside their scope
- **Deterministic arbitration**: when agents disagree, resolution is by rule (not by negotiation)
- **Shared structured state**: agents communicate via structured messages, not free-form conversation
- **Termination contracts**: every agent interaction has a defined exit condition

**Brainwires reference:** `CommunicationHub` provides typed message passing (`AgentMessage` enum);
`FileLockManager` provides deterministic conflict resolution; `OrchestratorAgent` in
`src/agents/orchestrator.rs` implements the single-orchestrator pattern.

---

## Anti-Pattern 8: No Observability Beyond Logs

### The Smell
- Debugging = grepping through console logs
- No structured trace per run
- No per-step token usage tracking
- No execution graph visualization
- "The agent failed but I don't know at which step or why"

### What Happens
- Production incidents are impossible to debug without reproducing the exact run
- Performance regressions are undetectable without per-step timing
- Cost overruns can't be attributed to specific steps or agents
- Model version changes produce behavior changes with no visibility into which steps changed
- Multi-agent bugs require correlating logs from multiple agent contexts

### Root Cause
Distributed systems require distributed tracing. An agentic system is a distributed computation
across N model calls, M tool invocations, and K memory retrievals. Log lines are insufficient for
understanding causality in this graph.

### The Fix
Build execution graphs, not just logs:

```
Run {
  id: UUID,
  model_version: String,
  prompt_hash: String,
  tool_registry_hash: String,
  temperature: f32,
  nodes: Vec<ExecutionNode>,
}

ExecutionNode {
  step: u32,
  type: Think | ToolCall | Validation | Retrieval,
  input_tokens: u32,
  output_tokens: u32,
  latency_ms: u64,
  tool_name: Option<String>,
  tool_args: Option<Value>,
  tool_result: Option<String>,
  validator_result: Option<ValidationResult>,
}
```

Required: replay capability. Every run must be reproducible from its execution graph.

**Brainwires reference:** `MdapMetrics` + `SubtaskMetric` in
`crates/brainwires-framework/crates/brainwires-mdap/src/metrics.rs` tracks per-subtask metrics.
`AuditLogger` in `brainwires-permissions` tracks tool executions. Full execution DAG replay is a
documented gap.

---

## Anti-Pattern 9: Over-Trusting Reflection and Self-Critique

### The Smell
- "Add a reflection step before the agent finishes to improve quality"
- Self-improvement loops: agent critiques its own output, then revises, then critiques, then revises
- Using the same model to evaluate the model's output
- Assuming reflection always improves quality

### What Happens
- Latency doubles or triples for marginal quality improvement
- Cost increases proportionally with reflection rounds
- Occasionally, reflection degrades quality (model convinces itself the correct answer is wrong)
- Reflection loops can become infinite if the completion condition is "until the agent is satisfied"
- The model's self-critique is correlated with the model's original error (same bias, same gaps)

### Root Cause
LLMs are statistically correlated with themselves. An LLM critiquing its own output faces the same
systematic biases and knowledge gaps as the original output. Independent validation requires an
independent signal — either a different model, a deterministic checker, or ground truth.

### The Fix
Replace self-reflection with external validation signals:
- **Deterministic validators**: syntax checkers, type checkers, build tools
- **Task-specific evaluators**: unit tests, integration tests, expected output comparisons
- **Tool-grounded verification**: if the agent claims to have written a file, check that the file exists
  and parses correctly
- **Independent model**: if you must use an LLM to evaluate, use a different model (or at least
  different context)

**Brainwires reference:** `ValidationLoop` in `src/agents/task_agent.rs:500-608` uses external
validators (`verify_build`, `check_duplicates`, `check_syntax`) rather than model self-evaluation.
Bug #5 fix: file existence check before accepting "task complete" signal.

---

## Anti-Pattern 10: Building Fully Autonomous Systems First

### The Smell
- "We'll automate the entire X workflow from the start"
- Minimal human checkpoints: "we want zero human involvement"
- Broad tool access from day one: agent has access to production systems
- Success criteria: "if it works 80% of the time, that's good enough"
- "We'll add safety checks once it's working"

### What Happens
- The 20% failure rate hits irreversible, high-impact actions (not just annoying failures)
- Edge cases discovered in production, not in testing
- System shutdown after a single high-visibility failure
- Trust destroyed with stakeholders who had no visibility into the autonomy level
- Retrofitting safety is 10× harder than designing for it

### Root Cause
Reliability is a spectrum. A system that works 95% of the time with full autonomy is unreliable
in proportion to the risk of that 5%. The right reliability target depends on the consequences of
failure — which is a function of autonomy scope, not just success rate.

### The Fix
Earn autonomy incrementally:
1. **Start fully supervised**: agent proposes, human approves every action
2. **Automate safe actions**: whitelist low-risk, reversible actions for auto-approval
3. **Measure failure modes**: catalog every failure type with frequency and severity
4. **Expand gradually**: promote action categories to autonomous as reliability is proven
5. **Keep irreversible gates**: some actions should always require human approval

**Brainwires reference:** `PermissionMode::Auto`, `::Ask`, `::Reject` in
`src/agents/task_agent.rs:72`; `PolicyEngine` in `brainwires-permissions/src/policy.rs`
implements declarative rules for approval gates.

---

## Anti-Pattern 11: Ignoring Organizational Feedback Loops

### The Smell
- No mechanism to capture user feedback on agent outputs
- Failures are not labeled or categorized
- No comparison between model versions after upgrade
- "We'll look at metrics later"
- Agent system is treated as a black box after deployment

### What Happens
- The same class of bugs recurs because no one is tracking failure modes
- Model upgrades produce behavior regressions that go undetected for weeks
- Teams lose institutional knowledge of which prompts, configs, and patterns work
- Quality silently decays as retrieval drift and memory accumulation compound
- No data to justify improvements or reject changes

### Root Cause
Machine learning systems require feedback loops to maintain quality. Without telemetry and failure
labeling, the system's behavior model is purely hypothetical. In production, all models are wrong;
the question is whether you have enough feedback to know which failures matter.

### The Fix
Build telemetry-driven iteration into the development process:
- Log every agent run with structured metadata (task, duration, steps, tools used, outcome)
- Capture user feedback signals (thumbs up/down, explicit corrections)
- Label failures by category (planning failure, tool misuse, memory corruption, hallucination)
- Run A/B experiments for model upgrades and prompt changes
- Set quality thresholds that trigger alerts when success rates drop below baseline

**Brainwires reference:** `AuditLogger` in `brainwires-permissions/src/audit.rs` captures tool
execution events. Success rate tracking and feedback collection pipeline are current gaps.
