# brainwires-training

[![Crates.io](https://img.shields.io/crates/v/brainwires-training.svg)](https://crates.io/crates/brainwires-training)
[![Documentation](https://img.shields.io/docsrs/brainwires-training)](https://docs.rs/brainwires-training)
[![License](https://img.shields.io/crates/l/brainwires-training.svg)](LICENSE)

Model training and fine-tuning for the Brainwires Agent Framework — cloud fine-tuning and local LoRA/QLoRA/DoRA training.

## Overview

`brainwires-training` provides a unified `TrainingManager` that dispatches fine-tuning jobs to either cloud providers (OpenAI, Together, Fireworks, Anyscale, AWS Bedrock, Google Vertex AI) or a local Burn-based training backend supporting LoRA, QLoRA, and DoRA adapter methods with DPO/ORPO alignment losses.

**Design principles:**

- **Dual-path** — one API for cloud fine-tuning and local adapter training; switch with a feature flag
- **Provider-agnostic** — `FineTuneProvider` trait abstracts all cloud APIs behind `submit`, `status`, `cancel`, `download`
- **Adapter-first** — local training uses parameter-efficient adapters (LoRA/QLoRA/DoRA) rather than full fine-tuning
- **Dataset-integrated** — consumes `brainwires-datasets` types directly for seamless data → training pipelines
- **Observable** — `TrainingProgress` and `TrainingMetrics` provide real-time job monitoring

```text
  ┌─────────────────────────────────────────────────────────────┐
  │                    brainwires-training                       │
  │                                                             │
  │                  ┌────────────────┐                         │
  │                  │TrainingManager │                         │
  │                  └───────┬────────┘                         │
  │                          │                                  │
  │              ┌───────────┴───────────┐                      │
  │              ▼                       ▼                      │
  │  ┌──────────────────┐   ┌───────────────────┐              │
  │  │  Cloud Providers  │   │  Local Burn Backend│             │
  │  │  ┌──────────────┐ │   │  ┌──────────────┐  │            │
  │  │  │   OpenAI     │ │   │  │   Adapters   │  │            │
  │  │  │   Together   │ │   │  │  LoRA/QLoRA  │  │            │
  │  │  │   Fireworks  │ │   │  │    DoRA      │  │            │
  │  │  │   Anyscale   │ │   │  └──────────────┘  │            │
  │  │  │   Bedrock    │ │   │  ┌──────────────┐  │            │
  │  │  │   Vertex     │ │   │  │  Alignment   │  │            │
  │  │  └──────────────┘ │   │  │  DPO / ORPO  │  │            │
  │  └──────────────────┘   │  └──────────────┘  │            │
  │                          │  ┌──────────────┐  │            │
  │                          │  │Checkpointing │  │            │
  │                          │  │   Export      │  │            │
  │                          │  └──────────────┘  │            │
  │                          └───────────────────┘             │
  │                                  │                         │
  │                                  ▼                         │
  │                        ┌─────────────────┐                 │
  │                        │ TrainedModel    │                 │
  │                        │  Artifact       │                 │
  │                        └─────────────────┘                 │
  └─────────────────────────────────────────────────────────────┘

  Flow: Dataset → TrainingManager → Cloud / Local → Artifacts
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-training = "0.1"
```

Submit a cloud fine-tuning job:

```rust
use brainwires_training::{
    TrainingManager, CloudFineTuneConfig, FineTuneProviderFactory,
    TrainingHyperparams, TrainingJobStatus,
};

// Create a provider (OpenAI in this example)
let provider = FineTuneProviderFactory::create("openai", "your-api-key")?;

// Configure the job
let config = CloudFineTuneConfig {
    model: "gpt-4o-mini-2024-07-18".into(),
    training_file: "data/train.jsonl".into(),
    validation_file: Some("data/eval.jsonl".into()),
    hyperparams: TrainingHyperparams {
        epochs: Some(3),
        learning_rate: Some(1e-5),
        batch_size: Some(4),
        ..Default::default()
    },
    ..Default::default()
};

// Submit and monitor
let job_id = provider.submit(config).await?;
loop {
    let status = provider.status(&job_id).await?;
    match status {
        TrainingJobStatus::Completed => break,
        TrainingJobStatus::Failed(err) => return Err(err.into()),
        _ => tokio::time::sleep(std::time::Duration::from_secs(30)).await,
    }
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `cloud` | Yes | Cloud fine-tuning via `reqwest` — OpenAI, Together, Fireworks, Anyscale, Bedrock, Vertex |
| `local` | No | Local adapter training via the Burn framework (LoRA, QLoRA, DoRA) |
| `full` | No | Enables both `cloud` and `local` |

```toml
# Local training only (no cloud deps)
[dependencies]
brainwires-training = { version = "0.1", default-features = false, features = ["local"] }

# Full — cloud + local
[dependencies]
brainwires-training = { version = "0.1", features = ["full"] }
```

## Architecture

### TrainingManager

Central coordinator that dispatches to the correct backend based on configuration.

| Method | Description |
|--------|-------------|
| `submit_cloud` | Submit a job to a cloud fine-tuning provider |
| `submit_local` | Start a local adapter training run |
| `status` | Query job status by ID |
| `cancel` | Cancel a running job |
| `list_jobs` | List all jobs with optional status filter |

### Cloud Providers

All providers implement the `FineTuneProvider` trait:

```rust
#[async_trait]
pub trait FineTuneProvider: Send + Sync {
    async fn submit(&self, config: CloudFineTuneConfig) -> Result<TrainingJobId>;
    async fn status(&self, job_id: &TrainingJobId) -> Result<TrainingJobStatus>;
    async fn cancel(&self, job_id: &TrainingJobId) -> Result<()>;
    async fn download(&self, job_id: &TrainingJobId) -> Result<TrainedModelArtifact>;
}
```

| Provider | API | Supported Models |
|----------|-----|------------------|
| OpenAI | Fine-tuning API | GPT-4o-mini, GPT-4o, GPT-3.5-turbo |
| Together | Fine-tuning API | Llama, Mistral, CodeLlama, etc. |
| Fireworks | Fine-tuning API | Llama, Mixtral, custom |
| Anyscale | Fine-tuning API | Open-source models |
| Bedrock | Custom model training | Titan, Llama (via AWS) |
| Vertex | Model tuning API | Gemini, PaLM |

### CloudFineTuneConfig

| Field | Type | Description |
|-------|------|-------------|
| `model` | `String` | Base model identifier |
| `training_file` | `String` | Path to JSONL training data |
| `validation_file` | `Option<String>` | Optional validation data |
| `hyperparams` | `TrainingHyperparams` | Epochs, learning rate, batch size, etc. |
| `suffix` | `Option<String>` | Custom suffix for the fine-tuned model name |

### Local Training (Burn Backend)

Requires the `local` feature. Uses the [Burn](https://burn.dev/) deep learning framework for GPU-accelerated adapter training.

#### Adapter Methods

| Method | Description |
|--------|-------------|
| `LoRA` | Low-Rank Adaptation — adds small trainable matrices to frozen weights |
| `QLoRA` | Quantized LoRA — 4-bit base model with LoRA adapters |
| `DoRA` | Weight-Decomposed Low-Rank Adaptation — direction + magnitude decomposition |

#### LoraConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `rank` | `usize` | `8` | Rank of low-rank matrices (lower = fewer params) |
| `alpha` | `f64` | `16.0` | Scaling factor (alpha/rank) |
| `dropout` | `f64` | `0.05` | Dropout applied to adapter layers |
| `target_modules` | `Vec<String>` | `["q_proj", "v_proj"]` | Which layers to adapt |

#### Alignment Methods

| Method | Description |
|--------|-------------|
| `DPO` | Direct Preference Optimization — learn from chosen/rejected pairs |
| `ORPO` | Odds Ratio Preference Optimization — single-stage alignment |

#### LocalTrainingConfig

| Field | Type | Description |
|-------|------|-------------|
| `adapter` | `AdapterMethod` | `LoRA`, `QLoRA`, or `DoRA` |
| `alignment` | `Option<AlignmentMethod>` | Optional DPO/ORPO alignment |
| `device` | `ComputeDevice` | `Cpu`, `Cuda(id)`, `Wgpu`, `Metal` |
| `hyperparams` | `TrainingHyperparams` | Epochs, learning rate, batch size, scheduler |
| `checkpoint_dir` | `Option<String>` | Directory for periodic checkpoint saves |
| `lora` | `LoraConfig` | LoRA-specific configuration |

#### LrScheduler

| Scheduler | Description |
|-----------|-------------|
| `Constant` | Fixed learning rate |
| `Cosine` | Cosine annealing to zero |
| `Linear` | Linear decay to zero |
| `OneCycle` | 1cycle policy (warmup + annealing) |

### Job Tracking

| Type | Description |
|------|-------------|
| `TrainingJobId` | Unique job identifier |
| `TrainingJobStatus` | `Pending`, `Running`, `Completed`, `Failed(String)`, `Canceled` |
| `TrainingProgress` | Step count, epoch, loss, learning rate |
| `TrainingMetrics` | Final metrics: train loss, eval loss, duration |
| `TrainedModelArtifact` | Path to exported model weights + adapter config |

## Usage Examples

### Local LoRA Training

```rust
#[cfg(feature = "local")]
{
    use brainwires_training::{
        TrainingManager, LocalTrainingConfig, LoraConfig,
        AdapterMethod, ComputeDevice, TrainingHyperparams, LrScheduler,
    };

    let config = LocalTrainingConfig {
        adapter: AdapterMethod::LoRA,
        device: ComputeDevice::Wgpu,
        lora: LoraConfig {
            rank: 16,
            alpha: 32.0,
            dropout: 0.05,
            target_modules: vec!["q_proj".into(), "k_proj".into(), "v_proj".into()],
        },
        hyperparams: TrainingHyperparams {
            epochs: Some(3),
            learning_rate: Some(2e-4),
            batch_size: Some(8),
            lr_scheduler: Some(LrScheduler::Cosine),
            ..Default::default()
        },
        checkpoint_dir: Some("checkpoints/".into()),
        ..Default::default()
    };

    let manager = TrainingManager::new();
    let job_id = manager.submit_local(config, &dataset).await?;

    // Monitor progress
    let progress = manager.status(&job_id).await?;
    println!("Step {}, loss: {:.4}", progress.step, progress.loss);
}
```

### DPO Alignment Training

```rust
#[cfg(feature = "local")]
{
    use brainwires_training::{
        LocalTrainingConfig, AdapterMethod, AlignmentMethod,
        ComputeDevice, TrainingHyperparams,
    };
    use brainwires_datasets::PreferenceDataset;

    let config = LocalTrainingConfig {
        adapter: AdapterMethod::LoRA,
        alignment: Some(AlignmentMethod::Dpo),
        device: ComputeDevice::Cuda(0),
        hyperparams: TrainingHyperparams {
            epochs: Some(1),
            learning_rate: Some(5e-5),
            batch_size: Some(4),
            ..Default::default()
        },
        ..Default::default()
    };

    let preference_data = PreferenceDataset::from_file("data/preferences.jsonl")?;
    let job_id = manager.submit_local(config, &preference_data).await?;
}
```

### Cloud Provider Selection

```rust
#[cfg(feature = "cloud")]
{
    use brainwires_training::{FineTuneProviderFactory, CloudFineTuneConfig};

    // OpenAI
    let openai = FineTuneProviderFactory::create("openai", "sk-...")?;

    // Together AI
    let together = FineTuneProviderFactory::create("together", "tok-...")?;

    // AWS Bedrock
    let bedrock = FineTuneProviderFactory::create("bedrock", "aws-credentials")?;

    // Same config works with any provider
    let config = CloudFineTuneConfig {
        model: "meta-llama/Llama-3-8b".into(),
        training_file: "data/train.jsonl".into(),
        ..Default::default()
    };
    let job_id = together.submit(config).await?;
}
```

### Monitoring Job Progress

```rust
use brainwires_training::{TrainingManager, TrainingJobStatus};

let manager = TrainingManager::new();
let status = manager.status(&job_id).await?;

match status {
    TrainingJobStatus::Running => {
        let progress = manager.progress(&job_id).await?;
        println!(
            "Epoch {}/{}, step {}, loss: {:.4}, lr: {:.2e}",
            progress.epoch, progress.total_epochs,
            progress.step, progress.loss, progress.learning_rate
        );
    }
    TrainingJobStatus::Completed => {
        let metrics = manager.metrics(&job_id).await?;
        println!(
            "Done! Train loss: {:.4}, eval loss: {:.4}, duration: {:?}",
            metrics.train_loss, metrics.eval_loss, metrics.duration
        );
    }
    TrainingJobStatus::Failed(err) => eprintln!("Failed: {err}"),
    _ => {}
}
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["training"] }
```

Or depend on `brainwires-training` directly for standalone training capabilities. The crate depends on `brainwires-datasets` for data types, so both are pulled in together.

## References

### Papers

- [DPO: Direct Preference Optimization](https://arxiv.org/abs/2305.18290) (2023) — the alignment method behind `AlignmentMethod::Dpo`
- [ORPO: Monolithic Preference Optimization](https://arxiv.org/html/2403.07691v2) (2024) — single-stage alignment, `AlignmentMethod::Orpo`
- [Cramming: Training on a Single GPU in One Day](https://openreview.net/forum?id=gUL6zYN4Uaf) (2023) — efficient small-model training strategies
- [Ring Attention: Near-Infinite Context](https://proceedings.iclr.cc/paper_files/paper/2024/file/1119587863e78451f080da2a768c4935-Paper-Conference.pdf) (ICLR 2024) — long-context training
- [Evolution Strategies for Billion-Parameter Fine-Tuning](https://arxiv.org/abs/2509.24372) (Sept 2025)
- [IR-Tuning: Efficient Layer-wise Fine-tuning](https://arxiv.org/abs/2510.00268) (Sept 2025)
- [QTHA: Quantum-Enhanced Fine Tuning](https://arxiv.org/abs/2503.12790) (March 2025)
- [Complexity-aware Fine-tuning](https://arxiv.org/abs/2506.21220) (June 2025)
- [LUNE: Efficient LLM Unlearning via LoRA](https://arxiv.org/html/2512.07375v1) (Dec 2025)
- [Mamba-3: State Space Models](https://openreview.net/forum?id=HwCvaJOiCj) (2025)
- [Revisiting Chinchilla Scaling Laws](https://aclanthology.org/2025.acl-long.1163.pdf) (ACL 2025)

### Technical Blogs & Guides

- [LoRA/QLoRA/DoRA Production Guide](https://medium.com/@abhi-84/lora-qlora-dora-rslora-the-complete-guide-to-7-production-ready-fine-tuning-variants-283ff3e574a3) (Dec 2025) — comprehensive adapter comparison
- [DoRA — NVIDIA Technical Blog](https://developer.nvidia.com/blog/introducing-dora-a-high-performing-alternative-to-lora-for-fine-tuning/) — weight-decomposed low-rank adaptation
- [How to Align LLMs in 2025 with DPO](https://www.philschmid.de/rl-with-llms-in-2025-dpo) — practical DPO guide
- [Small LLM Training Guide 2026](https://blog.premai.io/how-to-train-a-small-language-model-the-complete-guide/) — end-to-end training walkthrough
- [Mixed Precision: FP8 vs BF16](https://acecloud.ai/blog/fp8-vs-bf16-mixed-precision-tensor-cores/) — precision trade-offs
- [NVIDIA FP8 Training Blog](https://developer.nvidia.com/blog/floating-point-8-an-introduction-to-efficient-lower-precision-ai-training/)
- [FSDP vs DeepSpeed — HuggingFace](https://huggingface.co/docs/accelerate/en/concept_guides/fsdp_and_deepspeed) — distributed training comparison

### Rust ML Ecosystem

- [Burn Framework](https://burn.dev/) — the local training backend used by this crate
- [Candle — HuggingFace](https://github.com/huggingface/candle) — Rust inference framework
- [tch-rs: PyTorch Rust Bindings](https://github.com/LaurentMazare/tch-rs)
- [PyO3 for AI/Data Science](https://medium.com/@muruganantham52524/why-python-developers-are-turning-to-rust-with-pyo3-for-faster-ai-and-data-science-in-2025-cd5991973a4d) (2025)
- [Rust + CUDA for ML](https://dasroot.net/posts/2025/12/rust-cuda-gpu-programming-ml-applications/) (2025)

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
