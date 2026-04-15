# Brainwires Framework — Crate Dependency Tree

Crates organized in layers. Arrows (`->`) indicate internal dependencies. For standalone apps built on the framework, see [`extras/`](../extras/README.md).

```
brainwires  (facade — re-exports all crates via feature flags)
│
├─── Foundation (no internal deps)
│    ├── brainwires-core            Core types, traits, messages, tools, tasks
│    ├── brainwires-a2a             Agent-to-Agent protocol (JSON-RPC, REST, gRPC)
│    └── brainwires-a2a             Agent-to-Agent protocol (JSON-RPC, REST, gRPC)
│
├─── Providers
│    ├── brainwires-providers       AI providers (Anthropic, OpenAI, Google, Ollama, Bedrock, Vertex AI)
│    │   └─> core
│    └── brainwires-hardware         Audio, GPIO, Bluetooth, network, camera, USB hardware I/O
│        └─> providers (opt, "audio" feature)
│
├─── Tools & Agents
│    ├── brainwires-tools     Built-in tools (file ops, git, bash, web, search, validation)
│    │   └─> core
│    │   └─> knowledge (opt, "rag" feature)
│    │   └─> tools (interpreters feature) (opt, "interpreters" feature)
│    ├── brainwires-agents          Agent orchestration, lifecycle hooks, coordination patterns, SEAL
│    │   └─> core
│    │   └─> tools
│    │   └─> knowledge (opt, "seal-knowledge" feature)
│    │   └─> permissions (opt, "seal-feedback" feature)
│    └── brainwires-permissions     Permission policies, audit logging, trust profiles
│        └─> core
│
├─── Storage & Intelligence
│    ├── brainwires-storage         Unified database layer (9 backends), tiered memory, embeddings
│    │   └─> core
│    └── brainwires-knowledge       Unified intelligence — knowledge graphs, adaptive prompting, RAG, dream consolidation
│        └─> core
│        └─> storage (opt, "knowledge" and "rag" features)
│
├─── Networking
│    ├── brainwires-mcp             MCP client, transport, protocol types
│    │   └─> core
│    └── brainwires-network   MCP server, IPC, remote bridge, 5-layer protocol stack, mesh networking
│        └─> core
│        └─> mcp
│        └─> a2a (opt, "a2a-transport" feature)
│
├─── Learning & Training
│    ├── brainwires-datasets        Training data pipelines — JSONL, tokenization, dedup
│    │   └─> core
│    └── brainwires-training        Fine-tuning — cloud (Anthropic/OpenAI) & local (LoRA/QLoRA)
│        └─> core
│        └─> datasets
│        └─> providers (opt, "cloud" feature)
│
├─── System
│    └── brainwires-system          Generic OS-level primitives — FS reactor, service management
│        (no internal deps)
│
├─── Autonomy
│    └── brainwires-autonomy        Self-improvement, Git workflows, human-out-of-loop execution
│        └─> core
│        └─> agents (opt)
│        └─> tools (opt)
│        └─> training (opt)
│        └─> mdap (opt)
│        └─> knowledge (opt, "attention" feature)
│        └─> datasets (opt)
│        └─> hardware (opt, "gpio" feature — re-exports GPIO)
│
└─── WASM
     └── brainwires-wasm            Browser deployment bindings
         └─> core (wasm)
         └─> mdap (wasm)
         └─> tools (opt)
         └─> tools (interpreters feature) (opt)
```

## Longest Dependency Chain

```
core -> storage -> cognition (knowledge feature)
```

## Feature Presets (facade crate)

| Preset | Includes |
|--------|----------|
| `agent-full` | agents, permissions, cognition, tools |
| `researcher` | providers, agents, storage, cognition, training, datasets |
| `full` | everything |
