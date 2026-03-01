# brainwires-prompting

[![Crates.io](https://img.shields.io/crates/v/brainwires-prompting.svg)](https://crates.io/crates/brainwires-prompting)
[![Documentation](https://img.shields.io/docsrs/brainwires-prompting)](https://docs.rs/brainwires-prompting)
[![License](https://img.shields.io/crates/l/brainwires-prompting.svg)](LICENSE)

Adaptive prompting techniques, task clustering, and temperature optimization for the Brainwires Agent Framework.

## Overview

`brainwires-prompting` implements the research from "Adaptive Selection of Prompting Techniques" (arXiv:2510.18162). It provides 15 proven prompting techniques organized into 4 categories, semantic task clustering to match tasks to optimal techniques, a multi-source selection pipeline (PKS > BKS > cluster default), adaptive temperature optimization, and a learning coordinator that promotes effective patterns to the Behavioral Knowledge System.

**Design principles:**

- **Research-backed** — all 15 techniques drawn from the academic paper, with SEAL quality thresholds for each
- **Multi-source selection** — cascading priority: user preferences (PKS), shared knowledge (BKS), cluster defaults
- **Self-improving** — effectiveness tracking with automatic BKS promotion when reliability thresholds are met
- **WASM-compatible** — the `wasm` feature disables k-means clustering and SQLite storage

```text
  ┌──────────────────────────────────────────────────────────────┐
  │                    brainwires-prompting                       │
  │                                                              │
  │  ┌──────────┐     ┌──────────────┐     ┌─────────────────┐  │
  │  │  Task +  │────►│  Clustering  │────►│    Library       │  │
  │  │  SEAL    │     │  (k-means)   │     │  (15 techniques) │  │
  │  └──────────┘     └──────────────┘     └────────┬────────┘  │
  │                                                  │           │
  │                   ┌──────────────┐     ┌─────────▼────────┐  │
  │                   │  Temperature │────►│   Generator      │  │
  │                   │  Optimizer   │     │  (PKS>BKS>cluster)│  │
  │                   └──────┬───────┘     └─────────┬────────┘  │
  │                          │                       │           │
  │                          │           ┌───────────▼────────┐  │
  │                          │           │  GeneratedPrompt   │  │
  │                          │           │  (system_prompt +  │  │
  │                          │           │   techniques +     │  │
  │                          │           │   temperature)     │  │
  │                          │           └────────────────────┘  │
  │                          │                       │           │
  │                   ┌──────▼───────────────────────▼────────┐  │
  │                   │        Learning Coordinator           │  │
  │                   │  (effectiveness → BKS promotion)      │  │
  │                   └──────────────────┬────────────────────┘  │
  │                                      │                       │
  │                   ┌──────────────────▼────────────────────┐  │
  │                   │        Storage (SQLite)               │  │
  │                   │  clusters · technique perf · temp perf│  │
  │                   └───────────────────────────────────────┘  │
  └──────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-prompting = "0.1"
```

Generate an adaptive prompt for a task:

```rust
use brainwires_prompting::{
    PromptGenerator, TechniqueLibrary, TaskClusterManager, SealProcessingResult,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let library = TechniqueLibrary::new();
    let clusters = TaskClusterManager::new();
    let generator = PromptGenerator::new(library, clusters);

    let seal = SealProcessingResult {
        quality_score: 0.85,
        resolved_query: "Implement a binary search tree in Rust".into(),
    };

    let prompt = generator.generate_prompt(
        "Implement a binary search tree",
        &[],  // task embedding
        Some(&seal),
    ).await;

    println!("System prompt: {}", prompt.system_prompt);
    println!("Techniques: {:?}", prompt.techniques);
    println!("SEAL quality: {}", prompt.seal_quality);

    Ok(())
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Full-featured — k-means clustering via `linfa`, SQLite persistence via `rusqlite` |
| `wasm` | No | WASM target — disables clustering and storage, lean API for browser environments |

```toml
# Default (native with clustering + storage)
brainwires-prompting = "0.1"

# WASM target
brainwires-prompting = { version = "0.1", default-features = false, features = ["wasm"] }
```

## Architecture

### Prompting Techniques

15 techniques organized into 4 categories, each with metadata, templates, and SEAL quality thresholds.

**`TechniqueCategory` variants:**

| Category | Techniques | Description |
|----------|-----------|-------------|
| `RoleAssignment` | RolePlaying | Assign domain-expert roles |
| `EmotionalStimulus` | EmotionPrompting, StressPrompting | Emotional framing for focus |
| `Reasoning` | ChainOfThought, LogicOfThought, LeastToMost, ThreadOfThought, PlanAndSolve, SkeletonOfThought, ScratchpadPrompting | Step-by-step reasoning strategies |
| `Others` | DecomposedPrompting, IgnoreIrrelevantConditions, HighlightedCoT, SkillsInContext, AutomaticInformationFiltering | Complementary techniques |

**`ComplexityLevel` mapping (from SEAL quality):**

| Level | SEAL Quality Range | Description |
|-------|-------------------|-------------|
| `Simple` | < 0.5 | Basic tasks, minimal technique scaffolding |
| `Moderate` | 0.5 – 0.8 | Multi-step tasks, structured reasoning |
| `Advanced` | > 0.8 | Complex tasks, full technique composition |

**`TaskCharacteristic` variants:** `MultiStepReasoning`, `NumericalCalculation`, `LogicalDeduction`, `CreativeGeneration`, `LongContextSummarization`, `SpatialReasoning`, `VisualUnderstanding`, `CodeGeneration`, `AlgorithmicProblem`

**`TechniqueMetadata` fields:**

| Field | Type | Description |
|-------|------|-------------|
| `technique` | `PromptingTechnique` | Technique enum variant |
| `category` | `TechniqueCategory` | Category classification |
| `name` | `String` | Human-readable name |
| `description` | `String` | What the technique does |
| `template` | `String` | Prompt template with `{placeholders}` |
| `best_for` | `Vec<TaskCharacteristic>` | Task types this technique excels at |
| `min_seal_quality` | `f64` | Minimum SEAL quality to enable |
| `bks_promotable` | `bool` | Eligible for BKS promotion |

### Task Clustering

Semantic clustering with SEAL-boosted similarity matching and optional k-means.

**`TaskCluster` fields:**

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Cluster identifier |
| `description` | `String` | Human-readable cluster description |
| `embedding` | `Vec<f32>` | Cluster centroid embedding |
| `techniques` | `Vec<PromptingTechnique>` | Mapped techniques (3–4 per cluster) |
| `example_tasks` | `Vec<String>` | Example tasks in this cluster |
| `seal_query_cores` | `Vec<String>` | SEAL query cores for this cluster |
| `avg_seal_quality` | `f64` | Average SEAL quality observed |
| `recommended_complexity` | `ComplexityLevel` | Suggested complexity for this cluster |

**`TaskClusterManager` key methods:**

| Method | Description |
|--------|-------------|
| `new()` | Create empty manager |
| `add_cluster(cluster)` | Add a cluster |
| `find_matching_cluster(embedding, seal)` | Find best cluster by cosine similarity (10% SEAL boost if quality > 0.7) |
| `build_clusters_from_embeddings(data, k)` | Build clusters via k-means with optimal k selection (native only) |

**Helper functions:**

| Function | Description |
|----------|-------------|
| `cosine_similarity(a, b)` | Cosine similarity between two vectors |

### Technique Library

Complete library of all 15 techniques with BKS integration.

```rust
use brainwires_prompting::{TechniqueLibrary, TechniqueCategory};

let library = TechniqueLibrary::new();

// Get all reasoning techniques
let reasoning = library.get_by_category(TechniqueCategory::Reasoning);

// Get techniques suitable for a SEAL quality score
let suitable = library.get_by_seal_quality(0.75);

// Get BKS-recommended techniques for a cluster
let recommended = library.get_bks_recommended_techniques("code-generation");
```

**Key methods:**

| Method | Description |
|--------|-------------|
| `new()` | Create library with all 15 techniques |
| `with_bks(cache)` | Integrate BehavioralKnowledgeCache for shared knowledge |
| `get_by_category(cat)` | Filter techniques by category |
| `get_by_seal_quality(score)` | Filter by minimum SEAL quality threshold |
| `get_bks_recommended_techniques(cluster_id)` | Query BKS for learned effectiveness |

### Prompt Generator

Multi-source selection pipeline that composes techniques into a final prompt.

**Selection priority (cascading):**

1. **PKS (Personal Knowledge)** — user-specific technique preferences
2. **BKS (Behavioral Knowledge)** — collective learned effectiveness
3. **Cluster default** — techniques mapped to the matched cluster

**Selection rules (from the paper):**

- Always includes one `RoleAssignment` technique (RolePlaying)
- Selects one `EmotionalStimulus` technique, filtered by SEAL quality
- Selects one `Reasoning` technique, matched to complexity level
- Optionally selects `Others` techniques if SEAL quality > 0.6

**`PromptGenerator` methods:**

| Method | Description |
|--------|-------------|
| `new(library, clusters)` | Create generator with technique library and cluster manager |
| `with_bks(cache)` | Add BKS for shared knowledge queries |
| `with_pks(cache)` | Add PKS for user preference queries |
| `generate_prompt(task, embedding, seal)` | Generate adaptive prompt for a task |

**`GeneratedPrompt` fields:**

| Field | Type | Description |
|-------|------|-------------|
| `system_prompt` | `String` | Composed prompt text |
| `cluster_id` | `Option<String>` | Matched cluster |
| `techniques` | `Vec<PromptingTechnique>` | Selected techniques |
| `seal_quality` | `f64` | SEAL quality score used |
| `similarity_score` | `f64` | Cluster match quality |

**Template substitution heuristics:**

| Task Keywords | Inferred Role | Inferred Domain |
|--------------|---------------|-----------------|
| `code`, `implement`, `function` | Software Engineer | Software Development |
| `algorithm`, `data structure` | Computer Scientist | Computer Science |
| `calculate`, `math`, `equation` | Mathematician | Mathematics |
| `analyze`, `review`, `evaluate` | Analyst | Analysis |

### Learning Coordinator

Tracks technique effectiveness and promotes high-performing patterns to BKS.

**`TechniqueStats` fields:**

| Field | Type | Description |
|-------|------|-------------|
| `success_count` | `u32` | Number of successful uses |
| `failure_count` | `u32` | Number of failed uses |
| `avg_iterations` | `f64` | Average iterations to completion (EMA, alpha=0.3) |
| `avg_quality` | `f64` | Average quality score (EMA, alpha=0.3) |
| `last_used` | `DateTime` | Timestamp of last use |

**Key methods:**

| Method | Description |
|--------|-------------|
| `reliability()` | Success rate: `success_count / total_uses` |
| `total_uses()` | `success_count + failure_count` |
| `update(success, iterations, quality)` | Update stats with EMA averaging |

**`PromptingLearningCoordinator` methods:**

| Method | Description |
|--------|-------------|
| `record_outcome(record)` | Log technique effectiveness for a task |
| `should_promote(cluster_id, technique)` | Check if promotion criteria met (≥80% reliability, ≥5 uses) |
| `promote_technique_to_bks(cluster_id, technique)` | Create BehavioralTruth and queue BKS submission |
| `check_and_promote_all()` | Batch promotion of all eligible techniques |
| `get_cluster_summary(cluster_id)` | Stats for a cluster |

**Promotion thresholds (configurable):**

| Parameter | Default | Description |
|-----------|---------|-------------|
| `min_reliability` | 0.8 | 80% success rate required |
| `min_uses` | 5 | Minimum executions before eligible |

**`ClusterSummary` methods:**

| Method | Description |
|--------|-------------|
| `best_technique()` | Most effective technique (minimum 3 samples) |
| `promotable_techniques()` | Techniques eligible for BKS promotion |

### Temperature Optimizer

Adaptive temperature selection per cluster based on the paper's findings.

**Paper findings:**

| Temperature | Best For |
|------------|----------|
| 0.0 | Logical tasks (deduction, boolean, zebra puzzles) |
| 0.2 | Numerical and math tasks |
| 0.6 | Code and programming tasks |
| 0.7 | General-purpose default |
| 1.3 | Linguistic and creative tasks |

**Candidate temperatures:** `[0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.3]`

**`TemperaturePerformance` scoring:**

Combined score = 60% success_rate + 40% avg_quality (both tracked via EMA with alpha=0.3).

**Selection cascade (`get_optimal_temperature()`):**

1. Query BKS for shared temperature knowledge
2. Use local learned optimal (if ≥ `min_samples` data points)
3. Fall back to heuristic based on cluster description keywords

**`TemperatureOptimizer` methods:**

| Method | Description |
|--------|-------------|
| `record_temperature_outcome(cluster, temp, success, quality)` | Log outcome for a temperature |
| `get_optimal_temperature(cluster)` | Get best temperature via cascade |
| `check_and_promote_temperature(cluster, temp)` | Promote to BKS if criteria met |

**Heuristic fallbacks:**

| Keywords | Temperature |
|----------|------------|
| logic, reasoning, deduction, boolean | 0.0 |
| numerical, math, calculation | 0.2 |
| code, programming, implementation | 0.6 |
| creative, story, linguistic | 1.3 |
| (default) | 0.7 |

### SEAL Integration

Stub types for Self-Explanatory Adaptive Learning quality scores.

**`SealProcessingResult` fields:**

| Field | Type | Description |
|-------|------|-------------|
| `quality_score` | `f64` | Confidence in resolved query (0.0–1.0) |
| `resolved_query` | `String` | Refined query from self-explanatory analysis |

SEAL quality drives technique selection, cluster matching boosts, and complexity level inference throughout the system.

### Storage (native only)

SQLite persistence for clusters and performance data.

**Tables:**

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `clusters` | Cluster definitions | id, description, embedding (bincode), techniques (JSON), example_tasks (JSON) |
| `technique_performance` | Per-cluster technique stats | cluster_id, technique, success/failure counts, avg_iterations, avg_quality |
| `temperature_performance` | Per-cluster temperature stats | cluster_id, temperature_key, success_rate, avg_quality, sample_count |

**`ClusterStorage` methods:**

| Method | Description |
|--------|-------------|
| `new(path)` | Open or create SQLite database |
| `save_cluster(cluster)` / `load_cluster(id)` | Cluster CRUD |
| `load_clusters()` | Load all clusters |
| `delete_cluster(id)` | Delete with cascading deletes |
| `save_temperature_performance(...)` | Persist temperature data |
| `load_temperature_performance(cluster)` | Load temperature data |
| `get_stats()` | Returns cluster count, performance record count, DB size |
| `vacuum()` | Reclaim disk space |

## Usage Examples

### Technique Discovery

```rust
use brainwires_prompting::{
    TechniqueLibrary, TechniqueCategory, PromptingTechnique, ComplexityLevel,
};

let library = TechniqueLibrary::new();

// Browse techniques by category
let reasoning = library.get_by_category(TechniqueCategory::Reasoning);
for meta in &reasoning {
    println!("{}: {}", meta.name, meta.description);
}

// Filter by SEAL quality
let advanced = library.get_by_seal_quality(0.85);
for meta in &advanced {
    println!("{} (min SEAL: {})", meta.name, meta.min_seal_quality);
}
```

### Task Clustering and Matching

```rust
use brainwires_prompting::{TaskClusterManager, TaskCluster, SealProcessingResult, cosine_similarity};

let mut manager = TaskClusterManager::new();

// Add a pre-defined cluster
let cluster = TaskCluster {
    id: "code-generation".into(),
    description: "Code generation and implementation tasks".into(),
    embedding: vec![0.1, 0.2, 0.3], // simplified
    techniques: vec![],
    example_tasks: vec!["Implement a REST API".into()],
    ..Default::default()
};
manager.add_cluster(cluster);

// Match a task to a cluster
let seal = SealProcessingResult {
    quality_score: 0.9,
    resolved_query: "Build a REST endpoint".into(),
};
let task_embedding = vec![0.15, 0.22, 0.28];

if let Some((cluster, score)) = manager.find_matching_cluster(&task_embedding, Some(&seal)) {
    println!("Matched: {} (similarity: {:.2})", cluster.id, score);
}
```

### Effectiveness Tracking and BKS Promotion

```rust
use brainwires_prompting::{
    PromptingLearningCoordinator, PromptingTechnique, TechniqueEffectivenessRecord,
};
use chrono::Utc;

let mut coordinator = PromptingLearningCoordinator::new();

// Record outcomes as agents complete tasks
for i in 0..6 {
    coordinator.record_outcome(TechniqueEffectivenessRecord {
        technique: PromptingTechnique::ChainOfThought,
        cluster_id: "algorithm-design".into(),
        task_description: format!("Algorithm task {}", i),
        success: i < 5,  // 5/6 = 83% success
        iterations_used: 8,
        quality_score: 0.85,
        timestamp: Utc::now(),
    });
}

// Check if technique qualifies for BKS promotion
if coordinator.should_promote("algorithm-design", PromptingTechnique::ChainOfThought) {
    coordinator.promote_technique_to_bks(
        "algorithm-design",
        PromptingTechnique::ChainOfThought,
    ).await?;
}

// Or promote all eligible techniques at once
coordinator.check_and_promote_all().await?;
```

### Temperature Optimization

```rust
use brainwires_prompting::TemperatureOptimizer;

let mut optimizer = TemperatureOptimizer::new();

// Record outcomes for different temperatures
optimizer.record_temperature_outcome("code-generation", 0.6, true, 0.9);
optimizer.record_temperature_outcome("code-generation", 0.6, true, 0.85);
optimizer.record_temperature_outcome("code-generation", 0.8, false, 0.4);

// Get optimal temperature (uses cascade: BKS → learned → heuristic)
let temp = optimizer.get_optimal_temperature("code-generation").await;
println!("Optimal temperature: {}", temp); // likely 0.6 based on recorded data

// Promote to BKS if criteria met
optimizer.check_and_promote_temperature("code-generation", 0.6).await?;
```

### Full Pipeline with BKS/PKS

```rust
use brainwires_prompting::{
    PromptGenerator, TechniqueLibrary, TaskClusterManager, SealProcessingResult,
};
use brainwires_knowledge::{BehavioralKnowledgeCache, PersonalKnowledgeCache};

let library = TechniqueLibrary::new();
let clusters = TaskClusterManager::new();

let mut generator = PromptGenerator::new(library, clusters);

// Integrate knowledge systems for multi-source selection
let bks = BehavioralKnowledgeCache::new();
let pks = PersonalKnowledgeCache::new();
generator = generator.with_bks(bks).with_pks(pks);

let seal = SealProcessingResult {
    quality_score: 0.82,
    resolved_query: "Design a concurrent hash map with fine-grained locking".into(),
};

let prompt = generator.generate_prompt(
    "Design a concurrent hash map",
    &[],  // task embedding
    Some(&seal),
).await;

// Use the generated prompt as a system message
println!("Techniques selected: {:?}", prompt.techniques);
println!("Cluster: {:?}", prompt.cluster_id);
println!("System prompt:\n{}", prompt.system_prompt);
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = "0.1"
```

Or use standalone — `brainwires-prompting` depends on `brainwires-core` (the `knowledge` module is now merged into this crate behind the `knowledge` feature).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
