# Future Work

Ideas and planned improvements for the Brainwires CLI. Items are organized by area and tagged by rough priority:

- **High** — pre-release blocker or near-term must-have
- **Medium** — high-impact, target shortly after initial release
- **Low** — long-term vision, post-release

---

## User Experience & TUI

- **Clipboard support** `Medium` — paste images or files directly into the chat input; drag-and-drop file upload for attaching context to a message.
- **Multimodal input** `Medium` — image and screenshot paste into the chat for vision-capable models.
- **Up-arrow to edit queued messages** `Medium` — before a response begins, allow the user to recall and edit the last message, consistent with standard shell behavior.
- **Prompt quality meter** `Low` — evaluate the prompt as the user types and display a live quality indicator based on length, clarity, and specificity. Optionally surface suggestions using a small local model.
- **Input field placeholder suggestions** `Low` — rotate through example prompts in the input field's placeholder text to help new users get started.
- **Runtime feature flag toggle** `Low` — a TUI panel for toggling experimental features on/off at runtime, without restarting, to support in-session A/B testing during development.

---

## Agent Execution

- **Long-running process registry** `High` — maintain a registry of all processes spawned by agents. Enforce termination when the agent exits or when a configurable time limit is exceeded. Prevent orphaned processes and runaway loops. Add tools such as `wait_for_process` that let agents pause until a specific process state is reached.
- **Pause and restart system** `Medium` — allow the agent's compute loop to be paused and resumed, triggered either by a time estimate ("this build will take 20 minutes") or by an external event (a file appearing, a process exiting, a webhook).
- **Attention mechanism** `Medium` — when executing a multi-step plan, surface additional context relevant to the current step and suppress noise from earlier steps, keeping the agent focused.
- **Parallelism in plan execution** `Medium` — identify tasks within a plan that have no dependencies on each other and execute them concurrently, reducing wall-clock time for multi-step operations.
- **Resume most recent session on startup** `Medium` — a command-line flag (e.g. `--resume`) that reloads the most recent conversation automatically, so users can restart the app and pick up exactly where they left off.

---

## Multi-Agent & Distributed Computing

- **Bridge-to-bridge communication** `Medium` — a distributed mesh where agents running on different machines can communicate, share context, and collaborate on a single task. Agents with different capabilities (hardware, network access, local models) contribute to a shared goal. Enables pooling of compute resources across a network.
- **A2A protocol future phases** `Low`:
  - HTTP server mode — expose an agent directly via HTTP without requiring a bridge
  - Push notification webhooks — async callbacks for long-running tasks
  - gRPC transport — alternative to HTTP polling
  - External A2A agent federation — interoperate with non-Brainwires A2A agents
  - Agent registry — discover agents by declared skill or capability
- **Routines** `Low` — AI-created, reusable sequences of actions designed to accomplish recurring tasks. Routines can be named, saved, and triggered by events or conditions to automate repetitive workflows.

---

## Tool System

- **Sandboxed bash execution** `High` — run bash tool commands in an isolated subprocess with restricted environment variables, no outbound network unless explicitly permitted, and filesystem scope limited to the working directory. Prevents accidental or adversarial access to sensitive system resources.
- **Validation tool improvements** `Medium`:
  - Add path validation and canonicalization (defense against path traversal)
  - Add timeout protection for build commands that hang indefinitely
  - Convert synchronous I/O to async/await
  - Expand TypeScript/JS export detection: `export default`, named `export { }`, re-exports, `export async function`, decorators
  - Multi-language error parsing: Python, Go, Java, C/C++ in addition to TypeScript and Rust
  - Additional build system support: yarn, pnpm, bun, go build, gradle, maven, poetry, dotnet, make
  - LSP integration for accurate, language-server-backed syntax checking
  - Result caching and incremental validation (avoid re-running unchanged files)
  - Watch mode for continuous background validation
- **Image generation tools** `Medium` — allow agents to call image generation APIs inline and receive the result as an attachment or file reference.

---

## Providers & Model Routing

- **Token compression pipeline** `Medium` — before sending context to the model: summarize conversation history beyond N turns; compress tool results to key fields; truncate repetitive context blocks. Reduces cost and prevents context window exhaustion.
- **Batch processing support** `Medium` — use provider batch APIs where available (e.g. Anthropic Batch, OpenAI Batch) to reduce token costs on parallelizable workloads.
- **Dynamic model routing** `Low` — FrugalGPT-style routing: estimate task complexity from the request, then route to the appropriate model tier (haiku/flash for simple tasks, sonnet/pro for reasoning, opus/gpt-4 for the hardest). Target 60–80% cost reduction by keeping ~70% of tasks on cheaper models.
- **Prompt versioning with semantic IDs** `Low` — a `PromptVersion` struct that records a semantic identifier and hash of every prompt variant used. Run an evaluation suite before promoting prompt changes to ensure quality is not regressed.
- **Full replay framework** `Low` — deterministic replay from a stored execution graph: frozen model version, tool registry hash, exact tool I/O. Replaying from the same seed and mocked tool outputs produces identical agent decisions — enables regression testing of agent logic.
- **A/B experiment framework** `Low` — compare two model or prompt variants on the same task distribution; compute success rate difference with a statistical significance test; require significance before promoting a change.
- **Sub-context windows** `Low` — a focused, smaller context window scoped to a specific subtask (like the `Explore` subagent pattern). The agent performs lightweight exploration in the sub-context before committing tokens to the main context, reducing overall token spend.

---

## Integrations

- **VSCode extension** `Medium` — deep IDE integration comparable to Claude Code: inline diff view, file context awareness, inline agent spawning, status bar indicators.
- **Multimodal input** `Medium` — see UX section above; image paste into chat for vision models.
- **GitHub automation** `Low` — integrate with GitHub Issues, Pull Requests, and Actions. Agents can automatically create issues for bugs or feature requests, open pull requests for completed work, and trigger CI workflows. Enables a fully automated development loop driven by the agent.
- **Git repo + issue workflow** `Low` — specify a public GitHub repo URL; the agent fetches open issues and works through them autonomously, submitting PRs for completed fixes.

---

## Framework Infrastructure

- **OS detection for tools** `Medium` — tool executors that run shell commands should detect the host OS (Windows/macOS/Linux) and select the appropriate command syntax automatically, rather than assuming Unix.
- **Video and image processing crates** `Low` — in the same spirit as `brainwires-hardware` (audio), add crates for video and image handling: object detection, image classification, video summarization, frame extraction. Enables agents to reason about multimedia content.
- **WASM verification** `Low` — audit `brainwires-wasm` bindings for completeness against all core types; ensure the browser target builds successfully with `wasm-pack`; add basic WASM smoke tests to CI.
- **Structured extraction module** `Low` — typed LLM output extraction: deserialize model responses directly into Rust structs via JSON mode, similar to Rig's `extractor` module.
- **HuggingFace model hub integration** `Low` — a `from_pretrained()`-style API for downloading model weights from HuggingFace Hub for use with local inference, eliminating the need for users to manually source and place weight files.
- **Additional extras** `Low` — expand the extras workspace with standalone binaries for audio processing (building on `brainwires-hardware`) and training pipeline tools (building on the `burn`-based training crates).

---

## Research Directions

- **Compute-for-accuracy tradeoff loop** `Medium` — for less-capable or cheaper models, improve output reliability by running a mini-loop: Solve → Validate → Reflect → Correct → Solve. Related to MDAP's voting approach; applicable as a lightweight alternative when full k-agent voting is too expensive.
- **SEAL future enhancements** `Low`:
  - SEAL Reflection → BKS Correction: when the reflection module detects a recurring error pattern, automatically create a BKS truth to avoid it in future
  - Cross-user pattern aggregation: collect anonymized SEAL patterns server-side for meta-learning across users
  - Entity relationship graph integration: use SEAL's relationship graph for more sophisticated BKS truth matching
  - Adaptive threshold tuning: learn optimal confidence thresholds per user based on accuracy feedback over time
  - Pattern conflict resolution: when SEAL and BKS disagree, resolve via voting or confidence comparison
- **AI self-evaluation benchmark** `Low` — run the framework against a curated benchmark of refactoring, test design, code review, and documentation tasks to produce objective quality metrics. Use results to guide prompt and agent improvements.
