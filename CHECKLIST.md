# Brainwires Framework — Pre-Release Checklist

Remaining work items before public release. Completed items from previous phases have been removed.
See `analysis.md` for full evaluation context (crate architecture, Burn assessment, Rig comparison).

Priority definitions:
- **High** — pre-release blocker
- **Medium** — should address before or shortly after release
- **Low** — future enhancement, post-release

---

## Pre-Release Hygiene
> **Priority: HIGH**

- [x] **Add README.md to `brainwires-a2a`** — Crate has no README; needed for crates.io publish and developer onboarding.
- [x] **Add README.md to `brainwires-mesh`** — Crate has no README.
- [x] **Add README.md to `brainwires-audio`** — Crate has no README.
- [x] **Add README.md to `brainwires-datasets`** — New crate, needs documentation of format converters, tokenizer features, quality tools.
- [x] **Add README.md to `brainwires-training`** — New crate, needs documentation of cloud providers, local Burn training, adapter methods.
- [x] **Add README.md to framework root** — `crates/brainwires-framework/README.md` is the entry point for developers discovering the project.
- [x] **Add README.md to `extras/brainwires-proxy`** — Proxy crate already had a README.
- [ ] **Align `brainwires-rag` version** — Currently at `0.1.1` while all other crates are `0.1.0`. Synchronize before first public release.
- [x] **Document Rust 1.85+ minimum** — All crates use `edition = "2024"` which requires Rust 1.85+. Documented in framework root README.
- [ ] **Verify `rag` facade feature scope** — The `brainwires-rag` crate pulls 20+ optional tree-sitter language bindings under the `native` feature. Verify that enabling `rag` in the facade crate doesn't transitively pull all tree-sitter parsers. Consider adding a `rag-full-languages` feature flag if it does.

---

## Crate Consolidation
> **Priority: MEDIUM** — Reduces maintenance surface from 22 to 18 crates without losing functionality.

- [ ] **Evaluate collapsing `brainwires-mdap` into `brainwires-agents`** — MDAP is tightly coupled to agents (voting/decomposition layer on top of agent orchestration). No other crate uses it independently. Would become a feature `mdap` on `brainwires-agents`.
- [ ] **Evaluate collapsing `brainwires-permissions` into `brainwires-core`** — Permission policies are a cross-cutting concern. Most crates that need permissions already depend on core. Would become a feature `permissions` on `brainwires-core`.
- [ ] **Evaluate collapsing `brainwires-eval` into `brainwires-agents`** — Evaluation is always of agents. The N-trial Monte Carlo evaluator can't meaningfully run without the agent system. Would become a feature `eval` on `brainwires-agents`.
- [ ] **Evaluate collapsing `brainwires-a2a` into `brainwires-relay`** — A2A protocol is a specialization of agent communication. Relay already handles MCP server/client comms with shared transport concerns. Would become a feature `a2a` on `brainwires-relay`.

---

## Multi-Agent Coordination
> **Priority: MEDIUM**

- [ ] **Validator agent type** — Implement `ValidatorAgent` as a distinct agent type that holds only read locks, runs external validators (`verify_build`, `check_syntax`, `check_duplicates`), and returns a structured `ValidationResult` to the orchestrator.
- [ ] **Orchestrator ↔ TaskManager integration** — Wire `OrchestratorAgent` subtask assignment through `TaskManager`'s dependency graph so task ordering and status are tracked centrally.

---

## Security Hardening
> **Priority: MEDIUM**

- [ ] **Sandboxed bash execution** — Run bash tool commands in an isolated subprocess: restricted env vars, no network access unless explicitly permitted, filesystem scope limited to working directory.

---

## Training Completion
> **Priority: MEDIUM** — Flesh out scaffolded features in the new training crates.

- [ ] **Implement DPO alignment** — Currently scaffolded in `brainwires-training/src/local/alignment/dpo.rs`. Implement Direct Preference Optimization loss computation using Burn tensors.
- [ ] **Implement ORPO alignment** — Currently scaffolded in `brainwires-training/src/local/alignment/orpo.rs`. Implement Odds Ratio Preference Optimization.
- [ ] **Flesh out QLoRA adapter** — Currently a skeleton in `brainwires-training/src/local/adapters/qlora.rs`. Note: true QLoRA requires INT4 quantized kernels that Burn doesn't yet support (no QAT). Document limitations and implement what's feasible.
- [ ] **Flesh out DoRA adapter** — Currently a skeleton in `brainwires-training/src/local/adapters/dora.rs`. Implement direction-magnitude decomposition on top of the existing LoRA implementation.

---

## Production Excellence
> **Priority: LOW**

- [ ] **Dynamic model routing** — FrugalGPT-style: estimate task complexity, route to haiku/sonnet/opus class model. Target 60-80% cost reduction by routing ~70% of tasks to cheaper models.
- [ ] **Token compression pipeline** — Before sending to model: summarize conversation history beyond N turns, compress tool results to key fields only, truncate repetitive context.
- [ ] **Prompt versioning with semantic IDs** — `PromptVersion` struct with semantic identifier + hash; snapshot exact prompt text with every run; run evaluation suite before promoting prompt changes.
- [ ] **Full replay framework** — Deterministic seed from run ID; store frozen model version, tool registry hash, exact tool I/O; replay from `ExecutionGraph` + mocked tool outputs produces identical decisions.
- [ ] **A/B experiments** — Compare model upgrade / prompt change pairs; compute success rate diff with statistical significance test; require significance before promoting changes.

---

## Framework Extraction
> **Priority: LOW**

- [ ] **Verify `brainwires-wasm`** — Audit WASM bindings for all core types; ensure browser target builds succeed with `wasm-pack`; run basic WASM smoke tests.
- [ ] **Complete `brainwires-seal`** — Wire SEAL (Self-Evolving Agentic Learning) integration through `brainwires-prompting` (knowledge feature); implement the learning loop that reads `AuditLogger` feedback to improve prompting strategies over time.
- [ ] **`brainwires-eval` as standalone crate** — Extract as a publishable crate usable by projects outside the CLI. (May be superseded by crate consolidation above.)
- [ ] **CLI thin-wrapper audit** — Review every module in `src/` against its framework counterpart; confirm each is a genuine thin wrapper with no duplicated logic; document any intentional divergences.

---

## Future Enhancements
> **Priority: LOW** — Post-release improvements informed by competitive analysis.

- [ ] **ToolEmbedding for semantic tool discovery** — Inspired by Rig's `ToolEmbedding` trait. When an agent has many tools, use embedding similarity to discover the right tool rather than listing all definitions in the prompt.
- [ ] **Structured extraction module** — Typed LLM output extraction (like Rig's `extractor` module). Deserialize LLM responses directly into Rust structs via JSON mode.
- [ ] **Expand provider count** — Anyscale, Fireworks, Together providers are in progress (visible in git status). Complete and test these.
- [ ] **HuggingFace model hub integration** — Add model downloading for local training. Currently there's no `from_pretrained()` equivalent — users must manually provide model weights.

---

## Priority Matrix

| Section | Items | Priority | Notes |
|---------|-------|----------|-------|
| Pre-Release Hygiene | 10 | **High** | Blockers for crates.io publish |
| Crate Consolidation | 4 | Medium | 22 → 18 crates, evaluate before release |
| Multi-Agent Coordination | 2 | Medium | Retained from previous checklist |
| Security Hardening | 1 | Medium | Retained from previous checklist |
| Training Completion | 4 | Medium | New training crate scaffolds |
| Production Excellence | 5 | Low | Post-release optimization |
| Framework Extraction | 4 | Low | Architectural completeness |
| Future Enhancements | 4 | Low | Competitive feature parity |

**Total: 34 items** (10 high, 11 medium, 13 low)

**Recommended order:** Pre-Release Hygiene → Crate Consolidation → Training Completion → Multi-Agent + Security → Production Excellence → Framework Extraction → Future Enhancements
