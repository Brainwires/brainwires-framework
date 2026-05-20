# @brainwires/inference

LLM-driven agent workhorses: chat agent, task agent, planner, judge, validator,
plan executor, cycle orchestrator, validation loop, runtime, system prompts.

Extracted from `@brainwires/agents` in v0.11.0 to mirror Rust's
`brainwires-inference` crate. The coordination primitives (`CommunicationHub`,
`TaskManager`, `FileLockManager`, etc.) stay in `@brainwires/agents` (renamed to
`@brainwires/agent` in v0.11.0 — both names work during the transition).
