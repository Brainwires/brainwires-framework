# brainwires-training

[![Crates.io](https://img.shields.io/crates/v/brainwires-training.svg)](https://crates.io/crates/brainwires-training)
[![Documentation](https://docs.rs/brainwires-training/badge.svg)](https://docs.rs/brainwires-training)
[![License](https://img.shields.io/badge/license-MIT%20%7C%20Apache--2.0-blue)](https://github.com/Brainwires/brainwires-framework)

**Placeholder.** Reserved for actual training-from-scratch primitives
(full-parameter pretraining, distributed training, etc.). No
training-from-scratch code lives here yet.

The split:

- [`brainwires-finetune`](https://crates.io/crates/brainwires-finetune) — cloud fine-tune APIs (OpenAI / Anthropic / Together / Fireworks / Anyscale / Bedrock / Vertex AI) plus dataset pipelines.
- [`brainwires-finetune-local`](https://crates.io/crates/brainwires-finetune-local) — local PEFT fine-tuning (LoRA / QLoRA / DoRA) on a pre-trained model, Burn-backed.
- **`brainwires-training`** (this crate) — reserved for actual training from scratch. Add code here when that work begins.

## What happened to what used to be in `brainwires-training`?

`brainwires-training` v0.10.x and earlier shipped two unrelated things
under one "training" label, both of which are actually **fine-tuning**:

- **Cloud fine-tune APIs** (OpenAI, Anthropic, Together, Fireworks,
  Anyscale, Bedrock, Vertex AI) plus dataset pipelines — moved to
  [`brainwires-finetune`](https://crates.io/crates/brainwires-finetune).
- **Local PEFT** (LoRA / QLoRA / DoRA on a pre-trained base model,
  Burn-backed) — moved to
  [`brainwires-finetune-local`](https://crates.io/crates/brainwires-finetune-local).

If you were using `brainwires-training = "0.10"` in your `Cargo.toml`,
swap to one or both of those — see their READMEs for migration tables.

## Why is this crate still active, then?

The `brainwires-training` name is intentionally **kept active** on
crates.io (no deprecation tombstone, no yanked versions) because the
framework reserves it for future **training-from-scratch** work — the
thing the original name should have meant all along. There's no code
yet; v0.11.0 ships as a placeholder so the name stays under the
framework's control until real pretraining / distributed-training
primitives land.

If you `cargo add brainwires-training` today, you get nothing useful —
that's by design. Watch this crate (or the framework changelog) for
the actual training-from-scratch surface to land in a future release.

## License

MIT OR Apache-2.0
