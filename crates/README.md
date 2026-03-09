# Brainwires Framework — Crate Dependency Tree

23 crates organized in layers. Arrows (`->`) indicate internal dependencies.

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
│    │   └─> rag (opt, "rag" feature)
│    │   └─> code-interpreters (opt, "interpreters" feature)
│    ├── brainwires-agents          Agent orchestration, lifecycle hooks, coordination patterns
│    │   └─> core
│    │   └─> tool-system
│    ├── brainwires-mdap            MAKER voting — microagent decomposition & reliability
│    │   └─> core
│    └── brainwires-permissions     Permission policies, audit logging, trust profiles
│        └─> core
│
├─── Storage & Knowledge
│    ├── brainwires-storage         LanceDB vector storage, tiered memory, embeddings
│    │   └─> core
│    │   └─> agents (opt, "agents" feature)
│    ├── brainwires-brain           Knowledge graphs — BKS, PKS, entity extraction, facts
│    │   └─> core
│    │   └─> storage
│    └── brainwires-prompting       Adaptive prompting, task clustering, temperature optimization
│        └─> core
│        └─> brain (opt, "knowledge" feature)
│
├─── RAG & Search
│    └── brainwires-rag             Codebase indexing, semantic search (LanceDB/Qdrant, tree-sitter)
│        └─> core (opt)
│
├─── Networking
│    ├── brainwires-mcp             MCP client, transport, protocol types
│    │   └─> core
│    ├── brainwires-relay           MCP server framework, relay client, encrypted transport
│    │   └─> core
│    │   └─> mcp
│    └── brainwires-mesh            Distributed agent mesh networking
│        └─> core
│        └─> a2a (opt, "a2a" feature)
│
├─── Learning & Training
│    ├── brainwires-datasets        Training data pipelines — JSONL, tokenization, dedup
│    │   └─> core
│    ├── brainwires-training        Fine-tuning — cloud (Anthropic/OpenAI) & local (LoRA/QLoRA)
│    │   └─> core
│    │   └─> datasets
│    │   └─> providers (opt, "cloud" feature)
│    └── brainwires-seal            SEAL — self-evolving agentic learning
│        └─> core
│        └─> tool-system
│        └─> agents
│        └─> mdap (opt, "mdap" feature)
│        └─> brain (opt, "knowledge" feature)
│
├─── Autonomy
│    └── brainwires-autonomy        Self-improvement, Git workflows, human-out-of-loop execution
│        └─> core
│        └─> agents (opt)
│        └─> tool-system (opt)
│        └─> training (opt)
│        └─> mdap (opt)
│        └─> rag (opt, "attention" feature)
│        └─> datasets (opt)
│
└─── WASM
     └── brainwires-wasm            Browser deployment bindings
         └─> core (wasm)
         └─> mdap (wasm)
         └─> tool-system (opt)
         └─> code-interpreters (opt)
```

## Extras (`extras/`)

Standalone apps built on the framework:

| App | Description |
|-----|-------------|
| `agent-chat` | Interactive multi-agent chat application |
| `brainwires-brain-server` | Knowledge graph server (BKS/PKS) |
| `brainwires-proxy` | Protocol-agnostic traffic debugging proxy |
| `brainwires-rag-server` | RAG semantic search MCP server |
| `reload-daemon` | Hot-reload daemon for development |

## Longest Dependency Chain

```
core -> storage -> brain -> prompting (knowledge feature)
```

## Feature Presets (facade crate)

| Preset | Includes |
|--------|----------|
| `agent-full` | agents, permissions, prompting, tools |
| `researcher` | providers, agents, storage, rag, training, datasets |
| `full` | everything |
