# Extensibility Guide

This guide covers extension points in the Brainwires framework for researchers and plugin authors.

## Extension Points

The framework is trait-based: implement a trait, pass it to the component, done.

### Core Traits (brainwires-core)

| Trait | Required Methods | Purpose |
|-------|-----------------|---------|
| `Provider` | `name`, `chat`, `stream_chat` (+`max_output_tokens` default) | AI chat completion backend |
| `EmbeddingProvider` | `embed`, `dimension`, `model_name` | Text embedding generation |
| `VectorStore` | `initialize`, `upsert`, `search`, `delete`, `clear`, `count` | Embedding storage/search |
| `OutputParser` | `parse`, `format_instructions` | Structured LLM output parsing |
| `LifecycleHook` | `name`, `on_event` (+`priority`, `filter` defaults) | Framework event interception |
| `StagingBackend` | `stage`, `commit`, `rollback`, `pending_count` | Two-phase file write commits |

### RAG Traits (brainwires-rag)

| Trait | Required Methods | Purpose |
|-------|-----------------|---------|
| `Chunker` | `chunk_file` | Custom file chunking strategy |
| `SearchScorer` | `fuse` | Hybrid search result fusion |
| `VectorDatabase` | 10 methods (initialize, store, search, etc.) | Full RAG vector DB |
| `RelationsProvider` | `extract_definitions`, `extract_references`, `supports_language`, `precision_level` | Code symbol extraction |

### Agent Traits (brainwires-agents)

| Trait | Required Methods | Purpose |
|-------|-----------------|---------|
| `AgentRuntime` | 11 methods (call_provider, execute_tool, etc.) | Custom agent execution loop |
| `LockPersistence` | `try_acquire`, `release`, `release_all_for_agent`, `cleanup_stale` | Cross-process lock backend |
| `CompensableOperation` | `execute`, `compensate`, `description` (+`operation_type` default) | Saga step with rollback |
| `EvaluationCase` | `name`, `category`, `run` | Eval scenario |

### Tool Traits (brainwires-tool-system)

| Trait | Required Methods | Purpose |
|-------|-----------------|---------|
| `ToolExecutor` | `execute`, `available_tools` | Custom tool execution backend |
| `ToolPreHook` | `before_execute` | Pre-execution tool gate |

### MDAP Traits (brainwires-agents, feature `mdap`)

| Trait | Required Methods | Purpose |
|-------|-----------------|---------|
| `TaskDecomposer` | `decompose`, `is_minimal`, `strategy` | Task decomposition strategy |
| `MicroagentProvider` | `chat` | LLM adapter for voting loop |
| `RedFlagValidator` | `validate` | Response quality check |
| `ResultComposer` | `compose` | Subtask output composition |

### Training Traits (brainwires-training)

| Trait | Required Methods | Purpose |
|-------|-----------------|---------|
| `FineTuneProvider` | 9 methods (create_job, get_status, etc.) | Cloud fine-tuning provider |
| `TrainingBackend` | `name`, `available_devices`, `train` | Local training execution |

### Other Extension Traits

| Trait | Crate | Purpose |
|-------|-------|---------|
| `TextToSpeech` | brainwires-audio | TTS synthesis backend |
| `SpeechToText` | brainwires-audio | STT transcription backend |
| `LanguageExecutor` | brainwires-code-interpreters | Sandboxed code execution |
| `Dataset` | brainwires-datasets | Training data container |
| `FormatConverter` | brainwires-datasets | Training data format conversion |
| `Tokenizer` | brainwires-datasets | Token encoding/counting |
| `ApprovalPolicy` | brainwires-autonomy | Autonomous operation approval |
| `GitForge` | brainwires-autonomy | Git forge API (GitHub, GitLab) |

---

## Quick Recipes

### "I want to add a custom AI provider"

Implement `Provider` from `brainwires::core`:

```rust
use brainwires::prelude::*;
use async_trait::async_trait;
use futures::stream::BoxStream;

struct MyProvider;

#[async_trait]
impl Provider for MyProvider {
    fn name(&self) -> &str { "my-provider" }

    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
        options: &ChatOptions,
    ) -> anyhow::Result<ChatResponse> {
        let last = messages.iter().rev()
            .find(|m| m.role == Role::User)
            .and_then(|m| m.text())
            .unwrap_or_default();

        Ok(ChatResponse {
            message: Message::assistant(format!("Response to: {}", last)),
            usage: Usage::new(10, 20),
            finish_reason: Some("stop".to_string()),
        })
    }

    fn stream_chat<'a>(
        &'a self,
        messages: &'a [Message],
        tools: Option<&'a [Tool]>,
        options: &'a ChatOptions,
    ) -> BoxStream<'a, anyhow::Result<StreamChunk>> {
        Box::pin(async_stream::stream! {
            let resp = self.chat(messages, tools, options).await?;
            yield Ok(StreamChunk::Text(resp.message.text().unwrap_or_default().to_string()));
            yield Ok(StreamChunk::Done);
        })
    }
}
```

See `crates/brainwires/examples/custom_provider.rs` for a complete runnable example.

### "I want custom embeddings"

Implement `EmbeddingProvider` from `brainwires::core`:

```rust
use brainwires::prelude::*;

struct MyEmbedding { dim: usize }

impl EmbeddingProvider for MyEmbedding {
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        // Your embedding model here
        Ok(vec![0.0; self.dim])
    }
    fn dimension(&self) -> usize { self.dim }
    fn model_name(&self) -> &str { "my-embedding-v1" }
    // embed_batch has a default impl; override for native batching
}
```

See `crates/brainwires/examples/custom_embedding.rs` for a complete example.

### "I want custom RAG chunking"

Implement `Chunker` from `brainwires::rag::indexer`:

```rust
use brainwires::rag::indexer::{Chunker, CodeChunk, FileInfo, ChunkStrategy, CodeChunker};
use std::sync::Arc;

struct SemanticChunker;

impl Chunker for SemanticChunker {
    fn chunk_file(&self, file_info: &FileInfo) -> Vec<CodeChunk> {
        // Your chunking logic (sentence boundaries, ML segmentation, etc.)
        vec![]
    }
}

// Plug into the pipeline:
let strategy = ChunkStrategy::Custom(Arc::new(SemanticChunker));
let chunker = CodeChunker::new(strategy);
```

### "I want custom search scoring"

Implement `SearchScorer` from `brainwires::rag::bm25_search`:

```rust
use brainwires::rag::bm25_search::{SearchScorer, BM25Result};
use std::sync::Arc;

struct CrossEncoderReranker;

impl SearchScorer for CrossEncoderReranker {
    fn fuse(
        &self,
        vector_results: Vec<(u64, f32)>,
        bm25_results: Vec<BM25Result>,
        limit: usize,
    ) -> Vec<(u64, f32)> {
        // Your fusion/reranking logic
        vector_results.into_iter().take(limit).collect()
    }
}

// Plug into LanceVectorDB:
// let db = LanceVectorDB::with_path("/path").await?
//     .with_scorer(Arc::new(CrossEncoderReranker));
```

See `crates/brainwires/examples/rag_custom_pipeline.rs` for a complete example.

### "I want a custom agent loop"

Implement `AgentRuntime` from `brainwires::agents`:

```rust
use brainwires::agents::{AgentRuntime, AgentExecutionResult, run_agent_loop};
use brainwires::agents::{CommunicationHub, FileLockManager, LockType};

// AgentRuntime requires 11 methods:
//   agent_id, max_iterations, call_provider, extract_tool_uses,
//   is_completion, execute_tool, get_lock_requirement,
//   on_provider_response, on_tool_result, on_completion, on_iteration_limit
//
// Then run it:
// let result = run_agent_loop(my_runtime, &hub, &lock_manager).await?;
```

See `crates/brainwires/examples/agent_quickstart.rs` for infrastructure setup.

---

## Feature Flags

The facade crate (`brainwires`) gates each subsystem behind a feature flag.

### Researcher bundle

```toml
[dependencies]
brainwires = { version = "0.6", features = ["researcher"] }
```

This enables: `providers`, `agents`, `storage`, `rag`, `training`, `datasets`.

### Individual features

| Feature | Enables | Transitive Dependencies |
|---------|---------|------------------------|
| `tools` | `brainwires-tool-system` | — |
| `agents` | `brainwires-agents` | brainwires-tool-system |
| `storage` | `brainwires-storage` (with native) | lancedb, arrow, fastembed |
| `mcp` | `brainwires-mcp` | rmcp |
| `mdap` | `brainwires-agents/mdap` | — |
| `prompting` | `brainwires-prompting` | linfa-clustering, ndarray |
| `permissions` | `brainwires-permissions` | — |
| `rag` | `brainwires-rag` (with native, lancedb) | lancedb, tantivy, tree-sitter |
| `providers` | `brainwires-providers` | reqwest |
| `seal` | `brainwires-seal` | — |
| `relay` | `brainwires-relay` | — |
| `skills` | `brainwires-skills` | — |
| `audio` | `brainwires-audio` | — |
| `datasets` | `brainwires-datasets` | — |
| `training` | `brainwires-training` | — |
| `autonomy` | `brainwires-autonomy` | — |
| `brain` | `brainwires-brain` | — |

### Compound features

| Feature | Composition |
|---------|-------------|
| `researcher` | providers + agents + storage + rag + training + datasets |
| `agent-full` | agents + permissions + prompting + tools |
| `learning` | seal + knowledge + seal/knowledge |
| `full` | Everything |
| `rag-full-languages` | rag + tree-sitter language grammars |
| `training-full` | training/full + datasets/full |

### Default features

`default = ["tools", "agents"]` — minimal agent toolkit without heavy native deps.

---

## Architecture for Plugin Authors

### Crate dependency graph (simplified)

```
brainwires (facade)
  ├── brainwires-core (always)       ← core traits, types, errors
  ├── brainwires-tool-system         ← ToolExecutor, built-in tools
  ├── brainwires-agents              ← AgentRuntime, CommunicationHub
  ├── brainwires-providers           ← Anthropic, OpenAI, Google, Ollama
  ├── brainwires-rag                 ← Chunker, SearchScorer, VectorDatabase
  ├── brainwires-storage             ← TieredMemory, LanceDB stores
  ├── brainwires-training            ← Fine-tuning backends
  └── brainwires-datasets            ← Dataset containers, format converters
```

### Where to define new traits

- **Pure types/traits with no heavy deps** → `brainwires-core`
- **Tool implementations** → `brainwires-tool-system`
- **Agent coordination** → `brainwires-agents`
- **RAG pipeline components** → `brainwires-rag`

### Error handling

Use `FrameworkError` from `brainwires::core` for domain-specific errors:

```rust
use brainwires::prelude::*;

// Domain-specific constructors:
FrameworkError::provider_auth("my-provider", "Invalid API key")
FrameworkError::provider_model("my-provider", "gpt-5", "Model not found")
FrameworkError::embedding_dimension(384, 768)
FrameworkError::storage_schema("my-store", "Missing 'embeddings' table")
FrameworkError::training_config("learning_rate", "Must be between 0 and 1")

// Generic fallback (wrap any error):
FrameworkError::Provider("Something went wrong".to_string())
```

### Testing your extension

```bash
# Build with just the features you need
cargo build -p brainwires --features providers

# Run examples
cargo run -p brainwires --example custom_provider --features providers
cargo run -p brainwires --example custom_embedding
cargo run -p brainwires --example agent_quickstart --features agents
cargo run -p brainwires --example rag_custom_pipeline --features rag
```
