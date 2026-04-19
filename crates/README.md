# Brainwires Framework — Crate Dependency Tree

Crates organized in layers. Arrows (`->`) indicate internal dependencies. For standalone apps built on the framework, see [`extras/`](../extras/README.md).

```
brainwires  (facade — re-exports all crates via feature flags)
│
├─── Foundation (no internal deps)
│    └── brainwires-core               Core types, traits, messages, tools, tasks, embeddings
│
├─── Infrastructure
│    ├── brainwires-telemetry          OutcomeMetrics, Prometheus export, billing hooks
│    │   └─> core
│    ├── brainwires-storage            Unified database layer (9 backends), tiered memory, embeddings
│    │   └─> core
│    ├── brainwires-providers          AI providers (Anthropic, OpenAI, Google, Ollama, Bedrock, Vertex AI)
│    │   └─> core
│    │   └─> telemetry (opt, "telemetry" feature)
│    └── brainwires-hardware           Audio, GPIO, Bluetooth, camera, USB, Matter, homeauto I/O
│        └─> providers (opt, "audio" feature)
│
├─── Protocols
│    ├── brainwires-mcp                MCP client (rmcp-backed)
│    │   └─> core
│    ├── brainwires-mcp-server         MCP server framework with middleware; optional HTTP+SSE, OAuth
│    │   └─> core
│    └── brainwires-a2a                Agent-to-Agent protocol (JSON-RPC, REST, gRPC)
│        └─> core
│
├─── Intelligence
│    └── brainwires-knowledge          Knowledge (BKS/PKS), prompting, RAG (indexing + hybrid search)
│        └─> core
│        └─> storage (opt, "knowledge" / "rag" features)
│
├─── Action
│    ├── brainwires-tools              File ops, git, bash, web, search, validation, interpreters
│    │   └─> core
│    │   └─> knowledge (opt, "rag" feature)
│    └── brainwires-permissions        Permission policies, audit logging, trust profiles
│        └─> core
│
├─── Reasoning
│    └── brainwires-reasoning          Planners, validators, routers, strategies, scorers, output parsers
│        └─> core
│        └─> tools (dep on ToolCategory in router.rs)
│
├─── Agency
│    ├── brainwires-agents             Agent runtime, communication hub, task decomposition, MDAP, SEAL, skills, eval
│    │   └─> core
│    │   └─> tools
│    │   └─> knowledge (opt, "seal-knowledge" feature)
│    │   └─> permissions (opt, "seal-feedback" feature)
│    └── brainwires-network            IPC, TCP, remote bridge, 5-layer protocol stack, mesh
│        └─> core
│        └─> mcp
│        └─> a2a (opt, "a2a-transport" feature)
│
└─── Training
     └── brainwires-training           Fine-tuning — cloud (6 providers) & local LoRA/QLoRA/DoRA (Burn)
         └─> core
         └─> providers (opt, "cloud" feature)
```

## Longest Dependency Chain

With the `rag` features active (which pull in the optional `storage` and `knowledge` edges of `tools`), the longest leaf-to-leaf chain is 4 hops:

```
core -> storage -> knowledge -> tools -> reasoning
core -> storage -> knowledge -> tools -> agents
```

`reasoning` and `agents` both depend on `tools` directly; there is no edge between them. Without the optional `rag` features the chain collapses to `core -> tools -> reasoning` / `core -> tools -> agents`.

## Feature Presets (facade crate)

See [`crates/brainwires/README.md`](brainwires/README.md) for the full feature table. Convenience presets:

| Preset | Includes |
|--------|----------|
| `agent-full` | agents, permissions, prompting, tools |
| `researcher` | providers, agents, storage, rag, training, datasets |
| `learning` | seal, knowledge, permissions, seal-knowledge, seal-feedback |
| `full` | everything |
