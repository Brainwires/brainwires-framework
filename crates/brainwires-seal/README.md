# SEAL — Self-Evolving Agentic Learning

This module (inside `brainwires-agent`) implements the SEAL framework for enhancing conversational question answering and agent decision-making. It provides coreference resolution, structured query extraction, self-evolving pattern learning, and post-execution reflection — enabling agents to understand implicit references, build reusable knowledge, and correct their own mistakes without retraining.

> Inspired by: **SEAL: Self-Evolving Agentic Learning for Conversational Question Answering over Knowledge Graphs** (Wang et al., arXiv:2512.04868, December 2024)

## Why It Lives in `brainwires-agent`

SEAL was previously a standalone crate (`brainwires-seal`). It was moved here because:

1. **No circular dependencies** — SEAL needs `ResponseConfidence` (defined in agents) and `ToolOutcome`/`ToolErrorCategory` (from `brainwires-tools`, already a dep of agents).
2. **Semantically correct** — "Self-Evolving **Agentic** Learning" belongs with agents.
3. **Feature-gated** — the `seal` feature flag keeps it opt-in; optional integrations use `seal-knowledge`, `seal-feedback`, and `seal-mdap`.

## Feature Flags

| Feature | Description |
|---------|-------------|
| `seal` | Core SEAL pipeline (coreference, query extraction, learning, reflection) |
| `seal-mdap` | MDAP metric recording via `mdap` feature |
| `seal-knowledge` | BKS/PKS knowledge system integration via `brainwires-knowledge` |
| `seal-feedback` | Audit feedback bridge via `brainwires-permissions` |

```toml
# Core SEAL
brainwires-agent = { version = "0.10", features = ["seal"] }

# With knowledge integration
brainwires-agent = { version = "0.10", features = ["seal-knowledge"] }

# Via the brainwires facade
brainwires = { version = "0.10", features = ["seal"] }
```

## Architecture

```text
User Query
    │
    ▼
┌─── Coreference Resolution ─────────────────────────────────────┐
│  detect_references() → resolve() → rewrite_with_resolutions()  │
│  "What uses it?" → "What uses [main.rs]?"                      │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─── Query Core Extraction ──────────────────────────────────────┐
│  classify() → build_expression() → QueryCore                   │
│  S-expression: (JOIN DependsOn ?dep "main.rs")                 │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─── Learning Coordinator ───────────────────────────────────────┐
│  Local Memory (per-session)  │  Global Memory (cross-session)  │
│  process_query() → match pattern or create new                 │
│  record_outcome() → update reliability scores                  │
└────────────────────────────────┬────────────────────────────────┘
                                 │
                                 ▼
┌─── Reflection Module ──────────────────────────────────────────┐
│  analyze() → detect issues → suggest fixes → attempt_correction│
│  Errors: EmptyResult, Overflow, EntityNotFound, RelationMismatch│
└────────────────────────────────────────────────────────────────┘
```

## Quick Start

```rust,ignore
use brainwires_agent::seal::{SealProcessor, SealConfig, DialogState};
use brainwires_core::graph::{EntityStoreT, RelationshipGraphT};

let mut processor = SealProcessor::with_defaults();
processor.init_conversation("session-001");

let result = processor.process(
    "What uses it?",
    &dialog_state,
    &entity_store,
    Some(&graph),
)?;

println!("Resolved: {}", result.resolved_query);
```

## Components

- **`SealProcessor`** — Main orchestrator chaining all pipeline stages
- **`CoreferenceResolver`** — Salience-weighted anaphora resolution
- **`QueryCoreExtractor`** — NL → structured S-expression queries
- **`LearningCoordinator`** — Dual-level memory (local + global) with pattern learning
- **`ReflectionModule`** — Post-execution error detection and correction
- **`SealKnowledgeCoordinator`** — BKS/PKS bidirectional bridge (requires `seal-knowledge`)
- **`FeedbackBridge`** — Audit log → learning signal converter (requires `seal-feedback`)
