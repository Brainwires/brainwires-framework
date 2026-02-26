# Research to Production Implementation Mapping

## How to Use This Document

Each section follows the pattern:

```
Problem → Research Insight → Production Pattern → Brainwires Implementation Status → Gap / Next Step
```

Use this document when:
- **Implementing a new feature**: check what patterns research recommends and whether Brainwires
  already implements them
- **Reviewing a PR**: verify the implementation aligns with established production patterns
- **Diagnosing a production failure**: map the failure type to its research category and find
  the recommended fix
- **Planning roadmap items**: use the Gap column to identify highest-value missing implementations

---

## 1. Non-Determinism and Debugging

### Research Insights
- **DSPy** (2310.03714): Treating LM pipelines as compilable programs enables optimization and
  analysis that string prompts cannot support. Execution should be a program, not free-form text.
- **Graph of Thoughts** (2308.09687): Execution as a DAG (not linear log) enables aggregation,
  inspection, and partial re-execution. Cycles and orphan nodes are detectable bugs.
- **Language Model Cascades** (2207.10342): Provides formal framework for compositional LM call
  chains. Each call is a primitive; the composition is a program.

### Production Pattern

```rust
// Every run produces:
ExecutionGraph {
    run_id: Uuid,
    model_id: String,       // include patch version
    model_version: String,
    prompt_hash: [u8; 32],  // SHA256 of system prompt
    tool_registry_hash: [u8; 32],
    temperature: f32,
    seed: Option<u64>,
    start_time: DateTime<Utc>,
    nodes: Vec<StepNode>,
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_cost_usd: f64,
}

// Each step:
StepNode {
    id: u32,
    step_type: Think | ToolCall | Validation | Retrieval | Completion,
    parent_ids: Vec<u32>,   // DAG edges
    input_tokens: u32,
    output_tokens: u32,
    latency_ms: u64,
    tool_name: Option<String>,
    tool_args_hash: Option<[u8; 32]>,
    tool_result_hash: Option<[u8; 32]>,
    validator_passed: Option<bool>,
    error: Option<String>,
}

// Replay: given ExecutionGraph + mocked tool outputs + frozen model = identical decisions
```

### Brainwires Implementation Status

**Implemented:**
- `MdapMetrics` + `SubtaskMetric` in `brainwires-mdap/src/metrics.rs` — per-subtask tracking
  including tokens, latency, votes, and confidence
- `AuditEvent` + `AuditLogger` in `brainwires-permissions/src/audit.rs` — tool execution events
  with agent, action, target, and outcome
- `TaskAgentConfig.temperature` — temperature tracked per agent config
- `TaskAgentResult.iterations` — iteration count per run

**Gaps:**
- No per-run execution DAG for non-MDAP task agents
- No prompt hash per run (system prompt is static, but not hashed and snapshotted)
- No deterministic seed support (temperature=0 achieves near-determinism but not full replay)
- No tool argument or result hashing for replay verification
- Replay framework not implemented

**Next Step:** Add `ExecutionGraph` struct to `brainwires-agents` or `brainwires-core`. Emit from
`TaskAgent.execute()` loop alongside existing `TaskAgentResult`. Store in same LanceDB collection
as messages for indexed search.

---

## 2. Tool Contracts

### Research Insights
- **API-Bank** (2304.08244): Quantifies tool call failure rates as 12–30% even with structured
  outputs. Failure modes: wrong API selection (30%), wrong parameter values (45%), wrong order (25%).
- **Gorilla** (2305.15334): Fine-tuning improves mean accuracy but doesn't eliminate tail failures.
  Retrieval-augmented tool documentation reduces hallucination.
- **Outlines** (2307.09702): Grammar-constrained decoding provides hard structural guarantees at
  minimal overhead. Moves validation into generation rather than post-processing.

### Production Pattern

```
Tool Call Pipeline:

LLM generates JSON → [1] Schema Validator → [2] Semantic Validator → [3] Capability Check
                             ↓ fail                ↓ fail                   ↓ fail
                         reject fast            reject fast               reject + audit
                             ↓ pass               ↓ pass                   ↓ pass
                        [4] Idempotency Check → [5] Side-Effect Staging → [6] Executor
                             ↓                       ↓
                         assign key             stage + review
                                                    ↓ commit
                                             [7] Working Set Update
                                             [8] Audit Log
```

**Treat every tool invocation as an untrusted remote procedure call.**

Key requirements:
- Schema validation fails fast before execution (no partial execution on malformed input)
- Idempotency keys assigned before write operations (safe retry without duplication)
- Capability scoping: each agent session has a whitelist of permitted tool+target combinations
- Side effects staged for reversible actions (commit on validation, rollback on failure)

### Brainwires Implementation Status

**Implemented:**
- `ToolExecutor` in `src/tools/executor.rs` — validates and dispatches tool calls
- `PolicyEngine` + `PolicyRequest` in `brainwires-permissions/src/policy.rs` — declarative
  permission rules evaluated before execution
- `AuditLogger` — every tool execution logged with `ActionOutcome`
- `FileLockManager.acquire_lock()` — exclusive write locks prevent concurrent corruption
- `PermissionMode` (Auto/Ask/Reject) — session-level capability gating
- `approval_tx` channel — interactive approval for `PermissionMode::Ask`

**Gaps:**
- No idempotency keys on write operations (retrying a write may duplicate)
- No side-effect staging / two-phase commit for reversible write operations
- Semantic validation layer (beyond schema) not formalized
- Tool documentation retrieval (Gorilla-style) not implemented

**Next Step:** Add idempotency key tracking to `FileOpsTool` write operations. Design semantic
validation interface in `ToolExecutor` as a pluggable pre-execution hook.

---

## 3. Memory Architecture

### Research Insights
- **MemGPT** (2310.08560): Context window = RAM. External store = disk. Agent explicitly manages
  paging between tiers. Key insight: the agent should be the memory manager, not just the consumer.
- **Generative Agents** (2304.03442): Multi-factor retrieval (recency + importance + relevance)
  produces more consistent behavior than pure similarity search.
- **Self-RAG** (2310.11511): Selective retrieval (retrieve only when needed) with confidence scoring
  outperforms always-retrieve. Irrelevant context injection degrades quality.

### Production Pattern

| Tier | Scope | Authority | Write Access | Eviction |
|------|-------|-----------|-------------|---------|
| Ephemeral | Per-step | Lowest | Any agent | Immediate (context flush) |
| Session | Per-run | Medium | Any agent in session | On run completion |
| Canonical | Global | Highest | Authorized sources only | Never (manual only) |

**Governance requirements:**
```
Memory Write Decision Tree:
  Is this a confirmed fact from an authoritative external source?
    YES → write to Canonical tier
  Is this a task-relevant result for the current session?
    YES → write to Session tier with TTL = session duration
  Is this needed only for the current step?
    YES → ephemeral (context window only, never stored)

Memory Read Decision Tree:
  Retrieve from all tiers by similarity
  Filter by confidence threshold (min_score ≥ 0.75)
  Resolve conflicts: Canonical > Session > Ephemeral
  Check for poisoning signals: contradicting facts with high confidence in both
    → escalate to human review
```

### Brainwires Implementation Status

**Implemented:**
- `TieredMemory` (hot/warm/cold) in `brainwires-storage` — three-tier persistence
- `MessageStore` with LanceDB — vector search across session history
- `WorkingSet` in `brainwires-core/src/working_set.rs` — ephemeral file tracking per task
- Entity extraction + relationship graph in `brainwires-knowledge` — semantic context injection
- Retrieval confidence threshold (`min_score` in query operations)

**Gaps:**
- No canonical tier with write-authority enforcement (current hot/warm/cold = recency tiers,
  not authority tiers)
- No memory poisoning detection (conflicting facts not flagged)
- No canonical override rule enforced in retrieval
- TTL policies not implemented on session-tier memories
- Multi-factor retrieval (recency + importance + relevance) not implemented (pure similarity only)

**Next Step:** Define `MemoryAuthority` enum (Ephemeral/Session/Canonical) and add authority field
to stored memory entries. Implement conflict detection in `EntityStore` when two entries have
contradicting facts for the same entity. Add TTL support to session-tier `MessageStore` entries.

---

## 4. Planning Instability

### Research Insights
- **Tree of Thoughts** (2305.10601): Bounded search with external scoring significantly reduces
  planning failures. Depth must be explicitly bounded by orchestrator, not model.
- **Plan-and-Solve Prompting** (2305.04091): Explicit plan-then-execute structure at the prompt
  level reduces planning errors before any infrastructure changes.
- **ReAct** (2210.03629): Interleaved reasoning + acting reduces hallucination and error propagation
  compared to acting without reasoning traces.
- **Reflexion** (2303.11366): Multi-trial learning with external feedback can improve planning
  reliability across sessions.

### Production Pattern

**State machine for planning and execution:**

```
States: PLANNING → VALIDATING_PLAN → EXECUTING → VALIDATING_RESULT → COMPLETING | REPLANNING

PLANNING:
  - LLM generates plan (serializable, inspectable)
  - Plan estimates step count and token budget
  - If estimates exceed budget → reject plan, regenerate

VALIDATING_PLAN:
  - Check plan for obvious loops (step i depends on step j depends on step i)
  - Check plan feasibility (required tools available?)
  - Check plan against budget
  - If invalid → return to PLANNING with feedback

EXECUTING:
  - Execute plan steps sequentially or in parallel
  - Track which steps completed vs. failed
  - Re-validate goal every N steps (detect drift)
  - If step budget exhausted → abort, return partial results

VALIDATING_RESULT:
  - External validation (build, tests, file existence)
  - If validation fails → REPLANNING (not COMPLETING)
  - max_replan_attempts prevents infinite replan loops

COMPLETING:
  - All validation passed
  - Run complete, emit ExecutionGraph
```

**Loop detection:**
```rust
// In execution loop:
let recent_tool_calls = get_last_n_tool_calls(history, 5);
if recent_tool_calls.all_same_tool_and_similar_args() {
    inject_feedback("Loop detected: same tool called repeatedly without progress.
                    Consider a different approach.");
}
```

### Brainwires Implementation Status

**Implemented:**
- `TaskAgentConfig.max_iterations` — step budget enforced by orchestrator
- `attempt_validated_completion()` in `task_agent.rs:500` — validates before accepting completion
- `ValidationConfig.checks` — configurable validation gates
- Validation feedback injection — on failure, adds feedback to conversation history
- `brainwires-core/src/plan.rs` — `Plan` type with steps and dependencies

**Gaps:**
- Loop detection not implemented (same tool called repeatedly is not detected)
- Goal re-validation every N steps not implemented (drift detection)
- Serializable plan with budget estimation not implemented
- REPLAN state is not distinct from continuation (same loop, agent may or may not replan)
- Maximum replan attempts not tracked separately from max_iterations

**Next Step:** Add loop detection to `TaskAgent.execute()` — track recent tool call history,
inject loop detection feedback if N consecutive calls share tool name and similar argument structure.

---

## 5. Testing and Evaluation

### Research Insights
- **HELM** (2211.09110): Multi-dimensional distributional evaluation across N scenarios. Reliability
  is distributional, not binary. Confidence intervals required, not point estimates.
- **LLM-as-a-Judge** (2306.05685): LLM judges achieve 80% agreement with humans but have systematic
  biases (position, verbosity, self-enhancement). Useful for screening; not ground truth.
- **On the Reliability of Agents** (2407.01051): P(k steps correct) = p^k. Per-step reliability
  degrades multiplicatively. Validation gates reset accumulated error probability.

### Production Pattern

**Behavioral evaluation framework:**

```
Evaluation Suite:
├── Unit behavioral tests: N=30 trials per task type
│   Success = P(goal completion) > threshold (e.g., 0.95)
│   Metric: success rate with confidence interval [p - 2σ, p + 2σ]
│
├── Tool sequence validation
│   For each successful run: record tool call sequence
│   Expected sequence: defined in test specification
│   Match: exact (for deterministic tasks) or pattern (for flexible tasks)
│
├── Adversarial suite
│   Prompt injection attempts via tool outputs
│   Ambiguous instructions that could be interpreted multiple ways
│   Missing required context (what does the agent do when it doesn't know?)
│   Budget stress tests (tasks designed to exhaust step budget)
│
├── Long-horizon stability
│   Tasks requiring 15+ steps
│   Check: loop detection fires correctly
│   Check: goal is maintained across 15+ steps
│   Check: memory retrieval quality doesn't degrade
│
└── Regression suite
    Run on every prompt or model version change
    Baseline success rates per task category
    Fail if any category drops below baseline by > 5%
```

### Brainwires Implementation Status

**Implemented:**
- `test-results/` manual test archive — 42 tests with star ratings, 95% success rate baseline
- `ValidationLoop` — automated behavioral gate at completion time
- Progressive difficulty test methodology (Levels 1–7) documented in `CLAUDE.md`
- Bug discovery and fix workflow documented

**Gaps:**
- Automated Monte Carlo evaluation framework not implemented
- Tool sequence recording and comparison not automated
- Adversarial prompt injection test suite not implemented
- Regression test suite not automated (currently manual)
- Statistical confidence intervals not computed for success rates

**Next Step:** Create `test-framework/` crate with:
- `EvaluationSuite` — N-trial runner with success rate + confidence interval computation
- `ToolSequenceRecorder` — records and compares tool call sequences
- `AdversarialTestCase` — prompt injection, ambiguity, and budget stress tests
- Integration with `cargo test` for CI execution

---

## 6. Multi-Agent Coordination

### Research Insights
- **AutoGen** (2308.08155): Flexible multi-agent conversation enables complex tasks but requires
  explicit termination conditions and token budgets to be production-safe.
- **MetaGPT** (2308.00352): Standardized Operating Procedures (role-specific deliverables and
  workflows) dramatically reduce hallucination and inter-agent inconsistency.
- **CAMEL** (2303.17760): Role-playing with explicit role assignment reduces task drift. Role
  clarity is more important than model capability for multi-agent reliability.

### Production Pattern

**Single-orchestrator coordination model:**

```
Orchestrator
│   ├── Owns task decomposition
│   ├── Assigns subtasks to worker agents
│   ├── Tracks worker status via CommunicationHub
│   ├── Resolves conflicts (FileLockManager arbitration)
│   └── Aggregates results into final output
│
├── Worker Agent 1
│   ├── Scoped tool set (only tools needed for assigned subtask)
│   ├── Scoped file access (only files relevant to subtask)
│   └── Reports status and results to orchestrator
│
├── Worker Agent 2
│   └── [same constraints]
│
└── Validator Agent (optional)
    ├── Reads-only (no write operations)
    ├── Runs external validators (build, tests)
    └── Returns structured validation results to orchestrator
```

**Required coordination contracts:**
1. Every agent run has a defined exit condition before it starts
2. Agents cannot write to files outside their assigned scope
3. Conflicts resolved by lock acquisition order (deterministic, not negotiated)
4. Orchestrator is single point of coordination (no peer-to-peer agent communication)

### Brainwires Implementation Status

**Implemented:**
- `CommunicationHub` — typed `AgentMessage` broadcast + receive
- `FileLockManager` — read/write locks with deterministic acquisition order
- `OrchestratorAgent` in `src/agents/orchestrator.rs` — single-orchestrator pattern
- `AgentPool` in `src/agents/pool.rs` — lifecycle management for worker agents
- `TaskAgentConfig.permission_mode` — per-agent capability scoping
- `ThreeStateModel` in `brainwires-agents/src/state_model.rs` — distributed state tracking

**Gaps:**
- Per-agent file scope whitelist not enforced (agents can request any file lock)
- Validator agent role not formalized as a distinct agent type
- Orchestrator subtask assignment not integrated with `TaskManager`

**Next Step:** Add file scope whitelist to `TaskAgentConfig`. Implement `ValidatorAgent` as a
distinct type that only holds read locks and runs external validators.

---

## 7. Security

### Research Insights
- **Indirect Prompt Injection** (2302.12173): External content retrieved by agents (web pages,
  documents, emails) is a primary attack vector. Adversarial instructions in retrieved content
  hijack agent behavior.
- **HouYi Attack** (2306.05499): 86% of tested LLM-integrated applications were vulnerable.
  Defense requires instruction hierarchy enforcement, not just input filtering.
- **Formalizing Attacks and Defenses** (2310.12815): No single defense is sufficient. Defense-in-
  depth required. Current defenses are probabilistic mitigations, not guarantees.

### Production Pattern

**Defense-in-depth security pipeline:**

```
Input Pipeline:
External content → [1] Content Sanitizer → [2] Instruction Hierarchy Tagger
                        (strip injection patterns)    (mark as EXTERNAL_CONTENT, lower priority)
                                                ↓
                                  LLM Context (with priority markers)

Execution Pipeline:
LLM decision → [3] Capability Scope Check → [4] Target Validation
(tool call)       (is this in whitelist?)     (is target in allowed scope?)
                      ↓ fail                      ↓ fail
                   deny + audit               deny + audit
                      ↓ pass                      ↓ pass
               [5] Sandboxed Execution → [6] Output Filter
               (isolated process/dir)    (no sensitive data in result)

Audit Pipeline:
All events → AuditLogger → Anomaly Detector
              (structured)   (flag unusual patterns)
```

**Instruction hierarchy:**
```
Priority 1 (highest): System prompt (compiled, versioned)
Priority 2: User instructions (session)
Priority 3: Agent decisions (runtime)
Priority 4 (lowest): External content (retrieved documents, tool outputs)

Rule: Priority N instructions cannot override Priority M instructions where M < N
```

### Brainwires Implementation Status

**Implemented:**
- `PermissionMode` (Auto/Ask/Reject) — session-level capability gating
- `PolicyEngine` + `PolicyRequest` — declarative permission rules
- `AuditLogger` — tool executions logged with agent, action, target, outcome
- `TrustLevel` enum in `brainwires-permissions/src/trust.rs` — trust level tracking
- `AuditEventType::PolicyViolation` — policy violations audited

**Gaps:**
- Input sanitization layer (strip injection patterns from external content) not implemented
- Instruction hierarchy enforcement not formalized (no priority tagging for context sources)
- Sandboxed execution for bash commands (subprocess isolation) not implemented
- Output filtering for sensitive data not implemented
- Anomaly detection for unusual audit patterns not implemented

**Next Step:** Implement input sanitization in `ContextRecallTool` and web fetch results before
injection into agent context. Add `ContentSource` enum (SystemPrompt/UserInput/AgentReasoning/
ExternalContent) to context injection metadata.

---

## 8. Cost and Latency

### Research Insights
- **FrugalGPT** (2305.05176): Model cascading achieves 98% cost reduction vs. always using the
  most capable model. Route by estimated task complexity, not by preference.
- **Speculative Decoding** (2211.17192): 2–3× latency reduction for local model inference.
- **Efficient Prompting Survey** (2310.01382): 30–70% token reduction achievable with <5% quality
  loss through systematic compression techniques.

### Production Pattern

**Multi-level cost control:**

```
Budget Hierarchy:
  Project budget (monthly): $X total
    ↓
  Workflow budget (per task type): $Y max per run
    ↓
  Step budget: max_iterations = N
    ↓
  Per-call budget: max_tokens = M

Enforcement:
  1. At planning time: estimate total budget, reject if > workflow budget
  2. At each step: track accumulated tokens/cost, abort if > workflow budget - buffer
  3. At each call: hard max_tokens limit
  4. On timeout: return partial results with budget_exhausted signal

Model Routing:
  Complexity estimate → Model selection
  Low complexity (simple Q&A, lookup): cheap model (haiku-class)
  Medium complexity (code review, analysis): mid-tier model (sonnet-class)
  High complexity (architecture, novel problem): top model (opus-class)

  Cost ratio: ~1:6:18 (haiku:sonnet:opus class)
  Route 70% of tasks to cheap model → 60-80% cost reduction
```

**Token compression (before sending to model):**
- Summarize conversation history beyond N turns
- Compress tool results to key fields only
- Truncate repetitive context
- Use semantic chunking for retrieved documents

### Brainwires Implementation Status

**Implemented:**
- `TaskAgentConfig.max_tokens` (4096 default) — per-call token limit
- `TaskAgentConfig.max_iterations` — step budget
- `MdapConfig.max_samples_per_subtask` — MDAP sampling budget
- `MdapMetrics.actual_cost_usd` — cost tracking
- `MdapMetrics.total_input_tokens` + `total_output_tokens` — token tracking
- `TieredMemory` — conversation history management (prevents unbounded growth)

**Gaps:**
- No per-run total token budget (only per-call)
- No cost ceiling per workflow
- No dynamic model routing by task complexity
- No token compression pipeline for history beyond context window
- No partial result return on budget exhaustion (current: hard abort)
- Timeout ceiling not enforced

**Next Step:** Add `max_total_tokens: Option<u64>` and `max_cost_usd: Option<f64>` to
`TaskAgentConfig`. Track accumulated tokens in `TaskAgent.execute()` loop and abort with
partial results when budget approaches ceiling.

---

## Implementation Gap Analysis Summary

| Challenge | Key Research | Implemented | Gap | Priority |
|-----------|-------------|-------------|-----|---------|
| Execution observability | DSPy, GoT | MdapMetrics, AuditLogger | Execution DAG, replay | High |
| Tool structural validation | Outlines | ToolExecutor schema | Constrained decoding | Medium |
| Tool semantic validation | API-Bank | PolicyEngine | Semantic validator layer | High |
| Tool idempotency | API-Bank | — | Idempotency keys on writes | Medium |
| Memory tiering | MemGPT | TieredMemory (hot/warm/cold) | Authority hierarchy, canonical tier | High |
| Memory poisoning | — | — | Conflict detection, TTL | Medium |
| Selective retrieval | Self-RAG | min_score threshold | Retrieve-on-demand, importance scoring | Low |
| Loop detection | ToT | max_iterations | Loop pattern detection | High |
| Goal re-validation | Plan-and-Solve | ValidationLoop | N-step goal check | Medium |
| N-run evaluation | HELM | test-results/ manual | Automated Monte Carlo | High |
| Tool sequence testing | — | — | Automated sequence recorder | Medium |
| Adversarial testing | — | — | Prompt injection test suite | Medium |
| Per-agent scope | CAMEL, MetaGPT | PermissionMode | File scope whitelist | Medium |
| Input sanitization | Indirect Injection | — | Content sanitization layer | High |
| Instruction hierarchy | HouYi | — | Priority tagging for context sources | High |
| Sandboxed execution | — | — | Process isolation for bash | Medium |
| Cost routing | FrugalGPT | max_tokens, max_iterations | Dynamic model routing | Low |
| Total cost ceiling | FrugalGPT | — | Per-run cost/token budget | Medium |
| Prompt versioning | DSPy | Static Rust constants | CI regression eval | Low |
| Replay determinism | GoT | — | Full replay framework | Low |

**Priority definitions:**
- **High**: Production reliability risk; should be addressed before scaling to production load
- **Medium**: Production quality improvement; implement in next major iteration
- **Low**: Production excellence; implement when core gaps are closed

**Highest-priority items for next sprint:**
1. **Execution DAG** — enables debugging of all other issues
2. **Input sanitization** — security requirement before any external content exposure
3. **Instruction hierarchy** — security requirement (prevents injection)
4. **Semantic tool validator** — reduces tool misuse failure rate
5. **Automated Monte Carlo evaluation** — enables measuring impact of all other improvements
6. **Loop detection** — prevents runaway costs from planning loops
