# @brainwires/tools

Built-in tool implementations and a composable tool registry for the Brainwires Agent Framework. Provides ready-to-use tools for shell commands, file operations, git, web fetching, and code search.

Equivalent to the Rust `brainwires-tools` crate.

## Install

```sh
deno add @brainwires/tools
```

## Quick Example

```ts
import {
  ToolRegistry,
  BashTool,
  FileOpsTool,
  GitTool,
  SearchTool,
  WebTool,
} from "@brainwires/tools";

// Create a registry and register tools
const registry = new ToolRegistry();
registry.registerTools(BashTool.getTools());
registry.registerTools(FileOpsTool.getTools());
registry.registerTools(GitTool.getTools());
registry.registerTools(SearchTool.getTools());
registry.registerTools(WebTool.getTools());

// List all registered tools
for (const tool of registry.allTools()) {
  console.log(`${tool.name}: ${tool.description}`);
}

// Look up a tool by name
const bashTool = registry.get("bash");
console.log(bashTool?.name); // "bash"
```

## Built-in Tools

| Tool Class | Tools Provided | Description |
|------------|---------------|-------------|
| `BashTool` | `bash` | Shell command execution with output management |
| `FileOpsTool` | `read_file`, `write_file`, `edit_file`, `list_directory`, `search_files`, `delete_file`, `create_directory` | File system operations |
| `GitTool` | `git_status`, `git_diff`, `git_log`, `git_stage`, `git_commit`, `git_push`, `git_pull`, etc. | Git operations |
| `SearchTool` | `search_code` | Regex-based code search (respects `.gitignore`) |
| `WebTool` | `web_fetch` | URL fetching |

## Other Exports

| Export | Description |
|--------|-------------|
| `ToolRegistry` | Composable container with category filtering and search |
| `getSmartTools` | Context-aware tool selection based on message analysis |
| `sanitizeExternalContent` | Input sanitization for tool outputs |
| `classifyError` / `retryStrategy` | Error taxonomy and retry logic |
