# Brainwires Framework — Deno/TypeScript Port

A modular, Deno-native TypeScript port of the [Brainwires Agent Framework](https://github.com/Brainwires/brainwires-framework). Build autonomous AI agents with tool use, multi-provider support, inter-agent communication, and fine-grained permissions — all running on Deno.

## Packages

| Package | JSR | Description |
|---------|-----|-------------|
| `@brainwires/core` | `deno add @brainwires/core` | Foundation types, messages, tools, errors, lifecycle hooks |
| `@brainwires/providers` | `deno add @brainwires/providers` | AI chat providers (Anthropic, OpenAI, Google, Ollama, etc.) |
| `@brainwires/agents` | `deno add @brainwires/agents` | Agent runtime, task agents, coordination patterns |
| `@brainwires/mcp` | `deno add @brainwires/mcp` | Model Context Protocol client |
| `@brainwires/a2a` | `deno add @brainwires/a2a` | Agent-to-Agent protocol (Google A2A) |
| `@brainwires/storage` | `deno add @brainwires/storage` | Backend-agnostic storage with domain stores |
| `@brainwires/permissions` | `deno add @brainwires/permissions` | Capability profiles, policy engine, audit, trust |
| `@brainwires/tool-system` | `deno add @brainwires/tool-system` | Tool registry, built-in tools (bash, files, git, web, search) |
| `@brainwires/cognition` | `deno add @brainwires/cognition` | Prompting techniques, knowledge graph, RAG interfaces |
| `@brainwires/agent-network` | `deno add @brainwires/agent-network` | MCP server framework, middleware, routing, discovery |
| `@brainwires/skills` | `deno add @brainwires/skills` | Skill parsing, registry, routing, and execution |

All packages are versioned at **0.5.0** and published to JSR under the `@brainwires` scope.

## Documentation & Examples

- **[Documentation](./docs/)** — Guides covering architecture, each subsystem, and extensibility
- **[Examples](./examples/)** — 43 runnable TypeScript examples ported from the Rust crates

## Package Dependency Diagram

```
                     @brainwires/core
                    /    |    |    \
                   /     |    |     \
          providers  storage  mcp  permissions
              |        |       |
              +--------+-------+
              |
            agents -----> tool-system
              |               |
         agent-network    cognition
              |
             a2a
```

`core` has zero external dependencies. Every other package depends on `core`. The `agents` package pulls in `providers`, `storage`, `mcp`, and `tool-system`. The `agent-network` and `a2a` packages are leaf-level consumers.

## Quick Start

### 1. Create a provider and send a message

```ts
import { Message, ChatOptions } from "@brainwires/core";
import { AnthropicChatProvider } from "@brainwires/providers";

const provider = new AnthropicChatProvider(
  Deno.env.get("ANTHROPIC_API_KEY")!,
  "claude-sonnet-4-20250514",
  "anthropic",
);

const messages = [Message.user("What is the Deno runtime?")];
const options = new ChatOptions({ max_tokens: 1024 });

const response = await provider.chat(messages, undefined, options);
console.log(response.content);
```

### 2. Register tools and run an agent

```ts
import { ChatOptions, Message } from "@brainwires/core";
import { AnthropicChatProvider } from "@brainwires/providers";
import { ToolRegistry, BashTool, FileOpsTool } from "@brainwires/tool-system";
import { TaskAgent, AgentContext, spawnTaskAgent } from "@brainwires/agents";

// Set up tools
const registry = new ToolRegistry();
registry.registerTools(BashTool.getTools());
registry.registerTools(FileOpsTool.getTools());

// Create provider
const provider = new AnthropicChatProvider(
  Deno.env.get("ANTHROPIC_API_KEY")!,
  "claude-sonnet-4-20250514",
  "anthropic",
);

// Build agent context and run
const context = new AgentContext({ tools: registry.allTools() });
const result = await spawnTaskAgent({
  agentId: "demo-agent",
  provider,
  context,
  systemPrompt: "You are a helpful coding assistant.",
  taskDescription: "List the files in the current directory.",
});

console.log(`Success: ${result.success}, Output: ${result.output}`);
```

### 3. Connect to an MCP server

```ts
import { McpClient } from "@brainwires/mcp";

const client = McpClient.createDefault();
await client.connect("my-server", {
  command: "npx",
  args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
});

const tools = await client.listTools("my-server");
console.log("Available tools:", tools.map((t) => t.name));
```

## What's Ported vs What's Not

| Rust Crate | Deno Package | Status |
|------------|-------------|--------|
| `brainwires-core` | `@brainwires/core` | Fully ported |
| `brainwires-providers` | `@brainwires/providers` | Ported (Anthropic, OpenAI, Google, Ollama). OpenAI Responses API and Brainwires Relay not yet implemented. |
| `brainwires-agents` | `@brainwires/agents` | Fully ported (runtime, task agent, coordination patterns) |
| `brainwires-mcp` | `@brainwires/mcp` | Fully ported (client, stdio transport, config) |
| `brainwires-a2a` | `@brainwires/a2a` | Fully ported (JSON-RPC + REST, no gRPC) |
| `brainwires-storage` | `@brainwires/storage` | Fully ported (in-memory backend, domain stores, tiered memory) |
| `brainwires-permissions` | `@brainwires/permissions` | Fully ported (capabilities, policies, audit, trust, anomaly) |
| `brainwires-tool-system` | `@brainwires/tool-system` | Fully ported (bash, file ops, git, web, search) |
| `brainwires-cognition` | `@brainwires/cognition` | Fully ported (prompting, knowledge, RAG interfaces) |
| `brainwires-agent-network` | `@brainwires/agent-network` | Fully ported (MCP server, middleware, routing, discovery) |
| `brainwires-audio` | -- | Not ported |
| `brainwires-autonomy` | -- | Not ported |
| `brainwires-code-interpreters` | -- | Not ported |
| `brainwires-datasets` | -- | Not ported |
| `brainwires-skills` | `@brainwires/skills` | Fully ported (parsing, registry, routing, execution) |
| `brainwires-training` | -- | Not ported |
| `brainwires-wasm` | -- | Not ported |

## Installation

Install any package with `deno add`:

```sh
deno add @brainwires/core
deno add @brainwires/providers
deno add @brainwires/agents
# ... etc.
```

Or import directly from JSR in your source:

```ts
import { Message, ChatOptions } from "jsr:@brainwires/core@0.5.0";
```

## Rust Crate Documentation

For full API documentation of the underlying Rust crates, see the [crates README](../crates/README.md) and the per-crate docs on [docs.rs](https://docs.rs).

## License

Same license as the parent Brainwires Framework repository.
