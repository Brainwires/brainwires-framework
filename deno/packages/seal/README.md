# @brainwires/seal

Self-Evolving Agentic Learning: coreference resolution, query-core extraction,
pattern store, reflection module, learning coordinator.

Extracted from `@brainwires/agents` in v0.11.0 to mirror Rust's
`brainwires-seal` crate. The Deno port keeps the learning loop in-process; the
Rust crate ships a LanceDB-backed pattern store, which Deno consumers should
plug in via the `RagClient` interface in `@brainwires/rag`.
