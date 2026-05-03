#![allow(missing_docs)]
//! Local LoRA / QLoRA / DoRA training for the Brainwires Agent Framework.
//!
//! Standalone so consumers that only want cloud finetune APIs (Anthropic /
//! OpenAI / Bedrock / Vertex AI / etc., living in `brainwires-training`)
//! don't pay the burn / candle / safetensors compile cost.
//!
//! Depends on `brainwires-training` for the shared `config` /
//! `error` / `types` infrastructure.

// Re-export burn_core as `burn` so Burn's derive macros (Module, Config) can
// resolve their internal `burn::` paths when using the individual burn-*
// crates. Required because we don't depend on the umbrella `burn` crate.
extern crate burn_core as burn;

pub mod local;
pub use local::*;
