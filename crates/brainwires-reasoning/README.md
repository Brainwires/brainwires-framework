# brainwires-reasoning

[![Crates.io](https://img.shields.io/crates/v/brainwires-reasoning.svg)](https://crates.io/crates/brainwires-reasoning)
[![Documentation](https://img.shields.io/docsrs/brainwires-reasoning)](https://docs.rs/brainwires-reasoning)
[![License](https://img.shields.io/crates/l/brainwires-reasoning.svg)](LICENSE)

Provider-agnostic reasoning and semantic intelligence components for the Brainwires Agent Framework.

## Overview

`brainwires-reasoning` provides local inference components that add semantic understanding to agent workflows: query routing, task complexity scoring, response validation, context summarization, retrieval gating, relevance re-ranking, MDAP strategy selection, and entity extraction. Every component works with any `Provider` implementation (cloud APIs, local LLMs) and gracefully degrades to pattern-based heuristics when inference is unavailable.

**Design principles:**

- **Provider-agnostic** — all components accept `Arc<dyn Provider>` from `brainwires-core`, so they work with Anthropic, OpenAI, Ollama, local LLMs, or any custom backend
- **Graceful degradation** — every async LLM method has a sync heuristic fallback; components never fail, they just lose accuracy
- **Two-tier architecture** — Tier 1 (routing, validation, complexity) for fast decisions; Tier 2 (summarization, retrieval, relevance, strategy, entities) for richer understanding
- **Builder pattern** — every component uses `*Builder` with optional provider; `build()` returns `Option<T>` so missing providers are handled at construction time
- **Confidence scoring** — all results include a `confidence: f32` (0.0–1.0) so callers can decide whether to trust the result or fall back
- **Instrumented** — all inference calls are timed and logged via `tracing` with `InferenceTimer`

```text
  ┌───────────────────────────────────────────────────────────────────────┐
  │                       brainwires-reasoning                            │
  │                                                                       │
  │  ┌─── LocalInferenceConfig ───────────────────────────────────────┐  │
  │  │  tier1_enabled()  tier2_enabled()  all_enabled()               │  │
  │  │  Per-component model overrides (routing_model, etc.)           │  │
  │  └────────────────────────────────────────────────────────────────┘  │
  │                                                                       │
  │  ┌─── TIER 1: Quick Wins (fast, low-latency) ────────────────────┐  │
  │  │                                                                 │  │
  │  │  LocalRouter ──────► ToolCategory classification               │  │
  │  │  ComplexityScorer ──► 0.0–1.0 task difficulty score            │  │
  │  │  LocalValidator ───► Valid / Invalid { reason, severity }      │  │
  │  │                                                                 │  │
  │  │  Default model: lfm2-350m (small, sub-second)                  │  │
  │  └─────────────────────────────────────────────────────────────────┘  │
  │                                                                       │
  │  ┌─── TIER 2: Context & Retrieval (richer understanding) ─────────┐  │
  │  │                                                                 │  │
  │  │  LocalSummarizer ────► context compression + fact extraction   │  │
  │  │  RetrievalClassifier ► None | Low | Medium | High need         │  │
  │  │  RelevanceScorer ────► semantic re-ranking of retrieved items  │  │
  │  │  StrategySelector ───► MDAP decomposition strategy             │  │
  │  │  EntityEnhancer ─────► entities + relationships + concepts     │  │
  │  │                                                                 │  │
  │  │  Default model: lfm2-1.2b (summarization, strategy)           │  │
  │  │                  lfm2-350m (retrieval, relevance, entities)    │  │
  │  └─────────────────────────────────────────────────────────────────┘  │
  │                                                                       │
  │  ┌─── Shared Infrastructure ──────────────────────────────────────┐  │
  │  │  InferenceTimer ──► latency measurement per call               │  │
  │  │  log_inference() ──► structured tracing output                 │  │
  │  │  Heuristic fallbacks ──► regex / keyword / word-overlap        │  │
  │  └────────────────────────────────────────────────────────────────┘  │
  └───────────────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-reasoning = "0.1"
```

Score task complexity and route a query:

```rust
use brainwires_reasoning::{ComplexityScorer, LocalRouter};
use brainwires_core::Provider;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let provider: Arc<dyn Provider> = /* your provider */;

    // Score task complexity
    let scorer = ComplexityScorer::builder()
        .provider(provider.clone())
        .build()
        .unwrap();

    let result = scorer.score("Implement an LRU cache with O(1) operations").await
        .unwrap_or_else(|| scorer.score_heuristic("Implement an LRU cache with O(1) operations"));

    println!("Complexity: {} (confidence: {})", result.score, result.confidence);

    // Route a query to tool categories
    let router = LocalRouter::builder()
        .provider(provider.clone())
        .build()
        .unwrap();

    let route = router.classify("find all TODO comments in the src directory").await;
    if let Some(r) = route {
        println!("Categories: {:?} (confidence: {})", r.categories, r.confidence);
    }

    Ok(())
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Enables `brainwires-core/native`, `brainwires-tools` (for `ToolCategory`), and their transitive dependencies |

```toml
# Default (full functionality)
brainwires-reasoning = "0.1"

# No default features (no tool category routing)
brainwires-reasoning = { version = "0.1", default-features = false }
```

## Architecture

### LocalInferenceConfig

Central configuration controlling which components are active and which models they use.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `routing_enabled` | `bool` | `false` | Enable `LocalRouter` |
| `validation_enabled` | `bool` | `false` | Enable `LocalValidator` |
| `complexity_enabled` | `bool` | `false` | Enable `ComplexityScorer` |
| `summarization_enabled` | `bool` | `false` | Enable `LocalSummarizer` |
| `retrieval_gating_enabled` | `bool` | `false` | Enable `RetrievalClassifier` |
| `relevance_scoring_enabled` | `bool` | `false` | Enable `RelevanceScorer` |
| `strategy_selection_enabled` | `bool` | `false` | Enable `StrategySelector` |
| `entity_enhancement_enabled` | `bool` | `false` | Enable `EntityEnhancer` |
| `routing_model` | `Option<String>` | `None` | Model override for routing (default: `lfm2-350m`) |
| `validation_model` | `Option<String>` | `None` | Model override for validation |
| `complexity_model` | `Option<String>` | `None` | Model override for complexity |
| `summarization_model` | `Option<String>` | `None` | Model override for summarization (default: `lfm2-1.2b`) |
| `retrieval_model` | `Option<String>` | `None` | Model override for retrieval |
| `relevance_model` | `Option<String>` | `None` | Model override for relevance |
| `strategy_model` | `Option<String>` | `None` | Model override for strategy (default: `lfm2-1.2b`) |
| `entity_model` | `Option<String>` | `None` | Model override for entities |
| `log_inference` | `bool` | `false` | Log all inference calls via `tracing` |

**Preset constructors:**

| Preset | Enables | Use Case |
|--------|---------|----------|
| `default()` | Nothing | Zero overhead when reasoning not needed |
| `tier1_enabled()` | Routing, validation, complexity | Fast agent decisions |
| `tier2_enabled()` | All Tier 2 components | Context management and retrieval |
| `all_enabled()` | All components | Full semantic intelligence |
| `routing_only()` | Routing | Query classification only |
| `validation_only()` | Validation | Response quality checks only |
| `summarization_only()` | Summarization | Memory demotion only |

### ComplexityScorer

Assesses task difficulty on a 0.0–1.0 scale for adaptive MDAP voting (k-value adjustment).

| Method | Description |
|--------|-------------|
| `builder()` | Returns `ComplexityScorerBuilder` |
| `score(task)` | Async LLM-based scoring → `Option<ComplexityResult>` |
| `score_heuristic(task)` | Sync keyword-based scoring → `ComplexityResult` |

**`ComplexityResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `score` | `f32` | 0.0 (trivial) to 1.0 (very complex) |
| `confidence` | `f32` | Confidence in the score (0.0–1.0) |
| `used_local_llm` | `bool` | Whether LLM inference was used |

**Scoring guide:**

| Range | Level | Description |
|-------|-------|-------------|
| 0.0–0.2 | Trivial | Single step, no decisions |
| 0.2–0.4 | Simple | Few steps, straightforward |
| 0.4–0.6 | Moderate | Multiple steps, some decisions |
| 0.6–0.8 | Complex | Many steps, careful reasoning |
| 0.8–1.0 | Very Complex | Intricate logic, dependencies |

### LocalRouter

Classifies queries into tool categories for semantic routing (replaces keyword matching).

| Method | Description |
|--------|-------------|
| `builder()` | Returns `LocalRouterBuilder` |
| `classify(query)` | Async classification → `Option<RouteResult>` |

**`RouteResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `categories` | `Vec<ToolCategory>` | Matched tool categories (from `brainwires-tools`) |
| `confidence` | `f32` | Classification confidence (0.0–1.0) |
| `used_local_llm` | `bool` | Whether LLM inference was used |

**`ToolCategory` variants:** `FileOps`, `Search`, `SemanticSearch`, `Git`, `TaskManager`, `AgentPool`, `Web`, `WebSearch`, `Bash`, `Planning`, `Context`, `Orchestrator`, `CodeExecution`.

### LocalValidator

Semantic validation of agent responses to catch issues pattern matching might miss.

| Method | Description |
|--------|-------------|
| `builder()` | Returns `LocalValidatorBuilder` |
| `validate(task, response)` | Async semantic validation → `ValidationResult` |
| `validate_heuristic(task, response)` | Sync pattern-based checks → `ValidationResult` |

**`ValidationResult` enum:**

| Variant | Fields | Description |
|---------|--------|-------------|
| `Valid` | `confidence: f32` | Response passes validation |
| `Invalid` | `reason: String`, `severity: f32`, `confidence: f32` | Issues detected |
| `Skipped` | — | Validation could not be performed |

**Heuristic checks:** off-topic detection, refusal patterns, excessive repetition, insufficient length.

### LocalSummarizer

Generates summaries for tiered memory demotion and extracts key facts from conversations.

| Method | Description |
|--------|-------------|
| `builder()` | Returns `LocalSummarizerBuilder` |
| `summarize_message(content, role)` | Async 50–100 word summary → `Option<SummarizationResult>` |
| `extract_facts(summary)` | Extract key facts → `Option<Vec<ExtractedFact>>` |
| `compact_conversation(messages, keep_recent)` | Emergency context reduction → `Option<String>` |
| `summarize_heuristic(content)` | Sync truncation-based fallback → `SummarizationResult` |
| `extract_entities(content)` | Heuristic entity extraction → `Vec<String>` |

**`SummarizationResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `summary` | `String` | Condensed text |
| `confidence` | `f32` | Summary quality confidence |
| `used_local_llm` | `bool` | Whether LLM was used |

**`ExtractedFact`:**

| Field | Type | Description |
|-------|------|-------------|
| `fact` | `String` | The extracted fact |
| `fact_type` | `FactCategory` | Category: `Decision`, `Definition`, `Requirement`, `CodeChange`, `Configuration`, `Reference`, `Other` |
| `confidence` | `f32` | Extraction confidence |

**Builder options:** `max_summary_tokens` (default 150), `max_facts` (default 5).

### RetrievalClassifier

Classifies whether a query needs retrieval of earlier conversation context.

| Method | Description |
|--------|-------------|
| `builder()` | Returns `RetrievalClassifierBuilder` |
| `classify(query, context_len)` | Async classification → `Option<ClassificationResult>` |
| `classify_heuristic(query, context_len)` | Sync pattern-based fallback → `ClassificationResult` |

**`RetrievalNeed` enum:**

| Variant | `should_retrieve()` | `as_score()` | Description |
|---------|---------------------|--------------|-------------|
| `None` | `false` | `0.0` | No retrieval needed |
| `Low` | `false` | `0.25` | Might benefit from retrieval |
| `Medium` | `true` | `0.6` | Likely needs retrieval |
| `High` | `true` | `0.9` | Definitely needs retrieval |

**`ClassificationResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `need` | `RetrievalNeed` | Retrieval urgency level |
| `confidence` | `f32` | Classification confidence |
| `used_local_llm` | `bool` | Whether LLM was used |
| `intent` | `Option<String>` | Detected intent (LLM-only) |

**Heuristic signals:** reference patterns ("earlier", "before", "remember"), question patterns, continuation patterns, context length adjustment (shorter context → higher retrieval need).

### RelevanceScorer

Re-ranks retrieved context items by semantic relevance to the query (replaces fixed similarity thresholds).

| Method | Description |
|--------|-------------|
| `builder()` | Returns `RelevanceScorerBuilder` |
| `rerank(query, items)` | Async re-ranking → `Vec<RelevanceResult>` |
| `score_relevance(query, content)` | Score a single item → `Option<f32>` |
| `score_heuristic(query, content)` | Sync word-overlap heuristic → `f32` |

**`RelevanceResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `content` | `String` | The content item |
| `original_index` | `usize` | Position in the input list |
| `relevance_score` | `f32` | Re-ranked score (0.0–1.0) |
| `original_score` | `f32` | Original similarity score |
| `used_local_llm` | `bool` | Whether LLM was used |

**Builder options:** `min_score` (default 0.5), `max_items` (default 10).

### StrategySelector

Recommends optimal decomposition strategy for MDAP task execution.

| Method | Description |
|--------|-------------|
| `builder()` | Returns `StrategySelectorBuilder` |
| `select_strategy(task)` | Async selection → `Option<StrategyResult>` |
| `select_heuristic(task)` | Sync pattern-based fallback → `StrategyResult` |

**`TaskType` enum:**

| Variant | Description |
|---------|-------------|
| `Code` | Implementation, refactoring |
| `Planning` | Design, architecture |
| `Analysis` | Research, investigation |
| `Simple` | Single-step tasks |
| `Unknown` | Cannot determine |

**`RecommendedStrategy` enum:**

| Variant | Fields | Description |
|---------|--------|-------------|
| `BinaryRecursive` | `max_depth: u32` | Divide-and-conquer decomposition |
| `Sequential` | — | Step-by-step execution |
| `CodeOperations` | — | File-oriented code changes |
| `None` | — | No decomposition needed |

**`StrategyResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `strategy` | `RecommendedStrategy` | Recommended decomposition strategy |
| `task_type` | `TaskType` | Detected task type |
| `confidence` | `f32` | Selection confidence |
| `used_local_llm` | `bool` | Whether LLM was used |
| `reasoning` | `Option<String>` | Strategy justification (LLM-only) |

### EntityEnhancer

Extracts semantic entities, relationships, and domain concepts from text using LLM (beyond regex patterns).

| Method | Description |
|--------|-------------|
| `builder()` | Returns `EntityEnhancerBuilder` |
| `extract_entities(text)` | Async extraction → `Option<Vec<EnhancedEntity>>` |
| `extract_relationships(entities, context)` | Async extraction → `Option<Vec<EnhancedRelationship>>` |
| `extract_concepts(text)` | Async extraction → `Option<Vec<String>>` |
| `enhance(text)` | Full extraction (entities + relationships + concepts) → `EnhancementResult` |
| `extract_heuristic(text)` | Sync regex-based fallback → `Vec<EnhancedEntity>` |

**`SemanticEntityType` enum:**

| Category | Variants |
|----------|----------|
| Code entities | `File`, `Function`, `Type`, `Variable`, `Module`, `Package` |
| Domain concepts | `Concept`, `Pattern`, `Algorithm`, `Protocol` |
| Actions | `Command`, `Operation`, `Task` |
| Problem/Solution | `Error`, `Bug`, `Fix`, `Feature` |
| People | `Person`, `Role` |
| Resources | `Url`, `Path`, `Identifier` |

**`EnhancedEntity`:**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Entity name |
| `entity_type` | `SemanticEntityType` | Classification |
| `confidence` | `f32` | Extraction confidence |
| `context` | `Option<String>` | Surrounding context |

**`RelationType` enum:**

| Category | Variants |
|----------|----------|
| Structural | `Contains`, `DefinedIn`, `Imports`, `Exports`, `Extends`, `Implements` |
| Behavioral | `Calls`, `Uses`, `Modifies`, `Creates`, `Deletes` |
| Semantic | `RelatedTo`, `SimilarTo`, `DependsOn`, `Causes`, `Fixes`, `Replaces` |

**`EnhancedRelationship`:**

| Field | Type | Description |
|-------|------|-------------|
| `from` | `String` | Source entity |
| `to` | `String` | Target entity |
| `relation_type` | `RelationType` | Relationship classification |
| `confidence` | `f32` | Extraction confidence |

**`EnhancementResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `entities` | `Vec<EnhancedEntity>` | Extracted entities |
| `relationships` | `Vec<EnhancedRelationship>` | Extracted relationships |
| `concepts` | `Vec<String>` | Domain concepts |
| `used_local_llm` | `bool` | Whether LLM was used |

### Shared Infrastructure

**`InferenceTimer`:**

Created with `InferenceTimer::new()`, call `.elapsed_ms()` to get latency for logging.

**`log_inference(task, model, latency_ms, success)`:**

Structured tracing output for all inference calls. Automatically used by all components when `log_inference` is enabled in config.

## Usage Examples

### Score complexity for adaptive MDAP k-value

```rust
use brainwires_reasoning::ComplexityScorer;
use std::sync::Arc;

let scorer = ComplexityScorer::builder()
    .provider(provider.clone())
    .model_id("lfm2-350m".into())
    .build()
    .unwrap();

let result = scorer.score("Implement Dijkstra's shortest path with priority queue").await
    .unwrap_or_else(|| scorer.score_heuristic("Implement Dijkstra's shortest path with priority queue"));

let k = match result.score {
    s if s < 0.3 => 2,  // Simple: minimal voting
    s if s < 0.6 => 3,  // Moderate: default voting
    s if s < 0.8 => 5,  // Complex: more voters
    _ => 7,              // Very complex: maximum voting
};
println!("MDAP k={} for complexity {:.2}", k, result.score);
```

### Validate an agent response

```rust
use brainwires_reasoning::LocalValidator;

let validator = LocalValidator::builder()
    .provider(provider.clone())
    .build()
    .unwrap();

let task = "Write a function to reverse a linked list";
let response = "Here is the implementation...";

match validator.validate(task, response).await {
    brainwires_reasoning::ValidationResult::Valid { confidence } => {
        println!("Response valid (confidence: {:.2})", confidence);
    }
    brainwires_reasoning::ValidationResult::Invalid { reason, severity, .. } => {
        println!("Issue detected (severity {:.1}): {}", severity, reason);
    }
    brainwires_reasoning::ValidationResult::Skipped => {
        // Fall back to heuristic
        let _ = validator.validate_heuristic(task, response);
    }
}
```

### Summarize for memory demotion

```rust
use brainwires_reasoning::LocalSummarizer;

let summarizer = LocalSummarizer::builder()
    .provider(provider.clone())
    .max_summary_tokens(100)
    .max_facts(3)
    .build()
    .unwrap();

// Summarize a message for warm → cold memory demotion
let result = summarizer.summarize_message(
    "We decided to use a B-tree index for the user lookup table because...",
    "assistant",
).await;

if let Some(summary) = result {
    println!("Summary: {}", summary.summary);
}

// Extract key facts
let facts = summarizer.extract_facts("The team chose PostgreSQL for persistence...").await;
if let Some(facts) = facts {
    for fact in &facts {
        println!("[{:?}] {} (confidence: {:.2})", fact.fact_type, fact.fact, fact.confidence);
    }
}
```

### Classify retrieval need

```rust
use brainwires_reasoning::RetrievalClassifier;

let classifier = RetrievalClassifier::builder()
    .provider(provider.clone())
    .build()
    .unwrap();

let result = classifier.classify("What was the auth approach we discussed earlier?", 5).await
    .unwrap_or_else(|| classifier.classify_heuristic(
        "What was the auth approach we discussed earlier?", 5,
    ));

if result.need.should_retrieve() {
    println!("Retrieving context (need: {:.1})", result.need.as_score());
    // ... trigger RAG retrieval
}
```

### Re-rank retrieved context

```rust
use brainwires_reasoning::RelevanceScorer;

let scorer = RelevanceScorer::builder()
    .provider(provider.clone())
    .min_score(0.4)
    .max_items(5)
    .build()
    .unwrap();

let items = vec![
    ("Discussion about database indexing strategies", 0.82),
    ("Meeting notes from last Tuesday", 0.75),
    ("B-tree implementation details", 0.71),
];

let ranked = scorer.rerank("How should we index the users table?", &items).await;
for item in &ranked {
    println!("[{:.2} → {:.2}] {}", item.original_score, item.relevance_score, item.content);
}
```

### Select MDAP decomposition strategy

```rust
use brainwires_reasoning::StrategySelector;

let selector = StrategySelector::builder()
    .provider(provider.clone())
    .build()
    .unwrap();

let result = selector.select_strategy("Refactor the authentication module to use JWT tokens").await
    .unwrap_or_else(|| selector.select_heuristic(
        "Refactor the authentication module to use JWT tokens",
    ));

match result.strategy {
    brainwires_reasoning::RecommendedStrategy::CodeOperations => {
        println!("File-oriented decomposition");
    }
    brainwires_reasoning::RecommendedStrategy::BinaryRecursive { max_depth } => {
        println!("Recursive decomposition (depth: {})", max_depth);
    }
    brainwires_reasoning::RecommendedStrategy::Sequential => {
        println!("Sequential step-by-step execution");
    }
    brainwires_reasoning::RecommendedStrategy::None => {
        println!("No decomposition needed");
    }
}
```

### Extract semantic entities and relationships

```rust
use brainwires_reasoning::EntityEnhancer;

let enhancer = EntityEnhancer::builder()
    .provider(provider.clone())
    .build()
    .unwrap();

let result = enhancer.enhance(
    "The AuthService in src/auth.rs calls validate_token() which depends on the JWT library"
).await;

for entity in &result.entities {
    println!("{} ({:?}, confidence: {:.2})", entity.name, entity.entity_type, entity.confidence);
}

for rel in &result.relationships {
    println!("{} --{:?}--> {}", rel.from, rel.relation_type, rel.to);
}

for concept in &result.concepts {
    println!("Concept: {}", concept);
}
```

### Configure with LocalInferenceConfig

```rust
use brainwires_reasoning::LocalInferenceConfig;

// Enable only Tier 1 for fast agent decisions
let config = LocalInferenceConfig::tier1_enabled();
assert!(config.routing_enabled);
assert!(config.validation_enabled);
assert!(config.complexity_enabled);
assert!(!config.summarization_enabled);

// Enable everything with custom models
let config = LocalInferenceConfig {
    routing_model: Some("lfm2-350m".into()),
    summarization_model: Some("lfm2-1.2b".into()),
    log_inference: true,
    ..LocalInferenceConfig::all_enabled()
};
```

## Integration

Use via the `brainwires` facade crate with the `reasoning` feature, or depend on `brainwires-reasoning` directly:

```toml
# Via facade
[dependencies]
brainwires = { version = "0.1", features = ["reasoning"] }

# Direct
[dependencies]
brainwires-reasoning = "0.1"
```

Re-exports at crate root:

```rust
use brainwires_reasoning::{
    // Configuration
    LocalInferenceConfig,

    // Tier 1
    LocalRouter, RouteResult,
    ComplexityScorer, ComplexityResult,
    LocalValidator, ValidationResult,

    // Tier 2
    LocalSummarizer, SummarizationResult, ExtractedFact, FactCategory,
    RetrievalClassifier, RetrievalNeed, ClassificationResult,
    RelevanceScorer, RelevanceResult,
    StrategySelector, StrategyResult, TaskType, RecommendedStrategy,
    EntityEnhancer, EnhancementResult, EnhancedEntity, EnhancedRelationship,
    SemanticEntityType, RelationType,

    // Infrastructure
    InferenceTimer, log_inference,
};
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
