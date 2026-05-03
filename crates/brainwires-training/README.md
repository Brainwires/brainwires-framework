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

## Why a placeholder?

Earlier versions of `brainwires-training` mixed cloud fine-tune APIs and
local PEFT into one crate, both labelled "training" — which is technically
incorrect. Both are fine-tuning. The crate was renamed to
`brainwires-finetune` to fix the misnomer; this name is preserved for the
future training-from-scratch work that the framework will eventually grow.

## License

MIT OR Apache-2.0
