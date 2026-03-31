# SEAL Architecture

Self-Evolving Agentic Learning (SEAL) integration for brainwires-cli.

## Overview

The SEAL module enhances brainwires-cli with techniques from the SEAL paper to improve:
- **Coreference Resolution**: Understanding "it", "the file", "that function" in context
- **Semantic Query Cores**: Structured query extraction from natural language
- **Self-Evolving Learning**: Learning from successful interactions without retraining
- **Reflection & Correction**: Post-execution error detection and recovery

## Architecture

```
User Input
    │
    ▼
┌─────────────────────────────┐
│   SealProcessor             │
│                             │
│  ┌────────────────────────┐ │
│  │ CoreferenceResolver    │ │ ─► Resolves pronouns & definite NPs
│  └────────────┬───────────┘ │
│               │             │
│  ┌────────────▼───────────┐ │
│  │ QueryCoreExtractor     │ │ ─► Creates structured query
│  └────────────┬───────────┘ │
│               │             │
│  ┌────────────▼───────────┐ │
│  │ LearningCoordinator    │ │ ─► Checks for learned patterns
│  └────────────┬───────────┘ │
│               │             │
│  ┌────────────▼───────────┐ │
│  │ ReflectionModule       │ │ ─► Validates & corrects
│  └────────────────────────┘ │
└─────────────────────────────┘
```

## Module Structure

```
src/seal/
├── mod.rs            # Module root and SealProcessor
├── coreference.rs    # Coreference resolution
├── query_core.rs     # Semantic query extraction
├── learning.rs       # Self-evolving learning
└── reflection.rs     # Error detection & correction
```

## Components

### 1. Coreference Resolution

Resolves anaphoric references to concrete entities from conversation history.

**Reference Types:**
- `SingularNeutral`: "it", "this", "that"
- `Plural`: "they", "them", "those"
- `DefiniteNP`: "the file", "the function"
- `Demonstrative`: "that error", "this type"

**Resolution Algorithm:**

Uses salience-based ranking with weighted factors:
- **Recency** (0.35): More recently mentioned entities score higher
- **Frequency** (0.15): Entities mentioned multiple times score higher
- **Graph Centrality** (0.20): Important entities in RelationshipGraph score higher
- **Type Match** (0.20): Entity type compatibility with reference
- **Syntactic Prominence** (0.10): Subjects score higher than objects

**Usage:**

```rust
use brainwires::seal::{CoreferenceResolver, DialogState};

let resolver = CoreferenceResolver::new();
let mut state = DialogState::new();

// Track entity mentions
state.mention_entity("main.rs", EntityType::File);
state.next_turn();

// Detect and resolve references
let refs = resolver.detect_references("Fix it");
let resolved = resolver.resolve(&refs, &state, &entity_store, None);
// resolved[0].antecedent = "main.rs"

// Rewrite the message
let rewritten = resolver.rewrite_with_resolutions("Fix it", &resolved);
// rewritten = "Fix [main.rs]"
```

### 2. Semantic Query Cores

Extracts S-expression-like structured queries from natural language.

**Question Types:**
- `Definition`: "What is X?"
- `Location`: "Where is X defined?"
- `Dependency`: "What uses X?" / "What does X depend on?"
- `Count`: "How many X?"
- `Superlative`: "Which X has the most Y?"
- `Enumeration`: "List all X"
- `Boolean`: "Does X use Y?"

**Query Operations:**
- `Join(relation, subject, object)`: Traverse a relationship
- `And/Or`: Logical operations
- `Filter`: Apply predicates
- `Count`: Count results
- `Superlative`: Find max/min

**Usage:**

```rust
use brainwires::seal::QueryCoreExtractor;

let extractor = QueryCoreExtractor::new();
let entities = vec![("main.rs".to_string(), EntityType::File)];

let core = extractor.extract("What uses main.rs?", &entities);
// core.question_type = Dependency
// core.to_sexp() = "(JOIN DependsOn ?dependent \"main.rs\")"
```

### 3. Self-Evolving Learning

Enables learning from successful interactions without model retraining.

**Memory Types:**

**Local Memory (Per-Session):**
- Tracked entities with mention history
- Coreference resolution log
- Query execution history
- Focus stack for active context

**Global Memory (Cross-Session):**
- Query patterns with success/failure statistics
- Resolution patterns that worked well
- Template library by question type

**Learning Flow:**

```
Successful Query Execution
         │
         ▼
  Extract Pattern (generalize)
         │
         ▼
  Store with reliability score
         │
         ▼
  Future similar queries retrieve pattern
         │
         ▼
  Higher confidence = prioritize pattern
```

**Usage:**

```rust
use brainwires::seal::LearningCoordinator;

let mut coordinator = LearningCoordinator::new("session-123".to_string());

// Process a query
let pattern = coordinator.process_query(
    "What uses main.rs?",
    "What uses [main.rs]?",
    Some(query_core),
    turn_number,
);

// Record outcome
coordinator.record_outcome(pattern.map(|p| p.id.as_str()), true, 5, Some(&query_core));

// Get learned context for prompts
let context = coordinator.get_context_for_prompt();
```

### 4. Reflection Module

Post-execution analysis for error detection and correction.

**Error Types:**
- `EmptyResult`: Query returned no results
- `ResultOverflow`: Too many results to be useful
- `EntityNotFound`: Referenced entity doesn't exist
- `RelationMismatch`: Relationship type doesn't apply
- `CoreferenceFailure`: Could not resolve reference
- `SchemaAlignment`: Query structure doesn't match data

**Suggested Fixes:**
- `RetryWithQuery`: Retry with modified query
- `ExpandScope`: Add more relationships to search
- `NarrowScope`: Add filters to reduce results
- `ResolveEntity`: Suggest entity name correction
- `ManualIntervention`: Requires user action

**Usage:**

```rust
use brainwires::seal::{ReflectionModule, ReflectionConfig};

let mut reflection = ReflectionModule::new(ReflectionConfig::default());

// Analyze results
let report = reflection.analyze(&query_core, &result, &graph);

if !report.is_acceptable() {
    // Attempt automatic correction
    if reflection.attempt_correction(&mut report, &graph, &executor) {
        // Use corrected result
    }
}

// Provide feedback to learning system
reflection.provide_feedback(&report, &mut learning_coordinator);
```

## Integration with Orchestrator

The SEAL processor can be integrated into the agent orchestrator:

```rust
pub struct OrchestratorAgent {
    // ... existing fields ...
    seal: SealProcessor,
    dialog_state: DialogState,
    entity_store: EntityStore,
}

impl OrchestratorAgent {
    pub async fn process_message(&mut self, user_message: &str) -> Result<String> {
        // Initialize SEAL for this conversation
        self.seal.init_conversation(&self.conversation_id);

        // Process through SEAL pipeline
        let seal_result = self.seal.process(
            user_message,
            &self.dialog_state,
            &self.entity_store,
            Some(&self.relationship_graph),
        )?;

        // Use resolved query and matched patterns
        let context = format!(
            "{}\n\nLearning Context:\n{}",
            seal_result.resolved_query,
            self.seal.get_learning_context()
        );

        // Execute with enhanced context
        let response = self.execute(context).await?;

        // Record outcome for learning
        self.seal.record_outcome(
            seal_result.matched_pattern.as_deref(),
            true, // success
            1,    // result count
            seal_result.query_core.as_ref(),
        );

        Ok(response)
    }
}
```

## Configuration

```rust
pub struct SealConfig {
    /// Enable coreference resolution
    pub enable_coreference: bool,
    /// Enable query core extraction
    pub enable_query_cores: bool,
    /// Enable self-evolving learning
    pub enable_learning: bool,
    /// Enable reflection module
    pub enable_reflection: bool,
    /// Maximum retry attempts for reflection correction
    pub max_reflection_retries: u32,
    /// Minimum confidence score for coreference resolution
    pub min_coreference_confidence: f32,
    /// Minimum pattern reliability for learning
    pub min_pattern_reliability: f32,
}
```

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Coreference Accuracy | >90% | Correct resolutions in multi-turn conversations |
| Query Core Validity | >85% | Structurally valid query cores |
| Learning Effectiveness | Measurable | Performance improvement on repeated tasks |
| Reflection Recovery | >50% | Error recovery rate through reflection |
| Response Latency | <100ms | Time added by SEAL processing |

## Testing

All SEAL modules include comprehensive unit tests:

```bash
cargo test seal::
```

Test coverage includes:
- Pronoun detection and resolution
- Definite NP resolution
- Dialog state tracking
- Query classification and extraction
- Pattern learning and matching
- Reflection analysis and correction

## Future Enhancements

1. **Enhanced Entity Types**: Add more entity types (modules, packages, tests)
2. **Context Window Management**: Use SEAL insights to optimize context
3. **Multi-Agent Learning**: Share patterns across agent instances
4. **Temporal Reasoning**: Add time-based query support
5. **LanceDB Persistence**: Store learned patterns in LanceDB

## References

- SEAL Paper: Self-Evolving Agentic Learning for Knowledge-Based Conversational QA
- [Relationship Graph](../infinite-context/INFINITE_CONTEXT.md) - Knowledge graph implementation
- [Entity Extraction](../../src/utils/entity_extraction.rs) - Entity detection
