# Infinite Context: A Hybrid RAG Approach for Long Conversations

## The Problem

Large Language Models have finite context windows. Even with 100k+ token contexts, long coding sessions can exceed these limits. Traditional approaches include:

1. **Sliding Window**: Drop oldest messages as new ones arrive
   - Problem: Loses important early decisions and context

2. **Summarization**: Periodically summarize and replace old messages
   - Problem: Summaries lose detail; can't recall specific code snippets

3. **Hierarchical Memory**: Multiple levels of compression
   - Problem: Complex to implement; still loses fidelity

None of these approaches allow **perfect recall** of early conversation details when needed.

## Our Solution: Compaction + RAG Recall

Brainwires CLI implements a hybrid approach that combines compaction with Retrieval-Augmented Generation (RAG):

```
┌─────────────────────────────────────────────────────────────┐
│                    Full Conversation History                 │
│              (Stored in LanceDB with embeddings)            │
│  ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐   │
│  │ M1  │ M2  │ M3  │ M4  │ M5  │ M6  │ M7  │ M8  │ M9  │   │
│  └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘   │
│         ▲                                                    │
│         │ Semantic Search (384-dim embeddings)               │
│         │                                                    │
└─────────┼───────────────────────────────────────────────────┘
          │
┌─────────┴───────────────────────────────────────────────────┐
│                    Active Context (Sent to API)              │
│  ┌──────────────────┐  ┌─────┬─────┬─────┬─────┐           │
│  │ [Summary: M1-M5] │  │ M6  │ M7  │ M8  │ M9  │           │
│  │  Key decisions   │  │     │     │     │     │           │
│  │  and context...  │  │     │     │     │     │           │
│  └──────────────────┘  └─────┴─────┴─────┴─────┘           │
│                                                              │
│     Agent can use recall_context tool to search M1-M5       │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Key Insight

**Every message is stored with semantic embeddings before compaction.** When the active context is compacted, the original messages remain searchable in the vector database. The agent can recall specific details on demand.

## How It Works

### 1. Message Storage

Every message in a conversation is:
- Stored in LanceDB (a local vector database)
- Embedded using all-MiniLM-L6-v2 (384-dimensional vectors)
- Indexed for fast semantic search

```rust
// Messages automatically get embeddings when saved
message_store.add_batch(messages).await?;
```

### 2. Compaction Trigger

When the conversation approaches the token limit (default: 80k of 100k):

```rust
if conversation_manager.needs_compaction() {
    // Save all messages to DB first (with embeddings)
    conversation_manager.save_to_db().await?;

    // Generate summarization prompt
    let (prompt, count) = conversation_manager
        .generate_compaction_prompt(Some("Focus on code changes"))?;

    // Get LLM to summarize
    let summary = llm.generate(prompt).await?;

    // Apply compaction (keeps recent 20%, replaces rest with summary)
    conversation_manager.apply_compaction(&summary, count);
}
```

### 3. Context Recall

The agent has access to a `recall_context` tool:

```json
{
  "name": "recall_context",
  "description": "Search through full conversation history for specific details...",
  "input_schema": {
    "properties": {
      "query": "What to search for",
      "limit": "Max results (default: 5)",
      "min_score": "Minimum relevance 0-1 (default: 0.6)",
      "cross_conversation": "Search all conversations (default: false)"
    }
  }
}
```

When the agent needs to recall earlier details:

```
Agent: I need to check what authentication approach we discussed earlier.

[Uses recall_context tool with query: "authentication approach"]

Tool Result:
1. [Score: 0.89] [user]
   We should use JWT tokens for authentication...

2. [Score: 0.82] [assistant]
   Based on our discussion, I'll implement JWT with refresh tokens...
```

## Implementation Details

### Storage Layer

```
LanceDB (Local Vector Database)
├── conversations table
│   ├── conversation_id
│   ├── title
│   ├── model_id
│   ├── created_at, updated_at
│   └── message_count
│
└── messages table
    ├── message_id
    ├── conversation_id
    ├── role (user/assistant/system/tool)
    ├── content
    ├── vector (384-dim float32)
    ├── token_count
    └── created_at
```

### Search Implementation

```rust
// Semantic search with conversation scope
pub async fn search_conversation(
    &self,
    conversation_id: &str,
    query: &str,
    limit: usize,
    min_score: f32,
) -> Result<Vec<(MessageMetadata, f32)>> {
    let query_embedding = self.embeddings.embed(query)?;

    let results = table
        .vector_search(query_embedding)
        .only_if(format!("conversation_id = '{}'", conversation_id))
        .limit(limit)
        .execute()
        .await?;

    // Convert distance to similarity score
    // Score = 1 / (1 + distance)
    ...
}
```

### Token Management

```
Total Context Budget: 100,000 tokens
├── System Prompt: ~2,000 tokens
├── Compaction Summary: ~2,000 tokens
├── Recalled Context: up to 5,000 tokens (per recall)
├── Recent Messages: ~80,000 tokens
└── Buffer: ~11,000 tokens

Compaction Threshold: 80,000 tokens
Messages Retained: ~20% most recent (minimum 4)
```

## Advantages Over Traditional Approaches

| Approach | Detail Preservation | Recall Ability | Storage | Latency |
|----------|---------------------|----------------|---------|---------|
| Sliding Window | None | None | Low | Low |
| Summarization | Partial | None | Low | Low |
| Full History | Complete | Complete | High | High |
| **Our Hybrid** | **Complete** | **On-demand** | **Medium** | **Low*** |

*Latency is low because:
- Full context isn't sent every request
- RAG lookups are local (no network)
- Vector search is ~20-30ms

## Usage Examples

### Basic Compaction

```bash
# In chat, run /compact to trigger summarization
> /compact

Conversation Compaction
  Messages: 47
  Estimated tokens: ~85,000
  Threshold: 80,000 tokens

  Messages to summarize: 38
  Messages to keep: 9

Summarization request sent...
```

### Agent Using Recall

```
User: What was the database schema we designed?

Agent: [thinking] The schema discussion was earlier in the conversation
       and may have been compacted. Let me recall it.

[recall_context query="database schema design"]

Agent: Based on our earlier discussion, we designed a schema with:
- users table with id, email, created_at
- sessions table with user_id foreign key
- tokens table for JWT refresh tokens
[Shows exact details from recalled messages]
```

### Cross-Conversation Search

```rust
// Search across all past conversations
let results = message_store.search(
    "authentication implementation",
    10,    // limit
    0.7,   // min_score
).await?;
```

## Configuration

### Constants (src/config/constants.rs)

```rust
/// Maximum context tokens (model-dependent)
pub const MAX_CONTEXT_TOKENS: usize = 100_000;

/// Trigger compaction at this threshold
pub const COMPACTION_THRESHOLD_TOKENS: usize = 80_000;
```

### Tool Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `limit` | 5 | Maximum results to return |
| `min_score` | 0.6 | Minimum similarity score (0-1) |
| `cross_conversation` | false | Search all conversations |

## Privacy & Security

- **Local Storage**: All conversation data stored locally in LanceDB
- **No Cloud Sync**: Messages never leave your machine
- **Conversation Isolation**: Default search scope is current conversation only
- **Opt-in Cross-Search**: Must explicitly enable cross-conversation search

## Performance Characteristics

- **Embedding Model**: all-MiniLM-L6-v2 (384 dimensions)
- **Index Type**: IVF (Inverted File Index) with PQ compression
- **Search Latency**: 20-30ms for typical queries
- **Storage**: ~1KB per message (content + embedding)
- **Batch Embedding**: Up to 32 messages per batch

## Performance Optimizations (Phase 1)

### Embedding Cache

Query embeddings are cached using an LRU (Least Recently Used) cache with 1000 entries. This significantly reduces latency for repeated queries, which are common in agent loops.

```rust
// Cached embedding lookup - returns cached result or generates new
let embedding = provider.embed_cached(query)?;

// Cache statistics
println!("Cache size: {}", provider.cache_len());
provider.clear_cache();  // Clear if needed
```

**Impact**: 50-80% reduction in embedding latency for repeated queries.

### Retrieval Gating

Before performing expensive RAG lookups, a cheap classifier determines if retrieval is needed:

```rust
use brainwires_cli::utils::retrieval_gate::{needs_retrieval, classify_retrieval_need};

// Simple boolean check
if needs_retrieval(user_message, recent_context_len, has_compaction) {
    // Perform RAG lookup
}

// Detailed classification with confidence
let (need, confidence) = classify_retrieval_need(message, context_len, compacted);
match need {
    RetrievalNeed::High => { /* definitely retrieve */ },
    RetrievalNeed::Medium => { /* probably retrieve */ },
    RetrievalNeed::Low => { /* maybe retrieve */ },
    RetrievalNeed::None => { /* skip retrieval */ },
}
```

The gate checks for:
- Explicit back-references ("earlier", "we discussed", "remember when")
- Question patterns about past events ("what did", "when did")
- Short context (likely heavily compacted)
- Pronoun references in short follow-up questions

**Impact**: 30-40% reduction in unnecessary retrieval calls.

## Phase 2: Multi-Resolution Memory (Implemented)

### Tiered Storage

Messages are organized into three tiers based on importance and recency:

```
┌─────────────────────────────────────────────────┐
│  HOT TIER (Full Messages)                       │
│  - Recent messages                              │
│  - Important content (code, decisions)          │
│  - Recently accessed                            │
├─────────────────────────────────────────────────┤
│  WARM TIER (Summaries)                          │
│  - Compressed older messages                    │
│  - Key entities preserved                       │
│  - Promotable on access                         │
├─────────────────────────────────────────────────┤
│  COLD TIER (Key Facts)                          │
│  - Ultra-compressed archival                    │
│  - Decisions, definitions, requirements         │
│  - Minimal storage footprint                    │
└─────────────────────────────────────────────────┘
```

Messages flow down tiers based on:
- **Age**: Older messages get demoted
- **Importance**: High-importance stays in hot tier longer
- **Access patterns**: Frequently accessed content stays hot

### Importance Scoring

Each message receives an importance score (0.0-1.0) based on:

```rust
use brainwires_cli::utils::importance::{calculate_importance, ImportanceContext};

let context = ImportanceContext {
    forward_references: 2,  // Referenced by 2 later messages
    age_seconds: 3600.0,    // 1 hour old
    ..Default::default()
};

let result = calculate_importance(message_content, &context);
println!("Score: {}", result.score);
println!("Code: {}", result.code_score);
println!("Decision: {}", result.decision_score);
```

Scoring factors:
- **Named entities**: Files, functions, variables mentioned
- **Code blocks**: Code content scores higher
- **Decision language**: "We decided", "The solution", etc.
- **Forward references**: Messages mentioned later
- **Recency**: Exponential decay over time

### Adaptive Resolution Retrieval

Search queries automatically check tiers in order:
1. Search hot tier (full messages) - if score > 0.85, return immediately
2. Search warm tier (summaries) - promote high matches to hot
3. Search cold tier (facts) - for archival queries

```rust
let results = tiered_memory.search_adaptive(query, Some(conversation_id)).await?;

for result in results {
    println!("Tier: {:?}, Score: {}", result.tier, result.score);
}
```

## Phase 3: Auto-Inject and Prompt Caching (Implemented)

### Context Builder

Automatically enhances conversation context with relevant historical information:

```rust
use brainwires_cli::utils::context_builder::{ContextBuilder, ContextBuilderConfig};

let config = ContextBuilderConfig {
    injection_threshold: 0.75,  // Only inject if score > 75%
    max_inject_items: 3,        // Max 3 retrieved items
    use_gating: true,           // Use retrieval gating
    ..Default::default()
};

let builder = ContextBuilder::with_config(config);

// Check if retrieval is needed
if builder.should_retrieve(user_query, &messages) {
    let enhanced = builder
        .build_context(&messages, query, &message_store, conversation_id)
        .await?;
}
```

Features:
- **Retrieval gating**: Skips lookup when not needed
- **Threshold filtering**: Only injects high-relevance content
- **Smart positioning**: Inserts after compaction summary
- **Token budgeting**: Respects max injection token limits

### Prompt Caching

Utilities for Anthropic API prompt caching:

```rust
use brainwires_cli::utils::prompt_cache::{
    build_cached_system_prompt,
    CacheAnalyzer,
    CacheConfig,
};

// Build cached system prompt
let system = build_cached_system_prompt("You are a helpful assistant");

// Analyze messages for cache opportunities
let analyzer = CacheAnalyzer::new(CacheConfig::default());
let analysis = analyzer.analyze(&messages);

if analysis.should_cache() {
    println!("Estimated savings: {}ms", analysis.estimated_savings_ms());
}
```

Cache points identified:
- System prompts (100ms savings)
- Compaction summaries (150ms savings)
- Injected context (50ms savings)

## Integration with ConversationManager

The `ConversationManager` now integrates all these optimizations automatically:

```rust
use brainwires_cli::utils::conversation::ConversationManager;
use brainwires_cli::utils::context_builder::ContextBuilderConfig;

// Default configuration
let mut manager = ConversationManager::new(100_000);

// Or with custom context builder config
let config = ContextBuilderConfig {
    injection_threshold: 0.8,  // Higher threshold = fewer injections
    max_inject_items: 5,       // More items injected
    ..Default::default()
};
let mut manager = ConversationManager::with_context_config(100_000, config);

// Add messages as usual
manager.add_message(user_message);

// Get enhanced context - automatically injects relevant history if needed
let enhanced_messages = manager.get_enhanced_context(&user_query).await?;

// Check if conversation has been compacted
if manager.has_compaction() {
    // Context builder will consider retrieval
}
```

### Automatic Context Enhancement

The `get_enhanced_context()` method:

1. **Checks retrieval gating** - Only performs RAG lookup when patterns suggest it's needed
2. **Searches conversation history** - Uses semantic search on stored messages
3. **Filters by threshold** - Only injects high-relevance content (default >75%)
4. **Positions correctly** - Inserts after compaction summary, before recent messages

### When Enhancement Occurs

Context is automatically enhanced when:
- Conversation has been compacted (has summary message)
- User query contains back-references ("earlier", "we discussed", etc.)
- User asks questions about past events
- Context is short (likely heavily compacted)

No enhancement occurs when:
- No compaction has happened yet
- Query is a simple continuation
- No relevant history found above threshold

## Phase 4: Entity Extraction & Relationship Graph (Implemented)

### Entity Extraction

Automatically extracts named entities from conversation messages:

```rust
use brainwires_cli::utils::entity_extraction::{EntityExtractor, EntityStore, EntityType};

let extractor = EntityExtractor::new();
let mut store = EntityStore::new();

// Extract entities from a message
let result = extractor.extract(message_content, message_id);
store.add_extraction(result, message_id, timestamp);

// Query entities
let files = store.get_by_type(&EntityType::File);
let top_entities = store.get_top_entities(10);
let related = store.get_related("src/main.rs");
```

Entity types extracted:
- **File**: Source files (`.rs`, `.js`, `.py`, etc.)
- **Function**: Function definitions (`fn`, `function`, `def`)
- **Type**: Type definitions (`struct`, `class`, `interface`, `enum`)
- **Variable**: Variable declarations (longer names only)
- **Concept**: Programming concepts (api, authentication, database, etc.)
- **Error**: Error types and messages
- **Command**: CLI commands (cargo, npm, git, etc.)

### Relationship Graph

Stores and queries relationships between entities:

```rust
use brainwires_cli::storage::{RelationshipGraph, EdgeType};

let mut graph = RelationshipGraph::new();

// Build graph from entity store
let graph = RelationshipGraph::from_entity_store(&entity_store);

// Query relationships
let neighbors = graph.get_neighbors("src/main.rs");
let path = graph.find_path("src/main.rs", "Config");
let context = graph.get_entity_context("main", 2); // depth=2

// Search by name
let results = graph.search("authentication", 5);
```

Relationship types:
- **Contains**: File contains function/type
- **References**: Entity references another
- **DependsOn**: Dependency relationship
- **Modifies**: Modification relationship
- **Defines**: Definition relationship
- **CoOccurs**: Entities mentioned together

### Use Cases

1. **Context-aware retrieval**: Find all messages mentioning a file and its functions
2. **Impact analysis**: Trace relationships when modifying code
3. **Knowledge graph**: Build understanding of codebase structure over time

## Future Enhancements

1. **Hybrid Search**: Combine vector similarity with BM25 keyword matching
2. **Smart Summarization**: Generate summaries optimized for later retrieval
3. **Conversation Clustering**: Group related conversations for faster cross-search

## Comparison to Other Approaches

### vs. Claude Code's /compact

Claude Code's `/compact` command summarizes and discards old context. Our approach extends this by:
- Preserving original messages in a searchable database
- Providing agents explicit tools to recall details
- Supporting cross-conversation memory

### vs. LangChain Memory

LangChain provides various memory implementations (buffer, summary, entity). Our approach differs by:
- Using semantic search for recall (not just entity extraction)
- Operating at the conversation level (not document chunks)
- Providing agent-accessible tools (not automatic injection)

### vs. MemGPT

MemGPT creates an explicit memory hierarchy managed by the agent. Our approach is simpler:
- Single storage layer with semantic search
- No explicit memory management required
- Agent can recall on-demand without complex protocols

## Conclusion

The hybrid compaction + RAG approach gives us the best of both worlds:
- **Efficient Context**: Only relevant, recent messages sent to the API
- **Perfect Recall**: Any detail from any point in the conversation is searchable
- **Low Latency**: Local vector search, no network roundtrips for recall
- **Simple Mental Model**: Compact + search, no complex memory hierarchies

This enables truly long-running coding sessions where the agent can always recall "what we discussed earlier" without hitting context limits.
