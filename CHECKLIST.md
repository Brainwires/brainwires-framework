# Brainwires Framework — Master Work Checklist

All items are derived from the gap analysis in `research/06-research-to-production-mapping.md`.
Each checkbox maps to a named production pattern from the research corpus.

BE SURE TO ADD NEW TESTS!

Priority definitions:
- **High** — production reliability risk; address before scaling to production load
- **Medium** — production quality improvement; implement in next major iteration
- **Low** — production excellence; implement after core gaps are closed

---

## Phase 1 — Core Reliability
> **Priority: HIGH** — These gaps are pre-production blockers.

- [x] **Execution DAG** — Add `ExecutionGraph` + `StepNode` structs to `brainwires-agents` or `brainwires-core`; emit from `TaskAgent.execute()` alongside `TaskAgentResult`; store in LanceDB collection for indexed search. *(Research: DSPy, Graph of Thoughts — doc 06 §1)*

- [x] **Loop detection** — In `TaskAgent.execute()`, track the last N tool calls; if N consecutive calls share the same tool name and similar argument structure, inject forced-exit feedback and abort. *(Research: Tree of Thoughts — doc 06 §4)*

- [x] **Input sanitization** — Strip prompt injection patterns from external content (web fetch results, `ContextRecallTool` output) before injecting into agent context; add `ContentSource` enum (SystemPrompt/UserInput/AgentReasoning/ExternalContent). *(Research: Indirect Prompt Injection, HouYi — doc 06 §7)*

- [x] **Instruction hierarchy enforcement** — Tag every context entry with its `ContentSource` priority level; enforce that lower-priority sources cannot override higher-priority instructions in the context builder. *(Research: HouYi — doc 06 §7)*

- [x] **Semantic tool validator** — Add pluggable pre-execution hook to `ToolExecutor`; validates tool call intent against current agent state, not just JSON schema structure. *(Research: API-Bank, Gorilla — doc 06 §2)*

- [x] **Automated Monte Carlo evaluation** — Create `brainwires-eval` crate with `EvaluationSuite` N-trial runner, success rate + confidence interval computation, and `cargo test` integration. *(Research: HELM — doc 06 §5)*

---

## Phase 2 — Tool Contract Hardening
> **Priority: MEDIUM**

- [x] **Tool idempotency keys** — Add idempotency key tracking to `FileOpsTool` write operations; assign a key before execution so retries are safe without duplication. *(Research: API-Bank — doc 06 §2)*

- [x] **Side-effect staging** — Implement two-phase commit for reversible write operations: stage the write → validate → commit or rollback on failure. *(Research: API-Bank — doc 06 §2)*

- [x] **Tool sequence recorder** — Record the ordered sequence of tool calls per run; compare against expected sequences in behavioral tests. *(Research: HELM — doc 06 §5)*

---

## Phase 3 — Memory Authority Hierarchy
> **Priority: MEDIUM**

- [x] **Canonical memory tier** — Define `MemoryAuthority` enum (Ephemeral/Session/Canonical); add authority field to stored memory entries; enforce that only authorized sources can write to the Canonical tier. *(Research: MemGPT — doc 06 §3)*

- [x] **Memory poisoning detection** — In `EntityStore`, detect when two entries assert contradicting facts for the same entity; flag for human review rather than silently overwriting. *(Research: MemGPT — doc 06 §3)*

- [x] **TTL policies** — Add expiry timestamps to session-tier `MessageStore` entries; auto-evict on run completion or TTL expiry. *(Research: MemGPT — doc 06 §3)*

- [x] **Multi-factor retrieval** — Replace pure similarity search with combined recency + importance + relevance scoring for memory retrieval. *(Research: Generative Agents — doc 06 §3)*

---

## Phase 4 — Planning Stability
> **Priority: MEDIUM**

- [x] **Goal re-validation every N steps** — Add configurable `goal_revalidation_interval: Option<usize>` to `TaskAgentConfig`; every N steps, compare current execution state against the original goal and inject drift-detection feedback if misaligned. *(Research: Plan-and-Solve — doc 06 §4)*

- [x] **Serializable plan with budget estimation** — Before execution, have the agent produce a serializable plan that estimates step count and token budget; reject plans that exceed configured budgets before any side effects occur. *(Research: Tree of Thoughts, Plan-and-Solve — doc 06 §4)*

- [x] **Distinct REPLAN state** — Add `Replanning` as an explicit `TaskAgentStatus` variant; track `replan_count` separately from `iteration_count`; add `max_replan_attempts` to `TaskAgentConfig`. *(Research: Plan-and-Solve — doc 06 §4)*

---

## Phase 5 — Multi-Agent Coordination Hardening
> **Priority: MEDIUM**

- [x] **Per-agent file scope whitelist** — Add `allowed_files: Option<Vec<PathBuf>>` to `TaskAgentConfig`; enforce in `FileLockManager` so agents cannot request locks on files outside their assigned scope. *(Research: CAMEL, MetaGPT — doc 06 §6)*

- [ ] **Validator agent type** — Implement `ValidatorAgent` as a distinct agent type that holds only read locks, runs external validators (`verify_build`, `check_syntax`, `check_duplicates`), and returns a structured `ValidationResult` to the orchestrator. *(Research: MetaGPT — doc 06 §6)*

- [ ] **Orchestrator ↔ TaskManager integration** — Wire `OrchestratorAgent` subtask assignment through `TaskManager`'s dependency graph so task ordering and status are tracked centrally. *(Research: AutoGen, MetaGPT — doc 06 §6)*

---

## Phase 6 — Runtime Budgets & Cost Control
> **Priority: MEDIUM**

- [x] **Per-run token budget** — Add `max_total_tokens: Option<u64>` to `TaskAgentConfig`; accumulate input + output tokens across the full `execute()` loop; abort with partial results when approaching the ceiling. *(Research: FrugalGPT — doc 06 §8)*

- [x] **Per-run cost ceiling** — Add `max_cost_usd: Option<f64>` to `TaskAgentConfig`; track accumulated cost against `MdapMetrics.actual_cost_usd`; abort when ceiling is reached. *(Research: FrugalGPT — doc 06 §8)*

- [x] **Timeout ceiling** — Enforce a wall-clock timeout per run; return partial results with a `timed_out: bool` signal on expiry rather than aborting silently. *(Research: FrugalGPT — doc 06 §8)*

- [x] **Partial result return on budget exhaustion** — Extend `TaskAgentResult` with `budget_exhausted: bool` and `partial_output: Option<String>`; return best available work when any budget ceiling is hit. *(Research: FrugalGPT — doc 06 §8)*

---

## Phase 7 — Security Hardening
> **Priority: MEDIUM**

- [ ] **Sandboxed bash execution** — Run bash tool commands in an isolated subprocess: restricted env vars, no network access unless explicitly permitted, filesystem scope limited to working directory. *(Research: Formalizing Attacks and Defenses — doc 06 §7)*

- [x] **Output filtering** — Before injecting tool results into agent context, scan for sensitive data patterns (API keys, tokens, PII); redact or reject. *(Research: Indirect Prompt Injection — doc 06 §7)*

- [x] **Anomaly detection** — In `AuditLogger`, flag unusual patterns: repeated policy violations, high-frequency tool calls, unusual file scope requests; surface to operator. *(Research: Formalizing Attacks and Defenses — doc 06 §7)*

---

## Phase 8 — Observability & Telemetry
> **Priority: MEDIUM**

- [x] **Structured run telemetry** — Emit a structured record for every completed agent run (task type, duration, step count, tools used, outcome); store in LanceDB for queryable history. *(Anti-pattern 8, Anti-pattern 11 — doc 02)*

- [x] **User feedback capture** — Add a feedback signal API (thumbs up/down, explicit corrections); associate feedback with run ID in storage for correlation analysis. *(Anti-pattern 11 — doc 02)*

- [x] **Failure categorization** — Label each `TaskAgentStatus::Failed` with a failure type (planning_failure, tool_misuse, memory_corruption, hallucination, budget_exhausted); enable trend queries. *(Anti-pattern 11 — doc 02)*

- [x] **Prompt hash snapshotting** — At run start, compute SHA256 of the system prompt + tool registry; store as a "build ID" field in `ExecutionGraph` and run telemetry. *(Principle 4, Principle 5 — doc 03)*

---

## Phase 9 — Testing Infrastructure
> **Priority: HIGH** — Enables measuring the impact of every other phase.

- [x] **`brainwires-eval` crate** — Standalone evaluation framework crate with `EvaluationSuite` (N-trial runner), `ToolSequenceRecorder` (call sequence capture + diff), and `AdversarialTestCase` (injection, ambiguity, budget stress). *(Research: HELM — doc 06 §5)*

- [x] **Adversarial test suite** — Test cases: prompt injection via tool outputs, ambiguous instructions with multiple valid interpretations, missing required context, tasks designed to exhaust step budget. *(Research: HELM — doc 06 §5)*

- [x] **Long-horizon stability tests** — Tasks requiring 15+ steps; assertions: loop detection fires correctly, original goal maintained throughout, memory retrieval quality stable across steps. *(Research: HELM — doc 06 §5)*

- [x] **Regression suite for CI** — Baseline success rates per task category; CI fails if any category drops more than 5% below baseline; runs on every prompt or model version change. *(Research: HELM — doc 06 §5)*

- [x] **Statistical confidence intervals** — Report `P(success) ± 2σ` per task type across N ≥ 30 trials; never report binary pass/fail for stochastic evaluation. *(Research: HELM — doc 06 §5)*

---

## Phase 10 — Production Excellence
> **Priority: LOW** — Implement after core gaps are closed.

- [ ] **Dynamic model routing** — FrugalGPT-style: estimate task complexity → route to haiku/sonnet/opus class model; target 60–80% cost reduction by routing ~70% of tasks to cheaper models. *(Research: FrugalGPT — doc 06 §8)*

- [ ] **Token compression pipeline** — Before sending to model: summarize conversation history beyond N turns, compress tool results to key fields only, truncate repetitive context. *(Research: Efficient Prompting Survey — doc 06 §8)*

- [ ] **Prompt versioning with semantic IDs** — `PromptVersion` struct with semantic identifier + hash; snapshot exact prompt text with every run; run evaluation suite before promoting prompt changes. *(Anti-pattern 2, Principle 4 — doc 02, doc 03)*

- [ ] **Full replay framework** — Deterministic seed from run ID; store frozen model version, tool registry hash, exact tool I/O; replay from `ExecutionGraph` + mocked tool outputs produces identical decisions. *(Principle 12 — doc 03)*

- [ ] **A/B experiments** — Compare model upgrade / prompt change pairs; compute success rate diff with statistical significance test; require significance before promoting changes. *(Anti-pattern 11 — doc 02)*

---

## Phase 11 — Framework Extraction Completion
> **Priority: LOW** — Architectural completeness.

- [ ] **Verify `brainwires-wasm`** — Audit WASM bindings for all core types; ensure browser target builds succeed with `wasm-pack`; run basic WASM smoke tests.

- [ ] **Complete `brainwires-seal`** — Wire SEAL (Self-Evolving Agentic Learning) integration through `brainwires-knowledge` + `brainwires-prompting`; implement the learning loop that reads `AuditLogger` feedback to improve prompting strategies over time.

- [ ] **`brainwires-eval` as standalone crate** — After Phase 9 builds it internally, extract as a publishable crate usable by projects outside the CLI.

- [ ] **CLI thin-wrapper audit** — Review every module in `src/` against its framework counterpart; confirm each is a genuine thin wrapper with no duplicated logic; document any intentional divergences.

---

## Quick Reference: Priority Matrix

| Phase | Items | Priority | Unblocks |
|-------|-------|----------|---------|
| 1 — Core Reliability | 6 | **High** | Everything |
| 9 — Testing Infrastructure | 5 | **High** | Measuring all other phases |
| 2 — Tool Contracts | 3 | Medium | Tool reliability |
| 3 — Memory Authority | 4 | Medium | Knowledge reliability |
| 4 — Planning Stability | 3 | Medium | Long-horizon tasks |
| 5 — Multi-Agent | 3 | Medium | Concurrent agent workflows |
| 6 — Budgets & Cost | 4 | Medium | Cost predictability |
| 7 — Security | 3 | Medium | External content safety |
| 8 — Observability | 4 | Medium | Debugging & iteration |
| 10 — Production Excellence | 5 | Low | Cost optimization |
| 11 — Framework Extraction | 4 | Low | Architectural completeness |

**Recommended start order:** Phase 1 → Phase 9 → Phases 2–8 (any order) → Phases 10–11
