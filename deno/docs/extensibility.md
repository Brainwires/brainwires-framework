# Extensibility

This guide covers how to extend the Brainwires framework by implementing key interfaces. The framework is interface-driven: implement an interface, pass it to the component, done.

## Key Interfaces

| Interface | Package | Purpose |
|-----------|---------|---------|
| `Provider` | `@brainwires/core` | AI chat completion backend |
| `EmbeddingProvider` | `@brainwires/core` | Text embedding generation |
| `VectorStore` | `@brainwires/core` | Embedding storage and search |
| `StorageBackend` | `@brainwires/storage` | Record persistence backend |
| `VectorDatabase` | `@brainwires/storage` | Storage + vector search |
| `ToolExecutor` | `@brainwires/tools` | Custom tool execution backend |
| `ToolPreHook` | `@brainwires/tools` | Pre-execution tool gate |
| `AgentRuntime` | `@brainwires/agents` | Custom agent execution loop |
| `LifecycleHook` | `@brainwires/core` | Framework event interception |
| `OutputParser` | `@brainwires/core` | Structured LLM output parsing |
| `BrainClient` | `@brainwires/knowledge` | Knowledge storage interface |
| `RagClient` | `@brainwires/knowledge` | Semantic code search interface |
| `Middleware` | `@brainwires/network` | MCP server request processing |
| `Discovery` | `@brainwires/network` | Peer discovery protocol |
| `A2aHandler` | `@brainwires/a2a` | A2A agent server handler |

## Custom Provider

Implement `Provider` from `@brainwires/core`:

```ts
import type { Provider, ChatResponse, StreamChunk, Tool } from "@brainwires/core";
import { Message, ChatOptions, createUsage } from "@brainwires/core";

class MyProvider implements Provider {
  name(): string { return "my-provider"; }

  async chat(
    messages: Message[],
    tools?: Tool[],
    options?: ChatOptions,
  ): Promise<ChatResponse> {
    const last = messages.findLast((m) => m.role === "user");
    const text = last?.text() ?? "";
    return {
      message: Message.assistant(`Response to: ${text}`),
      usage: createUsage(10, 20),
      finish_reason: "stop",
    };
  }

  async *streamChat(
    messages: Message[],
    tools?: Tool[],
    options?: ChatOptions,
  ): AsyncIterable<StreamChunk> {
    const resp = await this.chat(messages, tools, options);
    yield { type: "text", text: resp.message.text() ?? "" };
    yield { type: "done" };
  }
}
```

Use it anywhere a `Provider` is expected -- `spawnTaskAgent`, `runAgentLoop`, etc.

## Custom Storage Backend

Implement `StorageBackend` from `@brainwires/storage`:

```ts
import type { StorageBackend, Record, Filter, FieldDef } from "@brainwires/storage";

class RedisBackend implements StorageBackend {
  async createTable(name: string, fields: FieldDef[]): Promise<void> { /* ... */ }
  async insert(table: string, record: Record): Promise<void> { /* ... */ }
  async get(table: string, id: string): Promise<Record | null> { /* ... */ }
  async update(table: string, id: string, record: Record): Promise<void> { /* ... */ }
  async delete(table: string, id: string): Promise<void> { /* ... */ }
  async query(table: string, filter: Filter): Promise<Record[]> { /* ... */ }
  async list(table: string, limit?: number, offset?: number): Promise<Record[]> { /* ... */ }
}
```

Pass it to any domain store: `new MessageStore(new RedisBackend())`.

## Custom Tools

Implement `ToolExecutor` from `@brainwires/tools`:

```ts
import type { ToolExecutor } from "@brainwires/tools";
import { ToolResult, type Tool, type ToolUse, objectSchema } from "@brainwires/core";

const databaseTool: Tool = {
  name: "query_db",
  description: "Run a SQL query",
  input_schema: objectSchema({ sql: { type: "string" } }, ["sql"]),
};

class DatabaseExecutor implements ToolExecutor {
  availableTools(): Tool[] { return [databaseTool]; }

  async execute(toolUse: ToolUse): Promise<ToolResult> {
    const result = await runQuery(toolUse.input.sql);
    return ToolResult.success(toolUse.id, JSON.stringify(result));
  }
}
```

## Custom Agent Runtime

Implement `AgentRuntime` for full control over the agent loop:

```ts
import type { AgentRuntime, AgentExecutionResult } from "@brainwires/agents";
import { runAgentLoop } from "@brainwires/agents";

class MyRuntime implements AgentRuntime {
  agentId(): string { return "custom-agent"; }
  maxIterations(): number { return 20; }
  async callProvider(): Promise<Message> { /* ... */ }
  extractToolUses(msg: Message): ToolUse[] { /* ... */ }
  isCompletion(msg: Message): boolean { /* ... */ }
  async executeTool(toolUse: ToolUse): Promise<ToolResult> { /* ... */ }
  // ... remaining lifecycle methods
}

const result = await runAgentLoop(new MyRuntime(), hub, lockManager);
```

## Custom Middleware

Implement `Middleware` for the MCP server pipeline:

```ts
import {
  type Middleware,
  type MiddlewareResult,
  middlewareContinue,
  middlewareReject,
  RequestContext,
} from "@brainwires/network";

class MetricsMiddleware implements Middleware {
  async process(ctx: RequestContext): Promise<MiddlewareResult> {
    const start = performance.now();
    // Middleware runs before the handler; return continue to proceed
    console.log(`Request: ${ctx.method} (${performance.now() - start}ms)`);
    return middlewareContinue();
  }
}
```

## Custom Lifecycle Hooks

Intercept framework events with `LifecycleHook`:

```ts
import type { LifecycleHook, LifecycleEvent, HookResult } from "@brainwires/core";

const loggingHook: LifecycleHook = {
  name: () => "logging",
  priority: () => 10,
  filter: () => ({ eventTypes: ["tool_start", "tool_end"] }),
  onEvent: async (event: LifecycleEvent): Promise<HookResult> => {
    console.log(`[${event.type}] ${event.agentId}: ${event.toolName}`);
    return { proceed: true };
  },
};
```

## Error Handling

Use `FrameworkError` for domain-specific errors:

```ts
import { FrameworkError } from "@brainwires/core";

throw FrameworkError.providerAuth("my-provider", "Invalid API key");
throw FrameworkError.storageSchema("my-store", "Missing table");
```

## Where to Define Extensions

- **Types and interfaces** -- `@brainwires/core`
- **Tool implementations** -- `@brainwires/tools`
- **Agent coordination** -- `@brainwires/agents`
- **Storage backends** -- `@brainwires/storage`
- **Network components** -- `@brainwires/network`

## Further Reading

- [Architecture](./architecture.md) for the package dependency graph
- [Providers](./providers.md) for the built-in provider implementations
- [Tools](./tools.md) for built-in tool examples
- [Storage](./storage.md) for built-in storage backends
