# Getting Started

Get from zero to a running agent in about 5 minutes.

## Prerequisites

- [Deno](https://deno.land/) 1.40+
- An API key for at least one AI provider (Anthropic, OpenAI, Google, or a local Ollama instance)

## 1. Install packages

```sh
deno add @brainwires/core
deno add @brainwires/providers
deno add @brainwires/agents
deno add @brainwires/tool-system
```

## 2. Create a provider

Every interaction starts with a `Provider` -- the bridge between your code and an AI model.

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

## 3. Register tools

Tools give agents the ability to interact with the world. The `ToolRegistry` holds them.

```ts
import { ToolRegistry, BashTool, FileOpsTool } from "@brainwires/tool-system";

const registry = new ToolRegistry();
registry.registerTools(BashTool.getTools());
registry.registerTools(FileOpsTool.getTools());
```

## 4. Run an agent

Combine a provider, tools, and a task description to spawn an autonomous agent.

```ts
import { AgentContext, spawnTaskAgent } from "@brainwires/agents";

const context = new AgentContext({ tools: registry.allTools() });

const result = await spawnTaskAgent({
  agentId: "demo-agent",
  provider,
  context,
  systemPrompt: "You are a helpful coding assistant.",
  taskDescription: "List the files in the current directory.",
});

console.log(`Success: ${result.success}`);
console.log(`Output: ${result.output}`);
```

## 5. Connect to an MCP server (optional)

Use external tool servers via the Model Context Protocol.

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

## Next steps

- See the full [quickstart example](../examples/core/quickstart.ts)
- Learn the [architecture](./architecture.md)
- Explore [providers](./providers.md), [tools](./tools.md), and [agents](./agents.md)
