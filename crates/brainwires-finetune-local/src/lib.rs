#![allow(missing_docs)]
//! Local LoRA / QLoRA / DoRA fine-tuning for the Brainwires Agent Framework.
//!
//! These are PEFT (parameter-efficient fine-tuning) methods running on a
//! pre-trained model — distinct from the cloud fine-tune APIs
//! (`brainwires-finetune`) and from training-from-scratch
//! (`brainwires-training`).
//!
//! Standalone so consumers that only want cloud fine-tune APIs don't pay
//! the burn / safetensors / tokenizers compile cost.
//!
//! Depends on `brainwires-finetune` for the shared `config` / `error` /
//! `types` infrastructure.

// Re-export burn_core as `burn` so Burn's derive macros (Module, Config) can
// resolve their internal `burn::` paths when using the individual burn-*
// crates. Required because we don't depend on the umbrella `burn` crate.
extern crate burn_core as burn;

pub mod local;
pub use local::*;
