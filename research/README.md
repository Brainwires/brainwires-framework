# Agentic Systems Research Reference

This directory is the authoritative engineering reference for Brainwires' agentic system architecture.
It contains synthesized research, production patterns, and implementation analysis — distinct from:

- `docs/` — user-facing documentation
- `reference/research/` — lightweight notes (superseded by this directory)

---

## Documents

### [01 — Core Engineering Challenges](./01-engineering-challenges.md)
**Use when:** Onboarding, diagnosing system failures, or explaining why something is hard.

Covers the 10 fundamental engineering challenges in production agentic systems:
determinism, tool contracts, memory architecture, planning stability, latency, testing,
multi-agent coordination, prompt versioning, security, and the autonomy tradeoff.

Each challenge includes: what breaks, developer pain points, and 2024–2025 state of the field.

---

### [02 — Production Anti-Patterns](./02-anti-patterns.md)
**Use when:** Reviewing architecture decisions, PR reviews, or post-mortem analysis.

11 anti-patterns with concrete smells, root causes, and fixes — including Brainwires-specific
references to where each fix is implemented or still needed.

Key patterns: "let the model figure it out," prompts as configuration, memory without lifecycle,
infinite autonomy, multi-agent without contracts, over-trusting self-reflection.

---

### [03 — Architectural Principles](./03-production-principles.md)
**Use when:** Designing new framework components or evaluating architectural trade-offs.

15 principles for production agentic systems, each with rationale, Brainwires implementation
reference, and gap status. Includes reference architecture diagram.

Meta-principle: *The more autonomous the system, the more deterministic the surrounding
infrastructure must be.*

---

### [04 — Research Paper Catalog](./04-research-catalog.md)
**Use when:** Literature review, justifying design decisions, evaluating new techniques.

~30 papers organized by domain with arXiv links, core findings, and production implications.

Domains: determinism/observability, tool use, memory, planning, evaluation, multi-agent,
security, cost/latency, structured outputs, formal methods.

Includes priority reading list for production engineering.

---

### [05 — Formal Guarantees and Architectural Patterns](./05-formal-guarantees-and-patterns.md)
**Use when:** Core framework architecture work or research alignment reviews.

**Part 1:** Papers approaching formal guarantees — what IS and IS NOT guaranteed by constrained
decoding, bounded search, probabilistic tool reliability bounds, memory formalization, and
distributional evaluation.

**Part 2:** Cross-paper architectural patterns — structures that converge across independent
research threads (LLM as policy engine, external scoring beats self-critique, memory tiering,
cost control always retrofitted, voting beats single sample).

**Part 3:** Current research gaps — areas where production stability is engineered empirically
because formal theory doesn't yet exist.

---

### [06 — Research to Production Mapping](./06-research-to-production-mapping.md)
**Use when:** Implementing features, reviewing implementations, or planning roadmap items.

The most actionable document. For each challenge area:
- Research insight (with paper reference)
- Production pattern (concrete code structure)
- Brainwires implementation status (what's done, what's a gap)
- Next step recommendation

Ends with a full **implementation gap analysis table** with priority ratings for every open item.

---

## Quick Reference: Where to Look for What

| Question | Document |
|----------|---------|
| Why is X so hard? | 01 — Engineering Challenges |
| This design smells wrong, is it an anti-pattern? | 02 — Anti-Patterns |
| What principle should guide this design? | 03 — Production Principles |
| What paper justifies this approach? | 04 — Research Catalog |
| Does this have a formal guarantee? | 05 — Formal Guarantees |
| Is this pattern already implemented in Brainwires? | 06 — Research to Production |
| What should we build next? | 06 — Gap Analysis Table |

---

## Key Brainwires Files Referenced

| File | What it Implements |
|------|--------------------|
| `src/agents/task_agent.rs` | Control plane, step budget, validation gate |
| `src/agents/orchestrator.rs` | Single-orchestrator pattern |
| `src/agents/pool.rs` | Agent lifecycle management |
| `src/tools/executor.rs` | Tool contract enforcement |
| `src/tools/validation_tools.rs` | External validators (build, duplicates, syntax) |
| `crates/brainwires-mdap/src/metrics.rs` | Execution metrics, cost tracking |
| `crates/brainwires-mdap/src/voting.rs` | First-to-ahead-by-k voting |
| `crates/brainwires-permissions/src/audit.rs` | Audit logging |
| `crates/brainwires-permissions/src/policy.rs` | Declarative permission rules |
| `crates/brainwires-agents/src/state_model.rs` | Three-state distributed model |
| `crates/brainwires-storage/` | Tiered memory (hot/warm/cold) |
| `crates/brainwires-agents/src/resource_locks.rs` | File lock coordination |

---

## Research Gaps (From Doc 06)

Highest-priority unimplemented items for production readiness:

1. **Execution DAG per run** — enables debugging of all other issues
2. **Input sanitization layer** — security requirement before external content exposure
3. **Instruction hierarchy enforcement** — prevents prompt injection
4. **Semantic tool validator** — reduces tool misuse failure rate
5. **Automated Monte Carlo evaluation** — measures impact of all improvements
6. **Loop detection** — prevents runaway cost from planning loops
7. **Memory authority hierarchy** — canonical facts cannot be overwritten by model output
8. **Per-run cost/token budget** — prevents runaway cost per workflow
