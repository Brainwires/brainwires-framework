The TUI (or future GUI) - to be able to "runtime feature flag" - some ability to toggle features for testing purposes.

(Focus) Attention Mechanism - When the agent is executing a plan, it should be able to "focus" on a specific step or subtask. This could involve highlighting the relevant information, providing additional context, or even temporarily blocking out distractions. The attention mechanism would help the agent stay on track and ensure that it is effectively executing the plan.

Parallelism - When executing a plan, the agent should be able to identify tasks that can be executed in parallel and do so to improve efficiency. This would involve analyzing the dependencies between tasks and determining which ones can be executed simultaneously without causing conflicts.

Work the checklist items in the CHECKLIST.md file, prioritizing high-impact, pre-release hygiene tasks first, followed by medium-priority crate consolidation and multi-agent coordination features. Low-priority production excellence features can be deferred until after the initial release.

Add suggestions on what to type in the input field's placeholder text.

Remove this file from the repo

How useful is this framework for academic use

Management of long running system processes - If an agent starts a long running bash process, and for whatever reason loses track of it. Maintain a registry of active processes, monitoring their status, and enforing that they are properly terminated when the agent finishes or if the process exceeds a certain time limit. This would prevent orphaned processes and resource leaks. Claude likes to also write while loops that never terminate, so this would be a useful safeguard.contin

Crates for video and images - in the same spirt as the audio crate, we could have crates for video and image processing. This would allow agents to handle multimedia content, such as analyzing images, generating videos, or extracting information from visual data. These crates could include features like object detection, image classification, video summarization, and more.

Need the ability to tie in to vscode, just like Claude Code does.

Goal is full automation of this framework though GitHub. Tieing into Issues, Pull Requests, and Actions. This would allow for a seamless development workflow, where agents can automatically create issues for bugs or feature requests, submit pull requests for code changes, and even trigger actions for testing and deployment. This level of automation would significantly enhance the efficiency of the development process and enable rapid iteration on the framework.

NAMES REFACTOR? >>> "● Good — there's already a lifecycle module in brainwires-core for observational event hooks. My new module serves a different purpose (loop control with conversation access and delegation). I'll name it agent_hooks to avoid confusion."

Batch Processing Support - Saves token costs.

Support for video input and output - This would allow agents to process and generate video content, opening up new possibilities for applications such as video summarization, content creation, and multimedia analysis.

More extras
    - for audio and training crates

WTF With Thalora!

● thalora - snapshot_url (MCP)(url: "https://crates.io/docs/rate-limits")                                                                                    
  ⎿  {                                                                                                                                                       
       "basic": {                                                                                                                                            
         "images": [],                                      
     … +18 lines (ctrl+o to expand)

● The page is a client-rendered SPA — no content extracted. Let me search for the rate limits info instead.  

Tip: Use /btw to ask a quick side question without interrupting Claude's current work 

Rag of nested contents... All rag db in same place.

The agent tools that access the operating system, need to know what operating system they are running on, to be able to execute the correct commands. This could be achieved by implementing a system detection mechanism that identifies the operating system and provides the appropriate command syntax for executing tasks. This would ensure that the agent can effectively interact with the underlying system regardless of whether it's running on Windows, macOS, or Linux.

Good System prompt addition: "Don't ever ask the use questions that you can systematically answer yourself by looking at the context or using tools. Always attempt to answer the question yourself before asking the user. If you do need to ask the user, make sure to provide them with options to choose from instead of leaving it open-ended."

# Brainwires Framework — Pre-Release Checklist

Remaining work items before public release. Completed items from previous phases have been removed.
See `analysis.md` for full evaluation context (crate architecture, Burn assessment, Rig comparison).

Priority definitions:
- **High** — pre-release blocker
- **Medium** — should address before or shortly after release
- **Low** — future enhancement, post-release

---

## Security Hardening
> **Priority: MEDIUM**

- [ ] **Sandboxed bash execution** — Run bash tool commands in an isolated subprocess: restricted env vars, no network access unless explicitly permitted, filesystem scope limited to working directory.

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

---

## Future Enhancements
> **Priority: LOW** — Post-release improvements informed by competitive analysis.

- [ ] **Structured extraction module** — Typed LLM output extraction (like Rig's `extractor` module). Deserialize LLM responses directly into Rust structs via JSON mode.
- [ ] **Expand provider count** — Anyscale, Fireworks, Together providers are in progress (visible in git status). Complete and test these.
- [ ] **HuggingFace model hub integration** — Add model downloading for local training. Currently there's no `from_pretrained()` equivalent — users must manually provide model weights.
