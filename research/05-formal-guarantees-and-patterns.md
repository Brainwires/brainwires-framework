# Formal Approaches and Cross-Paper Architectural Patterns

## Overview

This document synthesizes two things:
1. **Research papers that approach formal guarantees** — the closest the field has come to provable
   properties for agentic system components
2. **Architectural patterns that emerge across multiple papers** — convergent structures that appear
   independently in different research threads, signaling production-relevant universal patterns

Understanding both helps distinguish between:
- Engineering decisions with strong theoretical backing (constrained decoding, bounded search)
- Engineering decisions that are pragmatic best practices without formal proof (memory tiering,
  single-orchestrator pattern)
- Engineering gaps where production stability is achieved empirically, not theoretically (replay,
  memory poisoning detection, agent termination guarantees)

---

## Part 1: Papers Approaching Formal Guarantees

### 1.1 Constrained Decoding — Closest to Hard Structural Guarantees

#### Outlines: Efficient Guided Generation for Large Language Models
**arXiv:** [2307.09702](https://arxiv.org/abs/2307.09702) | **Year:** 2023

**What is guaranteed:**
Outlines reformulates generation as transitions in a finite-state machine (FSM). For a given
regular expression, context-free grammar, or JSON schema, the FSM is pre-compiled from the
vocabulary. During generation, only tokens that keep the FSM in a valid state are permitted.

This provides a **hard structural guarantee**: the generated output will match the specified grammar.
Every token generated is a valid next token in the grammar. Invalid structure is impossible.

**What is NOT guaranteed:**
- Semantic correctness (valid JSON schema ≠ semantically correct tool call)
- Correct parameter values (grammar allows any string for a string field)
- Intent alignment (correct structure ≠ intended action)

**Production implication:**
Move validation INTO the decoding process for structural properties. Eliminates an entire class
of retry loops (those triggered by malformed JSON or schema violations). Reduces but does not
eliminate tool call failures — semantic failures remain.

```
Current:  LLM generates → parse fails → retry
With Outlines: LLM generates only valid structures → parse always succeeds
```

**Current limitation:** Grammar-constrained decoding requires model to run locally or through
inference frameworks that support it (llama.cpp, vLLM). Not available through standard hosted APIs.

---

#### XGrammar: Flexible and Efficient Structured Generation
**arXiv:** [2411.15100](https://arxiv.org/abs/2411.15100) | **Year:** 2024

**What is guaranteed:**
Extends constrained generation to context-free grammars with adaptive token masking. Handles
recursive structures (nested JSON, Markdown) that pure FSM approaches struggle with.

**Production implication:**
2024 state of the art for structural output guarantees. More general than Outlines for complex
nested schemas. Relevant for Brainwires' tool call format enforcement.

---

### 1.2 Bounded Search — Structural Guarantees on Exploration

#### Tree of Thoughts: Deliberate Problem Solving with Large Language Models
**arXiv:** [2305.10601](https://arxiv.org/abs/2305.10601) | **Year:** 2023

**What is guaranteed:**
When depth (max tree depth) and branching factor (thoughts per step) are explicitly bounded, the
search space is finite and the computation terminates in bounded time and tokens.

**What is NOT guaranteed:**
- Correctness of the externally scored "best" path
- That the problem is solvable within the depth bound
- Quality of individual thoughts (still stochastic generation)

**Production implication:**
Bounded search provides **termination guarantees** when depth is explicit. The critical design
decision: who sets the depth bound? The model must NOT set its own depth bound (Anti-Pattern 6).
The orchestrator sets the bound; the model explores within it.

```
ToT with orchestrator-controlled depth:
  max_depth = 5 (set by orchestrator)
  branching = 3 (set by orchestrator)
  Total thoughts: ≤ 3^5 = 243 (worst case, finite)
  Total tokens: bounded
```

**Relation to Brainwires MDAP:**
MDAP's voting mechanism (`brainwires-mdap/src/voting.rs`) implements first-to-ahead-by-k voting,
which is a bounded parallel search across k samples. The bound (k) is set by `MdapConfig` — not
determined by the model.

---

#### Graph of Thoughts: Solving Elaborate Problems with Large Language Models
**arXiv:** [2308.09687](https://arxiv.org/abs/2308.09687) | **Year:** 2023

**What is guaranteed:**
DAG execution with explicit graph construction rules. The execution graph is a formal structure
whose properties (connectivity, acyclicity, depth) can be verified before and during execution.

**Production implication:**
Execution graphs are not just an observability technique — they are a correctness tool. A cycle in
the execution DAG is a detectable bug. An unbounded DAG is a detectable misconfiguration. Formalizing
execution as a graph makes structural invariants verifiable.

---

### 1.3 Tool Use Reliability — Probabilistic Bounds (Not Hard Guarantees)

#### API-Bank: Comprehensive Benchmark for Tool-Augmented LLMs
**arXiv:** [2304.08244](https://arxiv.org/abs/2304.08244) | **Year:** 2023

**What is established:**
Tool call failure rates are quantifiable and follow predictable patterns:
- Simple APIs (1–2 parameters): 12% failure rate
- Complex APIs (5+ parameters, nested types): 28–30% failure rate
- Failure modes: API selection (30%), parameter value (45%), call ordering (25%)

**What this means formally:**
Tool use reliability is a probability distribution, not a binary property. For a k-step workflow
with per-step reliability p:

```
P(k steps correct) = p^k
P(10 steps correct | p=0.88) = 0.88^10 = 0.28
P(10 steps correct | p=0.97) = 0.97^10 = 0.74
```

**Production implication:**
Validation gates after each step don't just improve quality — they reset the cumulative probability
product. With validation after each step that catches failures with 95% accuracy:

```
P(step i correct after validation) ≈ p + (1-p) * 0.95 (validation catches failure, agent retries)
```

This is the mathematical case for validation loops in Brainwires.

---

#### Gorilla: Large Language Model Connected with Massive APIs
**arXiv:** [2305.15334](https://arxiv.org/abs/2305.15334) | **Year:** 2023

**What is established:**
Fine-tuning improves mean API call accuracy but does NOT eliminate tail failures. Even the
fine-tuned Gorilla model has a non-zero failure rate on complex, real-world API calls.

**What this means formally:**
Improvement in mean ≠ elimination of tail risk. For production reliability requirements:

```
If tail failure rate = 0.5% and task = 200 steps:
P(200 steps correct) = 0.995^200 = 0.37
```

No current model achieves tail failure rate low enough for validation-free execution at scale.

**Production implication:**
External validation is not a workaround for poor models — it is a permanent architectural
requirement, even for the best available models.

---

### 1.4 Memory Formalization — Most Production-Aligned Theoretical Framework

#### MemGPT: Towards LLMs as Operating Systems
**arXiv:** [2310.08560](https://arxiv.org/abs/2310.08560) | **Year:** 2023

**What is formalized:**
MemGPT applies the OS virtual memory model to LLM context management:

```
Context window = RAM (fast, limited, volatile)
External storage = Disk (slow, unlimited, persistent)
Paging policy = Self-directed memory management functions
```

The agent explicitly calls functions to:
- Load information from disk to RAM (`retrieve(query)`)
- Flush information from RAM to disk (`store(content)`)
- Update stored content (`update(key, content)`)

**What this provides:**
A formal analogy that is directly implementable. The OS memory model has 50+ years of engineering
practice. Applying that practice to LLM context management provides:
- Clear interface definitions (load/store/update)
- Clear performance model (context window = working set)
- Clear lifecycle model (paging, eviction, garbage collection)

**Production gap (not addressed by MemGPT):**
- Authority hierarchy (who can write to canonical storage?)
- Poisoning detection (how to detect conflicting stored facts?)
- Confidence-weighted retrieval (not all retrieved content is equally trustworthy)

**Brainwires implementation:**
`TieredMemory` (hot/warm/cold) in `brainwires-storage` directly implements the MemGPT tiering.
Authority hierarchy and poison detection are current implementation gaps.

---

### 1.5 Evaluation — Distributional Reliability Guarantees

#### HELM: Holistic Evaluation of Language Models
**arXiv:** [2211.09110](https://arxiv.org/abs/2211.09110) | **Year:** 2022

**What is formalized:**
Reliability is a distribution over scenarios, not a point estimate. A model evaluated across
30+ scenarios with multiple metrics per scenario provides statistical confidence bounds on
performance.

**What this means formally:**
A production reliability claim requires:
1. Defined scenario distribution (representative of production inputs)
2. Multiple metric dimensions (accuracy, calibration, robustness, cost, latency)
3. Confidence intervals (not point estimates)
4. Failure mode taxonomy (not just pass rate)

**Production implication:**
"95% success rate in testing" is not a reliability guarantee until the testing distribution matches
the production distribution. HELM methodology applied to agent testing: run N trials across the
production scenario distribution, compute distribution of outcomes, set pass threshold with
statistical confidence.

---

## Part 2: Architectural Patterns Emerging Across Research

These patterns appear independently in multiple research threads, suggesting they represent
fundamental structural requirements rather than implementation choices.

---

### Pattern A: LLM as Policy Engine, Not Executor

**Papers:** ReAct (2210.03629), Tree of Thoughts (2305.10601), MemGPT (2310.08560)

**The convergent structure:**

```
External Controller
       │
       ▼
LLM (proposes next action)
       │
       ▼
Validator (is this action valid/permitted?)
       │
       ▼
Executor (deterministic execution)
       │
       ▼
State Update (execution result feeds back to controller)
```

All three papers independently arrive at this four-component structure. The LLM always occupies
exactly one layer — proposal. It never directly executes.

**Production requirement:** Make this separation explicit in code. In Brainwires:
- Controller = `TaskAgent.execute()` loop + `CommunicationHub`
- LLM = `call_provider()` → proposes next action
- Validator = `ToolExecutor` with `PolicyEngine` + lock acquisition
- Executor = tool implementations

**Anti-pattern:** Any architecture where the LLM directly causes side effects without an
intervening validator violates this pattern.

---

### Pattern B: External Scoring Beats Self-Critique

**Papers:** Tree of Thoughts (2305.10601), Graph of Thoughts (2308.09687), Reflexion (2303.11366)

**The convergent finding:**

All three papers show that model performance improves significantly when evaluation uses external
signals rather than the model evaluating its own output.

- ToT: external "thought scorer" (separate evaluation step) > self-evaluation
- GoT: aggregating external scores across multiple thoughts > single self-assessment
- Reflexion: verbal reflection works ONLY when grounded in external feedback signals

**The failure mode identified by Reflexion:**
When a model reflects without external grounding, it tends to:
1. Confirm its existing (possibly wrong) answer
2. Generate plausible-sounding justifications for errors
3. Increase confidence in wrong answers

**Production requirement:**
Replace self-reflection loops with deterministic external validators and task-specific evaluators.
Self-reflection should be used only to synthesize external feedback into actionable changes, not to
evaluate correctness.

**Brainwires implementation:**
`ValidationLoop` uses `verify_build`, `check_duplicates`, and `check_syntax` — all external,
deterministic validators. The validation loop never asks the model "is your output correct?"

---

### Pattern C: Memory Tiering Is Universal

**Papers:** Generative Agents (2304.03442), MemGPT (2310.08560), RETRO (2112.04426)

**The convergent structure:**

All three papers independently converge on a tiered memory architecture:

| Paper | Tier 1 | Tier 2 | Tier 3 |
|-------|--------|--------|--------|
| Generative Agents | In-context | Memory stream | Reflection summaries |
| MemGPT | Context window (RAM) | Working context | External DB (disk) |
| RETRO | Input context | Retrieved context | Retrieval corpus |

**Common structural requirements that emerge:**
1. Fast tier (context window) — low capacity, high immediacy
2. Medium tier (session) — moderate capacity, query-able
3. Slow tier (persistent) — high capacity, authoritative

**Production gap (none of the papers formalize):**
- Authority hierarchy across tiers (which tier wins on conflict?)
- Poisoning detection (what if a tier contains incorrect information?)
- Canonical override rules (how do high-authority facts propagate down?)
- Write access control (who can write to each tier?)

**Brainwires implementation:**
`TieredMemory` (hot/warm/cold) in `brainwires-storage` maps directly to this three-tier pattern.
The production gap items (authority, poisoning, override rules) are present in the architecture
as design questions, not yet as implemented mechanisms.

---

### Pattern D: Cost Control Always Retrofitted

**Papers:** FrugalGPT (2305.05176) appeared after: GPT-3 (2020), GPT-4 (2023), all capability
papers. Speculative decoding (2211.17192) appeared after: most inference research. Token budget
papers appear after: prompting papers.

**The pattern:**
Capability research comes first. Cost control research comes years later, as a reaction to deployment
experience showing that cost and latency are primary barriers to production adoption.

**Production implication:**
Cost control should NOT be retrofitted. It must be a first-class constraint from day one:
- Step budgets during initial design (not added after "it's too slow")
- Model routing by complexity (not added after "the bill is too high")
- Token compression (not added after "context limits are hit in production")
- Early exit policies (not added after "users complain it takes too long")

**Brainwires status:**
`max_iterations`, `max_tokens` per call, and MDAP cost tracking are first-class. Total run cost
ceiling and dynamic model routing are gaps that should be addressed before production scaling.

---

### Pattern E: Voting / Sampling / Ensemble Beats Single Sample

**Papers:** Tree of Thoughts (branching + selection), Graph of Thoughts (aggregation),
Reflexion (multi-trial improvement), MDAP (first-to-ahead-by-k voting)

**The convergent finding:**
Multiple samples from the same distribution + external selection consistently outperform single
best-effort samples, even with 3× the compute cost.

| Approach | Samples | Selection |
|----------|---------|-----------|
| ToT | tree paths | external scorer |
| GoT | aggregated thoughts | external aggregator |
| Reflexion | multi-trial | external environment feedback |
| MDAP | k samples | first-to-ahead-by-k voting |

**The mathematical reason:**
For a problem where P(correct single sample) = p, and samples are approximately independent:
- P(at least one correct in k samples) = 1 - (1-p)^k
- P(majority vote correct in k samples) > p for k ≥ 3

**Production requirement:**
For high-stakes decisions (code generation, irreversible actions), k-sample voting with external
verification is reliably more accurate than single-sample execution.

**Brainwires MDAP:**
`voting.rs` implements first-to-ahead-by-k. `MdapConfig.default()` uses k=3 (95% target),
`high_reliability()` uses k=5 (99% target). Empirically validated: 2.3× average efficiency
gain over single-sample on complex algorithms.

---

## Part 3: Current Research Gaps

Areas where the field lacks formal theory, and production stability is achieved empirically:

### Gap 1: Formal Verification of Agent Plans
**Current state:** No formal methods for verifying that a generated plan is correct before execution.
**Why it matters:** Plans with logical errors propagate those errors across all execution steps.
**Active research:** Planning verifiers using symbolic AI + LLM hybrid approaches (2024–2025).
**Pragmatic substitute:** Explicit plan review step with human approval before irreversible execution.

### Gap 2: Hard Termination Guarantees
**Current state:** Step budgets provide termination bounds, but models can declare false completion.
**Why it matters:** A model that says "done" when it isn't is a reliability failure without a
deterministic detection mechanism.
**Pragmatic substitute:** External validation gates (ValidationLoop) that independently verify
completion claims before accepting them.

### Gap 3: Deterministic Replay Frameworks
**Current state:** No standardized framework for deterministic replay of LLM agent executions.
**Why it matters:** Debugging production failures requires reproduction.
**Active research:** Several 2024–2025 papers on agent trace capture and replay.
**Pragmatic substitute:** Execution logging with tool I/O capture (AuditLogger) + manual reproduction.

### Gap 4: Contract-Level Semantic Tool Guarantees
**Current state:** Structural validation (Outlines) provides syntactic guarantees. Semantic
validation (is this the right tool call for this context?) has no formal framework.
**Why it matters:** Syntactically valid but semantically wrong tool calls are the dominant failure mode.
**Active research:** Tool semantic verification using symbolic constraints on agent state.
**Pragmatic substitute:** Semantic validators in tool execution pipeline (PolicyEngine).

### Gap 5: Memory Poisoning Detection Theory
**Current state:** No formal model of memory poisoning (when false information stored in memory
cascades into execution errors).
**Why it matters:** Memory poisoning produces failure modes that are invisible until they compound
into obvious errors, at which point debugging is difficult.
**Pragmatic substitute:** Confidence-weighted retrieval with human-in-loop escalation for
low-confidence retrievals.

### Gap 6: Cost-Aware Planning Algorithms
**Current state:** FrugalGPT provides routing, not planning-time cost estimation. No formal
algorithm for generating plans that are optimal under token budget constraints.
**Why it matters:** Plans that exceed budget mid-execution are worse than plans designed for
budget constraints from the start.
**Active research:** Budget-constrained planning with LLMs (emerging 2024–2025).
**Pragmatic substitute:** Step budget enforcement + early exit with partial results.

---

## Summary: What Is Formally Grounded vs. Empirically Grounded

| Property | Status | Source |
|----------|--------|--------|
| Structural output validity | Hard guarantee (with constrained decoding) | Outlines/XGrammar |
| Bounded execution time | Guarantee (with explicit depth bound) | ToT pattern |
| Per-step token count | Hard limit (with max_tokens parameter) | Inference API |
| Multi-step task success rate | Probabilistic bound | API-Bank + reliability math |
| Memory tiering benefit | Empirically strong | MemGPT + Generative Agents |
| Voting superiority | Probabilistic (law of large numbers) | ToT, MDAP, GoT |
| External validation benefit | Empirically strong | Reflexion, ValidationLoop |
| Prompt versioning benefit | Empirically grounded (DSPy) | DSPy |
| Role authority hierarchy benefit | Empirically grounded | MetaGPT, CAMEL |
| Full replay determinism | No formal solution yet | Research gap |
| Termination guarantee | No formal solution yet | Research gap |
| Memory poisoning detection | No formal solution yet | Research gap |
| Semantic tool guarantee | No formal solution yet | Research gap |

**Bottom line:** Most production stability today is engineered through empirically validated
patterns, not formally derived guarantees. The engineering investment in observable, controllable,
budget-bounded infrastructure is the pragmatic response to the formal gap.
