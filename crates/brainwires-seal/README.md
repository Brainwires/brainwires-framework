# brainwires-seal

[![Crates.io](https://img.shields.io/crates/v/brainwires-seal.svg)](https://crates.io/crates/brainwires-seal)
[![Documentation](https://img.shields.io/docsrs/brainwires-seal)](https://docs.rs/brainwires-seal)
[![License](https://img.shields.io/crates/l/brainwires-seal.svg)](LICENSE)

Self-Evolving Agentic Learning (SEAL) integration for the Brainwires Agent Framework.

## Overview

`brainwires-seal` implements the SEAL framework for enhancing conversational question answering and agent decision-making. The crate provides coreference resolution, structured query extraction, self-evolving pattern learning, and post-execution reflection — enabling agents to understand implicit references, build reusable knowledge, and correct their own mistakes without retraining.

Inspired by the research paper:

> **SEAL: Self-Evolving Agentic Learning for Conversational Question Answering over Knowledge Graphs**
> Hao Wang, Jialun Zhong, Changcheng Wang, Zhujun Nie, Zheng Li, Shunyu Yao, Yanzeng Li, Xinchi Li
> arXiv:2512.04868, December 2024
> https://arxiv.org/abs/2512.04868

The paper introduces a two-stage agentic framework that decomposes semantic parsing into semantic extraction and template-based completion, with local/global memory and a reflection module for continuous adaptation from dialog history and execution feedback.

**Design principles:**

- **Two-stage processing** — separates coreference resolution and query core extraction into distinct pipeline stages for structural fidelity and linking efficiency
- **Self-evolving memory** — dual-level memory (local per-session, global cross-session) learns from successful interactions without model retraining
- **Reflection-driven correction** — post-execution analysis detects errors (empty results, overflow, entity mismatches) and suggests corrections with automatic retry
- **Salience-based resolution** — coreference uses weighted salience scoring (recency, frequency, graph centrality, type match, syntactic prominence) for accurate reference resolution
- **Knowledge system integration** — optional bidirectional bridge with BKS/PKS promotes high-reliability patterns to collective knowledge and retrieves contextual truths

```text
  ┌───────────────────────────────────────────────────────────────────────┐
  │                          brainwires-seal                              │
  │                                                                       │
  │  User Query                                                           │
  │      │                                                                │
  │      ▼                                                                │
  │  ┌─── Coreference Resolution ───────────────────────────────────────┐│
  │  │  detect_references() → resolve() → rewrite_with_resolutions()    ││
  │  │  "What uses it?" → "What uses [main.rs]?"                        ││
  │  │  Salience: recency(0.35) + frequency(0.15) + centrality(0.20)    ││
  │  │            + type_match(0.20) + syntactic(0.10)                   ││
  │  └──────────────────────────────────────┬────────────────────────────┘│
  │                                         │                             │
  │                                         ▼                             │
  │  ┌─── Query Core Extraction ────────────────────────────────────────┐│
  │  │  classify() → build_expression() → QueryCore                     ││
  │  │  S-expression output: (JOIN DependsOn ?dep "main.rs")            ││
  │  │  Question types: Definition, Location, Dependency, Count,        ││
  │  │                  Superlative, Enumeration, Boolean, MultiHop      ││
  │  └──────────────────────────────────────┬────────────────────────────┘│
  │                                         │                             │
  │                                         ▼                             │
  │  ┌─── Learning Coordinator ─────────────────────────────────────────┐│
  │  │  Local Memory (per-session)  │  Global Memory (cross-session)    ││
  │  │  Entity tracking, focus      │  Query patterns, tool errors      ││
  │  │  Resolution history          │  Resolution patterns, templates   ││
  │  │  ─────────────────────────────────────────────────────────────── ││
  │  │  process_query() → match pattern or create new                   ││
  │  │  record_outcome() → update reliability scores                    ││
  │  └──────────────────────────────────────┬────────────────────────────┘│
  │                                         │                             │
  │                                         ▼                             │
  │  ┌─── Reflection Module ────────────────────────────────────────────┐│
  │  │  analyze() → detect issues → suggest fixes → attempt_correction  ││
  │  │  Errors: EmptyResult, Overflow, EntityNotFound, RelationMismatch ││
  │  │  Fixes: RetryWithQuery, ExpandScope, NarrowScope, ResolveEntity  ││
  │  └──────────────────────────────────────────────────────────────────┘│
  │                                                                       │
  │  ┌─── Knowledge Integration (optional) ─────────────────────────────┐│
  │  │  SEAL ↔ BKS/PKS bidirectional bridge                             ││
  │  │  Pattern promotion • Confidence harmonization • Entity tracking   ││
  │  └──────────────────────────────────────────────────────────────────┘│
  └───────────────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-seal = "0.1"
```

Process a user query through the SEAL pipeline:

```rust
use brainwires_seal::{SealProcessor, SealConfig, DialogState};
use brainwires_core::graph::{EntityStore, RelationshipGraph};

fn main() -> anyhow::Result<()> {
    let mut processor = SealProcessor::with_defaults();
    processor.init_conversation("session-001");

    // Set up dialog state with active entities
    let mut dialog_state = DialogState::default();
    dialog_state.current_turn = 3;
    dialog_state.focus_stack.push("main.rs".to_string());

    let entity_store = EntityStore::new();
    let graph = RelationshipGraph::new();

    // Process a query with an unresolved reference
    let result = processor.process(
        "What uses it?",
        &dialog_state,
        &entity_store,
        Some(&graph),
    )?;

    println!("Resolved: {}", result.resolved_query);
    // → "What uses [main.rs]?"

    if let Some(ref core) = result.query_core {
        println!("Query type: {:?}", core.question_type);
        println!("S-expression: {}", core.to_sexp());
    }

    println!("Quality: {:.2}", result.quality_score);

    Ok(())
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Enables `brainwires-core/native` and transitive dependencies |
| `mdap` | No | Enables MDAP metric recording via `brainwires-mdap` |
| `knowledge` | No | Enables BKS/PKS knowledge system integration via `brainwires-prompting` (knowledge feature) and `tokio` |

```toml
# Default (core SEAL processing)
brainwires-seal = "0.2"

# With knowledge system integration
brainwires-seal = { version = "0.2", features = ["knowledge"] }

# With MDAP metric recording
brainwires-seal = { version = "0.2", features = ["mdap"] }

# Everything enabled
brainwires-seal = { version = "0.2", features = ["knowledge", "mdap"] }
```

## Architecture

### SealConfig

Central configuration controlling which pipeline stages are active and their thresholds.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_coreference` | `bool` | `true` | Enable coreference resolution stage |
| `enable_query_cores` | `bool` | `true` | Enable query core extraction stage |
| `enable_learning` | `bool` | `true` | Enable self-evolving learning |
| `enable_reflection` | `bool` | `true` | Enable reflection analysis |
| `max_reflection_retries` | `u32` | `2` | Maximum correction attempts per query |
| `min_coreference_confidence` | `f32` | `0.5` | Minimum confidence to accept a resolution |
| `min_pattern_reliability` | `f32` | `0.7` | Minimum pattern reliability for reuse |

### SealProcessor

Main orchestrator that chains all four pipeline stages.

| Method | Description |
|--------|-------------|
| `new(config)` | Create processor with custom configuration |
| `with_defaults()` | Create processor with all stages enabled |
| `init_conversation(id)` | Set conversation ID for learning coordinator |
| `process(query, dialog, entities, graph)` | Run full pipeline → `SealProcessingResult` |
| `record_outcome(pattern_id, success, count, core)` | Record execution result for learning |
| `reflect(core, result, graph)` | Post-execution reflection → `ReflectionReport` |
| `get_learning_context()` | Retrieve learned patterns for prompt injection |
| `coreference()` | Access `CoreferenceResolver` |
| `query_extractor()` | Access `QueryCoreExtractor` |
| `learning_mut()` | Mutable access to `LearningCoordinator` |
| `reflection()` | Access `ReflectionModule` |
| `record_mdap_metrics(metrics)` | Record MDAP execution patterns (requires `mdap` feature) |

**`SealProcessingResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `original_query` | `String` | Raw user input |
| `resolved_query` | `String` | Query with coreferences resolved to `[entity]` notation |
| `query_core` | `Option<QueryCore>` | Structured query expression (if extracted) |
| `matched_pattern` | `Option<String>` | Learning pattern ID (if matched) |
| `resolutions` | `Vec<ResolvedReference>` | All coreference resolutions applied |
| `quality_score` | `f32` | Reflection quality score (0.0–1.0) |
| `issues` | `Vec<Issue>` | Problems detected by reflection |

### CoreferenceResolver

Resolves anaphoric references ("it", "the file", "that function") to concrete entities from dialog history using salience-weighted scoring.

| Method | Description |
|--------|-------------|
| `detect_references(message)` | Find unresolved references via regex → `Vec<UnresolvedReference>` |
| `resolve(refs, dialog, entities, graph)` | Resolve references to entities → `Vec<ResolvedReference>` |
| `rewrite_with_resolutions(message, resolutions)` | Replace references with `[entity]` → `String` |

**`ReferenceType` enum:**

| Variant | Examples | Description |
|---------|----------|-------------|
| `SingularNeutral` | "it", "this", "that" | Singular neuter pronouns |
| `Plural` | "they", "them", "those" | Plural references |
| `DefiniteNP` | "the file", "the function" | Definite noun phrases with entity type |
| `Demonstrative` | "that error", "this type" | Demonstrative references |
| `Ellipsis` | (implied subject) | Implied missing subject |

**`SalienceScore` weights:**

| Factor | Weight | Description |
|--------|--------|-------------|
| Recency | 0.35 | Recently mentioned entities rank higher |
| Frequency | 0.15 | Frequently mentioned entities rank higher |
| Graph centrality | 0.20 | Entities central in the relationship graph |
| Type match | 0.20 | Entity type compatibility with reference |
| Syntactic prominence | 0.10 | Subjects rank higher than objects |

### QueryCoreExtractor

Converts natural language queries into structured S-expression-like representations for graph traversal.

| Method | Description |
|--------|-------------|
| `extract(query, entities)` | Extract query core → `Option<QueryCore>` |

**`QuestionType` enum:**

| Variant | Pattern | Example |
|---------|---------|---------|
| `Definition` | "What is X?" | "What is the Provider trait?" |
| `Location` | "Where is X defined?" | "Where is main defined?" |
| `Dependency` | "What uses/depends on X?" | "What depends on core?" |
| `Count` | "How many X?" | "How many tests are there?" |
| `Superlative` | "Which X has most Y?" | "Which file has most functions?" |
| `Enumeration` | "List all X" | "List all modules" |
| `Boolean` | "Does X use Y?" | "Does auth use JWT?" |
| `MultiHop` | Complex multi-step | "What calls functions in main.rs?" |
| `Unknown` | Unclassified | Fallback |

**`QueryExpr` variants:**

| Variant | Description |
|---------|-------------|
| `Variable(name)` | Placeholder like `?file`, `?function` |
| `Constant(value, EntityType)` | Literal like `"main.rs"` with type |
| `Op(QueryOp)` | Complex operation (Join, And, Or, Filter, Count, etc.) |

**`QueryOp` operations:**

| Operation | S-expression | Description |
|-----------|-------------|-------------|
| `Join` | `(JOIN relation subject object)` | Traverse a relationship |
| `And` | `(AND expr1 expr2 ...)` | Logical AND |
| `Or` | `(OR expr1 expr2 ...)` | Logical OR |
| `Filter` | `(FILTER source predicate)` | Apply predicate to results |
| `Count` | `(COUNT expr)` | Count matching results |
| `Superlative` | `(ARGMAX expr property)` | Find max/min by property |
| `Values` | `(VALUES v1 v2 ...)` | Literal value list |

**`RelationType` variants:** `Contains`, `References`, `DependsOn`, `Modifies`, `Defines`, `CoOccurs`, `HasType`, `HasError`, `CreatedAt`, `ModifiedAt`.

### LearningCoordinator

Dual-level memory system that learns from successful interactions without retraining.

| Method | Description |
|--------|-------------|
| `new(conversation_id)` | Create coordinator for a session |
| `process_query(original, resolved, core, turn)` | Check for matching patterns → `Option<QueryPattern>` |
| `record_outcome(pattern_id, success, count, core)` | Record execution result |
| `get_context_for_prompt()` | Retrieve learned patterns for prompt injection → `String` |
| `get_promotable_patterns(threshold, min_uses)` | Get patterns eligible for BKS promotion → `Vec<QueryPattern>` |

**`LocalMemory` (per-session):**

| Field | Type | Description |
|-------|------|-------------|
| `conversation_id` | `String` | Session identifier |
| `entities` | `HashMap<String, TrackedEntity>` | Entities mentioned in session |
| `coreference_log` | `Vec<ResolvedReference>` | Resolution history |
| `query_history` | `Vec<QueryCore>` | Queries executed |
| `focus_stack` | `Vec<String>` | Currently active entities |
| `current_turn` | `usize` | Conversation progress |

**`GlobalMemory` (cross-session):**

| Field | Type | Description |
|-------|------|-------------|
| `query_patterns` | `Vec<QueryPattern>` | Learned query templates |
| `resolution_patterns` | `Vec<ResolutionPattern>` | Reference → entity mappings |
| `tool_error_patterns` | `Vec<ToolErrorPattern>` | Tool failure patterns |
| `tool_stats` | `HashMap<String, ToolStats>` | Tool usage statistics |

**`QueryPattern`:**

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Unique pattern identifier |
| `question_type` | `QuestionType` | Question type this applies to |
| `template` | `String` | Template string with placeholders |
| `required_entity_types` | `Vec<EntityType>` | Entity types required |
| `success_count` | `u32` | Successful uses |
| `failure_count` | `u32` | Failed uses |
| `reliability()` | `f32` | `success / (success + failure)` |

### ReflectionModule

Post-execution analysis for error detection and automatic correction.

| Method | Description |
|--------|-------------|
| `new(config)` | Create with custom configuration |
| `analyze(core, result, graph)` | Detect issues → `ReflectionReport` |
| `validate_query_core(core)` | Check structure validity → `Vec<Issue>` |
| `attempt_correction(report, graph, executor)` | Try fixes → `ReflectionReport` |
| `provide_feedback(report, learning)` | Update learning from reflection |

**`ErrorType` enum:**

| Variant | Description |
|---------|-------------|
| `EmptyResult` | No results returned |
| `ResultOverflow` | Too many results (exceeds threshold) |
| `EntityNotFound(name)` | Referenced entity does not exist |
| `RelationMismatch(relation)` | Relationship type invalid for context |
| `CoreferenceFailure(text)` | Reference could not be resolved |
| `SchemaAlignment(msg)` | Query structure doesn't match graph schema |
| `Timeout` | Execution timed out |
| `Unknown(msg)` | Unclassified error |

**`SuggestedFix` enum:**

| Variant | Description |
|---------|-------------|
| `RetryWithQuery(query)` | Try a modified query |
| `ExpandScope { relation }` | Add relationships to broaden results |
| `NarrowScope { filter }` | Apply filter to reduce results |
| `ResolveEntity { original, suggested }` | Try alternative entity name |
| `AddRelation { from, to, relation }` | Add missing graph edge |
| `ManualIntervention(msg)` | Requires human action |

**`ReflectionConfig`:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_results` | `usize` | `100` | Result overflow threshold |
| `min_results` | `usize` | `1` | Empty result threshold |
| `max_retries` | `usize` | `2` | Maximum correction attempts |
| `auto_correct` | `bool` | `true` | Automatically apply simple fixes |

**`Severity` levels:** `Info`, `Warning`, `Error`, `Critical`.

### SealKnowledgeCoordinator (requires `knowledge` feature)

Bidirectional bridge between SEAL and the BKS/PKS knowledge system.

| Method | Description |
|--------|-------------|
| `get_pks_context(seal_result)` | Look up personal facts about resolved entities → `String` |
| `get_bks_context(query)` | Look up behavioral truths for query context → `String` |
| `harmonize_confidence(seal, bks, pks)` | Weighted confidence: SEAL(0.5) + BKS(0.3) + PKS(0.2) → `f32` |
| `adjust_retrieval_threshold(base, quality)` | Quality-aware threshold: `base * (0.7 + 0.3 * quality)` → `f32` |
| `check_and_promote_pattern(pattern, context)` | Promote reliable patterns to BKS |
| `observe_seal_resolutions(resolutions)` | Record entity resolutions in PKS (local-only) |
| `record_tool_failure(tool, error, context)` | Record failure pattern as BKS truth |

**`IntegrationConfig`:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `true` | Master toggle |
| `seal_to_knowledge` | `bool` | `true` | Enable pattern promotion to BKS |
| `knowledge_to_seal` | `bool` | `true` | Enable loading BKS patterns |
| `min_seal_quality_for_bks_boost` | `f32` | `0.7` | SEAL quality threshold for BKS context |
| `min_seal_quality_for_pks_boost` | `f32` | `0.5` | SEAL quality threshold for PKS context |
| `pattern_promotion_threshold` | `f32` | `0.8` | Minimum reliability to promote pattern |
| `min_pattern_uses` | `u32` | `5` | Minimum uses before promotion |
| `cache_bks_in_seal` | `bool` | `true` | Cache BKS results in SEAL memory |

**`EntityResolutionStrategy` enum:**

| Variant | Description |
|---------|-------------|
| `SealFirst` | Always prefer SEAL's resolution |
| `PksContextFirst` | Always prefer PKS-based resolution |
| `Hybrid { seal_weight, pks_weight }` | Weighted combination of both |

## Usage Examples

### Resolve coreferences in conversation

```rust
use brainwires_seal::{CoreferenceResolver, DialogState};
use brainwires_core::graph::{EntityStore, RelationshipGraph};

let resolver = CoreferenceResolver::new();
let mut dialog = DialogState::default();
dialog.current_turn = 5;
dialog.focus_stack.push("auth.rs".to_string());

let entity_store = EntityStore::new();
let graph = RelationshipGraph::new();

// Detect references in user message
let refs = resolver.detect_references("What functions does it export?");
println!("Found {} references", refs.len());

// Resolve against dialog context
let resolutions = resolver.resolve(&refs, &dialog, &entity_store, Some(&graph));
for r in &resolutions {
    println!("{} → {} (confidence: {:.2})", r.reference.text, r.antecedent, r.confidence);
}

// Rewrite query with resolutions
let resolved = resolver.rewrite_with_resolutions("What functions does it export?", &resolutions);
println!("Resolved: {}", resolved);
// → "What functions does [auth.rs] export?"
```

### Extract structured query cores

```rust
use brainwires_seal::{QueryCoreExtractor, QuestionType};
use brainwires_core::graph::EntityType;

let extractor = QueryCoreExtractor::new();

let entities = vec![
    ("main.rs".to_string(), EntityType::File),
    ("handle_request".to_string(), EntityType::Function),
];

if let Some(core) = extractor.extract("What depends on main.rs?", &entities) {
    println!("Type: {:?}", core.question_type);   // Dependency
    println!("S-expr: {}", core.to_sexp());        // (JOIN DependsOn ?dep "main.rs")
    println!("Confidence: {:.2}", core.confidence); // 0.95
}
```

### Learn from execution outcomes

```rust
use brainwires_seal::{LearningCoordinator, QueryCoreExtractor};
use brainwires_core::graph::EntityType;

let mut coordinator = LearningCoordinator::new("session-001".to_string());
let extractor = QueryCoreExtractor::new();

let entities = vec![("utils.rs".to_string(), EntityType::File)];
let core = extractor.extract("What uses utils.rs?", &entities);

// Process query — check for existing patterns
let matched = coordinator.process_query(
    "What uses utils.rs?",
    "What uses [utils.rs]?",
    core.clone(),
    1,
);

// Record successful outcome — builds reliability
coordinator.record_outcome(
    matched.as_ref().map(|p| p.id.as_str()),
    true,  // success
    3,     // result count
    core.as_ref(),
);

// After many successful uses, get patterns for prompt injection
let context = coordinator.get_context_for_prompt();
println!("Learned context:\n{}", context);
```

### Reflect on execution results

```rust
use brainwires_seal::{ReflectionModule, ReflectionConfig, Severity};
use brainwires_seal::{QueryCoreExtractor, QueryResult};
use brainwires_core::graph::{EntityType, RelationshipGraph};

let module = ReflectionModule::new(ReflectionConfig {
    max_results: 50,
    min_results: 1,
    max_retries: 2,
    auto_correct: true,
});

let extractor = QueryCoreExtractor::new();
let entities = vec![("config.rs".to_string(), EntityType::File)];
let core = extractor.extract("What uses config.rs?", &entities).unwrap();

let result = QueryResult {
    values: vec![],  // Empty — no results found
    count: 0,
    success: true,
    error: None,
};

let graph = RelationshipGraph::new();
let report = module.analyze(&core, &result, &graph);

println!("Quality: {:.2}", report.quality_score);
for issue in &report.issues {
    println!("[{:?}] {}", issue.severity, issue.message);
    for fix in &issue.suggested_fixes {
        println!("  Suggestion: {:?}", fix);
    }
}

if report.is_acceptable() {
    println!("Result acceptable");
} else {
    println!("Needs correction");
}
```

### Full pipeline with knowledge integration

```rust
use brainwires_seal::{SealProcessor, SealConfig, DialogState};
use brainwires_core::graph::{EntityStore, RelationshipGraph};

// Configure with all stages enabled
let config = SealConfig {
    enable_coreference: true,
    enable_query_cores: true,
    enable_learning: true,
    enable_reflection: true,
    max_reflection_retries: 3,
    min_coreference_confidence: 0.6,
    min_pattern_reliability: 0.8,
};

let mut processor = SealProcessor::new(config);
processor.init_conversation("session-042");

let entity_store = EntityStore::new();
let graph = RelationshipGraph::new();

let mut dialog = DialogState::default();
dialog.current_turn = 7;
dialog.focus_stack.push("provider.rs".to_string());

// Process query through full pipeline
let result = processor.process(
    "How many functions does it define?",
    &dialog,
    &entity_store,
    Some(&graph),
)?;

// Use results
println!("Original:  {}", result.original_query);
println!("Resolved:  {}", result.resolved_query);
println!("Quality:   {:.2}", result.quality_score);
println!("Issues:    {}", result.issues.len());

if let Some(pattern_id) = &result.matched_pattern {
    println!("Matched pattern: {}", pattern_id);
}

// After execution, record outcome for learning
processor.record_outcome(
    result.matched_pattern.as_deref(),
    true,
    12,
    result.query_core.as_ref(),
);

// Inject learned context into future prompts
let context = processor.get_learning_context();
```

## Integration

Use via the `brainwires` facade crate with the `seal` feature, or depend on `brainwires-seal` directly:

```toml
# Via facade
[dependencies]
brainwires = { version = "0.2", features = ["seal"] }

# Direct
[dependencies]
brainwires-seal = "0.2"
```

The crate re-exports all components at the top level:

```rust
use brainwires_seal::{
    // Orchestrator
    SealProcessor, SealConfig, SealProcessingResult,

    // Coreference resolution
    CoreferenceResolver, DialogState, ReferenceType,
    ResolvedReference, UnresolvedReference, SalienceScore,

    // Query core extraction
    QueryCoreExtractor, QueryCore, QueryExpr, QueryOp,
    QueryResult, QuestionType, RelationType,
    FilterPredicate, SuperlativeDir,

    // Self-evolving learning
    LearningCoordinator, LocalMemory, GlobalMemory,
    QueryPattern, TrackedEntity,

    // Reflection
    ReflectionModule, ReflectionConfig, ReflectionReport,
    Issue, ErrorType, Severity, SuggestedFix, CorrectionRecord,
};

// With `knowledge` feature
#[cfg(feature = "knowledge")]
use brainwires_seal::{
    SealKnowledgeCoordinator, IntegrationConfig, EntityResolutionStrategy,
};
```

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
