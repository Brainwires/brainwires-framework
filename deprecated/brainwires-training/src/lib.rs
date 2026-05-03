#![deprecated(
    since = "0.10.1",
    note = "the 0.10.x `brainwires-training` content (cloud + local fine-tuning) was renamed for accuracy. Use `brainwires-finetune` for cloud APIs + dataset pipelines, and `brainwires-finetune-local` for local LoRA/QLoRA/DoRA. The `brainwires-training` name is reserved for actual training-from-scratch in 0.11+ — see https://crates.io/crates/brainwires-training (0.11.0+) for the new placeholder."
)]
//! `brainwires-training` 0.10.x is **deprecated**.
//!
//! What was previously called "training" was actually fine-tuning. The
//! 0.10.x content moved to two crates:
//!
//! - [`brainwires-finetune`](https://crates.io/crates/brainwires-finetune) —
//!   cloud fine-tune APIs (OpenAI / Anthropic / Together / Fireworks /
//!   Anyscale / Bedrock / Vertex AI) plus dataset pipelines.
//! - [`brainwires-finetune-local`](https://crates.io/crates/brainwires-finetune-local) —
//!   local PEFT (LoRA / QLoRA / DoRA), Burn-backed.
//!
//! The `brainwires-training` name is preserved (0.11+) for actual
//! training-from-scratch primitives — see the active 0.11.0+ versions on
//! crates.io.
