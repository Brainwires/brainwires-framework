#![deny(missing_docs)]
//! `brainwires-training` — placeholder for training-from-scratch primitives.
//!
//! No training-from-scratch code lives here yet. The crate exists to occupy
//! the `brainwires-training` name on crates.io and document the intended
//! split:
//!
//! - **`brainwires-finetune`** — cloud fine-tune APIs (OpenAI, Anthropic,
//!   Together, Fireworks, Anyscale, Bedrock, Vertex AI) plus dataset
//!   pipelines.
//! - **`brainwires-finetune-local`** — local PEFT fine-tuning
//!   (LoRA / QLoRA / DoRA) on a pre-trained model, Burn-backed.
//! - **`brainwires-training`** (this crate) — reserved for actual training
//!   from scratch (full-parameter pretraining, distributed training, etc.).
//!   Add code here when that work begins.
