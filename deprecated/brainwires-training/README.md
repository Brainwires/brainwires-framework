# brainwires-training 0.10.x (DEPRECATED)

The 0.10.x content of `brainwires-training` was a misnomer — the crate
contained cloud **fine-tune** APIs (OpenAI / Anthropic / Bedrock / Vertex
AI / etc.) and local **fine-tune** PEFT methods (LoRA / QLoRA / DoRA),
not training-from-scratch.

Renamed for accuracy:

| Old (`brainwires-training` 0.10.x) | New |
|---|---|
| Cloud fine-tune APIs + dataset pipelines | [`brainwires-finetune`](https://crates.io/crates/brainwires-finetune) (0.11+) |
| Local LoRA / QLoRA / DoRA (Burn-backed) | [`brainwires-finetune-local`](https://crates.io/crates/brainwires-finetune-local) (0.11+) |

The `brainwires-training` name itself is **reserved** for future training-from-scratch primitives. crates.io 0.11.0+ ships an empty placeholder under that name; see [the active crate page](https://crates.io/crates/brainwires-training).

## Migration

```toml
# Before
brainwires-training = "0.10"

# After — pick whichever halves you need:
brainwires-finetune = "0.11"
brainwires-finetune-local = "0.11"
```

```rust
// Before
use brainwires_training::{TrainingManager, CloudFineTuneConfig};
use brainwires_training::local::{BurnBackend, LocalTrainingConfig};

// After
use brainwires_finetune::{TrainingManager, CloudFineTuneConfig};
use brainwires_finetune_local::{BurnBackend, LocalTrainingConfig};
```

## Versioning

The 0.10.x line of `brainwires-training` on crates.io carries the
historical content. v0.10.1 is this deprecation marker (no real code).
v0.11.0+ is the new placeholder crate (also no real code yet, reserved
for training-from-scratch).

## Why a deprecation marker AND a placeholder

The name `brainwires-training` is genuinely useful for the future
training-from-scratch work (full-parameter pretraining, distributed
training, etc.) that the framework will eventually grow. So we kept the
name for that future use while making the historical retirement
explicit via this 0.10.1 marker.
