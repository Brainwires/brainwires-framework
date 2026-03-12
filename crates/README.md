# Brainwires Framework — Crate Dependency Tree

Crates organized in layers. Arrows (`->`) indicate internal dependencies. For standalone apps built on the framework, see [`extras/`](../extras/README.md).

```
brainwires  (facade — re-exports all crates via feature flags)
│
├─── Foundation (no internal deps)
│    ├── brainwires-core            Core types, traits, messages, tools, tasks
│    ├── brainwires-a2a             Agent-to-Agent protocol (JSON-RPC, REST, gRPC)
│    ├── brainwires-code-interpreters  Sandboxed execution (Rhai, Lua, JS, Python)
│    └── brainwires-skills          Skill system — SKILL.md parsing, registry, routing
│
├─── Providers
│    ├── brainwires-providers       AI providers (Anthropic, OpenAI, Google, Ollama, Bedrock, Vertex AI)
│    │   └─> core
│    └── brainwires-audio           Speech-to-text & text-to-speech
│        └─> providers (opt)
│
├─── Tools & Agents
│    ├── brainwires-tool-system     Built-in tools (file ops, git, bash, web, search, validation)
│    │   └─> core
│    │   └─> cognition (opt, "rag" feature)
│    │   └─> code-interpreters (opt, "interpreters" feature)
│    ├── brainwires-agents          Agent orchestration, lifecycle hooks, coordination patterns, SEAL
│    │   └─> core
│    │   └─> tool-system
│    │   └─> cognition (opt, "seal-knowledge" feature)
│    │   └─> mdap (opt, "seal-mdap" feature)
│    │   └─> permissions (opt, "seal-feedback" feature)
│    ├── brainwires-mdap            MAKER voting — microagent decomposition & reliability
│    │   └─> core
│    └── brainwires-permissions     Permission policies, audit logging, trust profiles
│        └─> core
│
├─── Storage & Intelligence
│    ├── brainwires-storage         LanceDB vector storage, tiered memory, embeddings
│    │   └─> core
│    └── brainwires-cognition       Unified intelligence — knowledge graphs, adaptive prompting, RAG
│        └─> core
│        └─> storage (opt, "knowledge" and "rag" features)
│
├─── Networking
│    ├── brainwires-mcp             MCP client, transport, protocol types
│    │   └─> core
│    └── brainwires-agent-network   MCP server framework, encrypted IPC, remote bridge, mesh networking
│        └─> core
│        └─> mcp
│        └─> a2a (opt, "mesh" feature)
│
├─── Learning & Training
│    ├── brainwires-datasets        Training data pipelines — JSONL, tokenization, dedup
│    │   └─> core
│    └── brainwires-training        Fine-tuning — cloud (Anthropic/OpenAI) & local (LoRA/QLoRA)
│        └─> core
│        └─> datasets
│        └─> providers (opt, "cloud" feature)
│
├─── Autonomy
│    └── brainwires-autonomy        Self-improvement, Git workflows, human-out-of-loop execution
│        └─> core
│        └─> agents (opt)
│        └─> tool-system (opt)
│        └─> training (opt)
│        └─> mdap (opt)
│        └─> cognition (opt, "attention" feature)
│        └─> datasets (opt)
│
└─── WASM
     └── brainwires-wasm            Browser deployment bindings
         └─> core (wasm)
         └─> mdap (wasm)
         └─> tool-system (opt)
         └─> code-interpreters (opt)
```

## Crate Merges (v0.3)

| Old Crate | Merged Into | Notes |
|-----------|-------------|-------|
| `brainwires-brain` | `brainwires-cognition` | Knowledge graphs, PKS/BKS, entity extraction → `knowledge` feature |
| `brainwires-prompting` | `brainwires-cognition` | Adaptive prompting, clustering → `prompting` feature |
| `brainwires-rag` | `brainwires-cognition` | Codebase indexing, semantic search → `rag` feature |
| `brainwires-relay` | `brainwires-agent-network` | MCP server framework, IPC, remote bridge → `server` feature |
| `brainwires-mesh` | `brainwires-agent-network` | Mesh networking, topology, routing → `mesh` feature |
| `brainwires-seal` | `brainwires-agents/seal/` | Self-evolving agentic learning → `seal` feature |

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
