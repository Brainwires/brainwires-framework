# brainwires-datasets

[![Crates.io](https://img.shields.io/crates/v/brainwires-datasets.svg)](https://crates.io/crates/brainwires-datasets)
[![Documentation](https://img.shields.io/docsrs/brainwires-datasets)](https://docs.rs/brainwires-datasets)
[![License](https://img.shields.io/crates/l/brainwires-datasets.svg)](LICENSE)

Training data pipelines for the Brainwires Agent Framework — JSONL I/O, tokenization, deduplication, format conversion.

## Overview

`brainwires-datasets` handles every step between raw training data and model-ready datasets. It reads and writes JSONL files, converts between popular fine-tuning formats (OpenAI, Together, Alpaca, ShareGPT, ChatML), validates data quality, deduplicates examples, tokenizes text, and splits into train/eval sets.

**Design principles:**

- **Format-agnostic** — a single `TrainingExample` type normalizes all formats; convert freely between them
- **Streaming I/O** — `JsonlReader`/`JsonlWriter` stream line-by-line to handle datasets larger than memory
- **Quality-first** — `DataValidator` catches missing fields, empty messages, and role violations before training
- **Pluggable tokenizers** — HuggingFace Tokenizers and tiktoken via feature flags
- **Deduplication** — exact hash-based dedup to remove repeated examples

```text
  ┌──────────────────────────────────────────────────────────────┐
  │                     brainwires-datasets                       │
  │                                                              │
  │  ┌──────┐    ┌───────────┐    ┌──────────┐    ┌──────────┐  │
  │  │ JSONL │───▶│  Format   │───▶│ Dataset  │───▶│ Quality  │  │
  │  │ Reader│    │ Converter │    │ Instruct │    │ Validator│  │
  │  │ Writer│    │ OpenAI    │    │Preference│    │ Dedup    │  │
  │  └──────┘    │ Together  │    └──────────┘    └────┬─────┘  │
  │              │ Alpaca    │                         │        │
  │              │ ShareGPT  │                         ▼        │
  │              │ ChatML    │                  ┌───────────┐   │
  │              └───────────┘                  │ Tokenizer │   │
  │                                             │ HF / Tik  │   │
  │                                             └─────┬─────┘   │
  │                                                   │         │
  │                                             ┌─────▼─────┐   │
  │                                             │Train/Eval │   │
  │                                             │  Split     │   │
  │                                             └───────────┘   │
  └──────────────────────────────────────────────────────────────┘

  Flow: JSONL → Format Converter → Dataset → Quality/Dedup → Tokenizer → Split
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-datasets = "0.4"
```

Load a JSONL dataset and validate it:

```rust
use brainwires_datasets::{
    JsonlReader, DataValidator, ValidatorConfig,
    InstructDataset, TrainingExample, compute_stats,
};

// Read training examples from JSONL
let examples: Vec<TrainingExample> = JsonlReader::read("data/train.jsonl")?;

// Validate data quality
let validator = DataValidator::new(ValidatorConfig::default());
let report = validator.validate(&examples)?;
println!("Issues: {}", report.issues.len());

// Compute statistics
let stats = compute_stats(&examples);
println!("Examples: {}, avg tokens: {:.0}", stats.total_examples, stats.avg_tokens);

// Build a dataset
let dataset = InstructDataset::from_examples(examples);
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `hf-tokenizer` | Yes | HuggingFace Tokenizers for token counting and BPE tokenization |
| `tiktoken` | No | OpenAI tiktoken tokenizer (`cl100k_base`, `o200k_base`, etc.) |
| `dedup` | No | Exact deduplication via SHA-256 hashing |
| `full` | No | Enables all optional features |

```toml
# With tiktoken for OpenAI token counting
[dependencies]
brainwires-datasets = { version = "0.4", features = ["tiktoken"] }

# Full feature set
[dependencies]
brainwires-datasets = { version = "0.4", features = ["full"] }

# Minimal — no tokenizer, just I/O and format conversion
[dependencies]
brainwires-datasets = { version = "0.4", default-features = false }
```

## Architecture

### Core Types

| Type | Description |
|------|-------------|
| `TrainingExample` | Normalized training example: list of messages with optional metadata |
| `TrainingMessage` | Single message with role and content |
| `TrainingRole` | `System`, `User`, `Assistant`, or `Tool` |
| `PreferencePair` | Chosen + rejected responses for preference tuning (DPO/ORPO) |
| `DataFormat` | Enum of supported formats: `OpenAi`, `Together`, `Alpaca`, `ShareGpt`, `ChatMl` |

### JSONL I/O

| Type | Description |
|------|-------------|
| `JsonlReader` | Streaming line-by-line JSONL reader |
| `JsonlWriter` | Streaming JSONL writer with flush control |
| `read_jsonl` | Convenience function to read all examples from a file |
| `write_jsonl` | Convenience function to write all examples to a file |

### Dataset Abstractions

| Type | Description |
|------|-------------|
| `InstructDataset` | Collection of instruction-following examples (system/user/assistant turns) |
| `PreferenceDataset` | Collection of preference pairs for alignment training |
| `Dataset` | Common trait with `len()`, `get()`, `iter()`, and `shuffle()` |

### Format Converters

All converters implement the `FormatConverter` trait with `to_format()` and `from_format()` methods.

| Converter | Target Format | Notes |
|-----------|---------------|-------|
| `OpenAiFormat` | OpenAI fine-tuning JSONL | `messages` array with role/content |
| `TogetherFormat` | Together AI format | Similar to OpenAI with provider-specific fields |
| `AlpacaFormat` | Stanford Alpaca | `instruction`, `input`, `output` fields |
| `ShareGptFormat` | ShareGPT conversations | `conversations` array with `from`/`value` |
| `ChatMlFormat` | ChatML template | `<\|im_start\|>role\n...<\|im_end\|>` markup |

### Quality Tools

| Type | Description |
|------|-------------|
| `DataValidator` | Validates examples against configurable rules (required fields, role order, length limits) |
| `ValidatorConfig` | Configuration for validation rules |
| `ValidationReport` | Summary of all issues found |
| `DatasetStats` | Statistics: example count, token distribution, role balance |
| `Deduplicator` | SHA-256 based exact deduplication (requires `dedup` feature) |

### Tokenizers

| Type | Feature | Description |
|------|---------|-------------|
| `Tokenizer` | — | Common trait for all tokenizers |
| `HfTokenizer` | `hf-tokenizer` | HuggingFace Tokenizers — any model from the Hub |
| `TiktokenTokenizer` | `tiktoken` | OpenAI tiktoken — `cl100k_base`, `o200k_base` |

### Sampling

| Function | Description |
|----------|-------------|
| `train_eval_split` | Split dataset by ratio with optional shuffle |
| `sample_n` | Random sample of N examples |
| `curriculum_order` | Sort examples by complexity (token count) for curriculum learning |

## Usage Examples

### Format Conversion

```rust
use brainwires_datasets::{
    read_jsonl, write_jsonl,
    OpenAiFormat, AlpacaFormat, FormatConverter,
    TrainingExample,
};

// Read Alpaca-format data
let examples = AlpacaFormat::from_file("data/alpaca.jsonl")?;

// Convert to OpenAI format
let openai_lines: Vec<String> = examples
    .iter()
    .map(|ex| OpenAiFormat::to_string(ex))
    .collect::<Result<Vec<_>>>()?;

write_jsonl(&openai_lines, "data/openai-format.jsonl")?;
```

### Token Counting

```rust
use brainwires_datasets::{Tokenizer, TrainingExample};

#[cfg(feature = "hf-tokenizer")]
{
    use brainwires_datasets::HfTokenizer;
    let tokenizer = HfTokenizer::from_pretrained("bert-base-uncased")?;
    let count = tokenizer.count_tokens("Hello, world!")?;
    println!("Tokens: {count}");
}

#[cfg(feature = "tiktoken")]
{
    use brainwires_datasets::TiktokenTokenizer;
    let tokenizer = TiktokenTokenizer::new("cl100k_base")?;
    let count = tokenizer.count_tokens("Hello, world!")?;
    println!("Tokens: {count}");
}
```

### Deduplication

```rust
#[cfg(feature = "dedup")]
{
    use brainwires_datasets::{Deduplicator, exact_dedup, TrainingExample};

    let examples: Vec<TrainingExample> = load_examples()?;
    let deduped = exact_dedup(&examples);
    println!("Removed {} duplicates", examples.len() - deduped.len());
}
```

### Train/Eval Split

```rust
use brainwires_datasets::{train_eval_split, SplitConfig};

let config = SplitConfig {
    eval_ratio: 0.1,
    shuffle: true,
    seed: Some(42),
};
let split = train_eval_split(&examples, config)?;
println!("Train: {}, Eval: {}", split.train.len(), split.eval.len());
```

### Preference Datasets

```rust
use brainwires_datasets::{PreferencePair, PreferenceDataset};

let pairs = vec![
    PreferencePair {
        prompt: "Explain recursion".into(),
        chosen: "Recursion is when a function calls itself...".into(),
        rejected: "It's a loop thing".into(),
        ..Default::default()
    },
];

let dataset = PreferenceDataset::from_pairs(pairs);
```

### Data Validation

```rust
use brainwires_datasets::{DataValidator, ValidatorConfig, IssueSeverity};

let validator = DataValidator::new(ValidatorConfig {
    max_tokens: Some(4096),
    require_system_message: false,
    ..Default::default()
});

let report = validator.validate(&examples)?;
for issue in &report.issues {
    if issue.severity == IssueSeverity::Error {
        eprintln!("Error in example {}: {}", issue.example_index, issue.message);
    }
}
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.4", features = ["datasets"] }
```

Or depend on `brainwires-datasets` directly for standalone dataset tooling without the rest of the framework.

The `brainwires-training` crate consumes `brainwires-datasets` types directly — datasets flow seamlessly into both cloud and local training pipelines.

## References

### Papers

- [FED: GPU-Accelerated Deduplication Framework](https://arxiv.org/html/2501.01046v2) (Jan 2025) — high-throughput dedup strategies
- [LSHBloom: Internet-Scale Deduplication](https://arxiv.org/html/2411.04257v3) (Nov 2024) — locality-sensitive hashing for massive datasets
- [Linguistic Laws & Subword Tokenization](https://arxiv.org/html/2411.17669v1) (Nov 2024) — analysis of tokenizer behavior
- [DPO: Direct Preference Optimization](https://arxiv.org/abs/2305.18290) (2023) — the preference pair format consumed by `PreferenceDataset`
- [ORPO: Monolithic Preference Optimization](https://arxiv.org/html/2403.07691v2) (2024) — single-stage alignment data format
- [SLM-Bench: Small Language Model Benchmark](https://aclanthology.org/2025.findings-emnlp.1165/) (EMNLP 2025) — evaluation datasets for small models

### Technical Blogs & Guides

- [Modern Tokenization Techniques — CodeSignal](https://codesignal.com/learn/courses/2-modern-tokenization-techniques-for-ai-llms/) — BPE, WordPiece, and SentencePiece
- [Tokenization Deep Dive — Let's Data Science](https://www.letsdatascience.com/blog/tokenization-deep-dive-why-it-matters-more-than-you-think) — why tokenization matters
- [Diffusion Curriculum (DisCL) — ICCV 2025](https://joliang17.github.io/DisCL/) — curriculum learning strategies (cf. `curriculum_order`)
- [Synthetic Data for ML 2025](https://cleverx.com/blog/synthetic-data-for-ml-the-game-changer-in-training-for-2025/) — generating training data

### Data Tools

- [Duplodocus — Allen AI](https://github.com/allenai/duplodocus) — large-scale deduplication
- [fastdedup](https://github.com/wapplewhite4/fastdedup) — fast exact dedup
- [DataTrove — HuggingFace](https://github.com/huggingface/datatrove) — data processing pipelines

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
