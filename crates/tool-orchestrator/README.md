# Tool Orchestrator - Universal Programmatic Tool Calling

[![Tests](https://img.shields.io/badge/tests-64%20passing-brightgreen)](https://github.com/anthropics/tool-orchestrator)
[![Coverage](https://img.shields.io/badge/coverage-92.59%25-brightgreen)](https://github.com/anthropics/tool-orchestrator)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange)](https://www.rust-lang.org/)

A model-agnostic implementation of Anthropic's [Programmatic Tool Calling](https://www.anthropic.com/engineering/advanced-tool-use) pattern. Instead of sequential tool calls consuming tokens, any LLM writes Rhai scripts that orchestrate multiple tools efficiently.

## Background: The Problem with Traditional Tool Calling

Traditional AI tool calling follows a request-response pattern:

```
LLM: "Call get_expenses(employee_id=1)"
→ Returns 100 expense items to context
LLM: "Call get_expenses(employee_id=2)"
→ Returns 100 more items to context
... (20 employees later)
→ 2,000+ line items polluting the context window
→ 110,000+ tokens just to produce a summary
```

Each intermediate result floods the model's context window, wasting tokens and degrading performance.

## Anthropic's Solution: Programmatic Tool Calling

In November 2024, Anthropic introduced [Programmatic Tool Calling (PTC)](https://www.anthropic.com/engineering/advanced-tool-use) as part of their advanced tool use features. The key insight:

> **LLMs excel at writing code.** Instead of reasoning through one tool call at a time, let them write code that orchestrates entire workflows.

Their approach:
1. Claude writes Python code that calls multiple tools
2. Code executes in Anthropic's managed sandbox
3. Only the final result returns to the context window

**Results:** 37-98% token reduction, lower latency, more reliable control flow.

### References

- [Introducing advanced tool use on the Claude Developer Platform](https://www.anthropic.com/engineering/advanced-tool-use) - Anthropic Engineering Blog
- [CodeAct: Executable Code Actions Elicit Better LLM Agents](https://arxiv.org/abs/2402.01030) - Academic research on code-based tool orchestration

## Why This Crate? Universal Access

Anthropic's implementation has constraints:
- **Claude-only**: Requires Claude 4.5 with the `advanced-tool-use-2025-11-20` beta header
- **Python-only**: Scripts must be Python
- **Anthropic-hosted**: Execution happens in their managed sandbox
- **API-dependent**: Requires their code execution tool to be enabled

**Tool Orchestrator** provides the same benefits for **any LLM provider**:

| Constraint | Anthropic's PTC | Tool Orchestrator |
|------------|-----------------|-------------------|
| **Model** | Claude 4.5 only | Any LLM that can write code |
| **Language** | Python | Rhai (Rust-like, easy for LLMs) |
| **Execution** | Anthropic's sandbox | Your local process |
| **Runtime** | Server-side (their servers) | Client-side (your control) |
| **Dependencies** | API call + beta header | Pure Rust, zero runtime deps |
| **Targets** | Python environments | Native Rust + WASM (browser/Node.js) |

### Supported LLM Providers

- Claude (all versions, not just 4.5)
- OpenAI (GPT-4, GPT-4o, o1, etc.)
- Google (Gemini Pro, etc.)
- Anthropic competitors (Mistral, Cohere, etc.)
- Local models (Ollama, llama.cpp, vLLM)
- Any future provider

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                     TRADITIONAL APPROACH                        │
│                                                                 │
│  LLM ─→ Tool Call ─→ Full Result to Context ─→ LLM reasons     │
│  LLM ─→ Tool Call ─→ Full Result to Context ─→ LLM reasons     │
│  LLM ─→ Tool Call ─→ Full Result to Context ─→ LLM reasons     │
│                     (tokens multiply rapidly)                   │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                 PROGRAMMATIC TOOL CALLING                       │
│                                                                 │
│  LLM writes script:                                             │
│  ┌──────────────────────────────────────┐                      │
│  │ let results = [];                     │                      │
│  │ for id in employee_ids {              │   Executes locally   │
│  │   let expenses = get_expenses(id);    │ ─────────────────→   │
│  │   let flagged = expenses.filter(...); │   Tools called       │
│  │   results.push(flagged);              │   in sandbox         │
│  │ }                                     │                      │
│  │ summarize(results)  // Only this      │ ←─────────────────   │
│  └──────────────────────────────────────┘   returns to LLM     │
└─────────────────────────────────────────────────────────────────┘
```

1. **Register tools** - Your actual tool implementations (file I/O, APIs, etc.)
2. **LLM writes script** - Any LLM generates a Rhai script orchestrating those tools
3. **Sandboxed execution** - Script runs locally with configurable safety limits
4. **Minimal context** - Only the final result enters the conversation

## Multi-Target Architecture

This crate produces **two outputs** from a single codebase:

| Target | Description | Use Case |
|--------|-------------|----------|
| **Rust Library** | Native Rust crate with `Arc<Mutex>` thread safety | CLI tools, server-side apps, native integrations |
| **WASM Package** | Browser/Node.js module with `Rc<RefCell>` | Web apps, npm packages, browser-based AI |

## Benefits

- **37-98% token reduction** - Intermediate results stay in sandbox, only final output returns
- **Batch operations** - Process thousands of items in loops without context pollution
- **Conditional logic** - if/else based on tool results, handled in code not LLM reasoning
- **Data transformation** - Filter, aggregate, transform between tool calls
- **Explicit control flow** - Loops, error handling, retries are code, not implicit reasoning
- **Model agnostic** - Works with any LLM that can write Rhai/Rust-like code
- **Audit trail** - Every tool call is recorded with timing and results

## Installation & Building

### Rust Library (default)

```bash
# Add to Cargo.toml
cargo add tool-orchestrator

# Or build from source
cargo build
```

### WASM Package

```bash
# Build for web (browser)
wasm-pack build --target web --features wasm --no-default-features

# Build for Node.js
wasm-pack build --target nodejs --features wasm --no-default-features

# The package is generated in ./pkg/
```

## Usage

### Rust Library

```rust
use tool_orchestrator::{ToolOrchestrator, ExecutionLimits};

// Create orchestrator
let mut orchestrator = ToolOrchestrator::new();

// Register tools as executor functions
orchestrator.register_executor("read_file", |input| {
    let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
    std::fs::read_to_string(path).map_err(|e| e.to_string())
});

orchestrator.register_executor("list_directory", |input| {
    let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
    let entries: Vec<String> = std::fs::read_dir(path)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path().display().to_string()))
        .collect();
    Ok(entries.join("\n"))
});

// Execute a Rhai script (written by any LLM)
let script = r#"
    let files = list_directory("src");
    let rust_files = [];
    for file in files.split("\n") {
        if file.ends_with(".rs") {
            rust_files.push(file);
        }
    }
    `Found ${rust_files.len()} Rust files: ${rust_files}`
"#;

let result = orchestrator.execute(script, ExecutionLimits::default())?;
println!("Output: {}", result.output);           // Final result only
println!("Tool calls: {:?}", result.tool_calls); // Audit trail
```

### WASM (JavaScript/TypeScript)

```typescript
import init, { WasmOrchestrator, ExecutionLimits } from 'tool-orchestrator';

await init();

const orchestrator = new WasmOrchestrator();

// Register a JavaScript function as a tool
orchestrator.register_tool('get_weather', (inputJson: string) => {
  const input = JSON.parse(inputJson);
  // Your implementation here
  return JSON.stringify({ temp: 72, condition: 'sunny' });
});

// Execute a Rhai script
const limits = new ExecutionLimits();
const result = orchestrator.execute(`
  let weather = get_weather("San Francisco");
  \`Current weather: \${weather}\`
`, limits);

console.log(result);
// { success: true, output: "Current weather: ...", tool_calls: [...] }
```

## Safety & Sandboxing

The orchestrator includes built-in limits to prevent runaway scripts:

| Limit | Default | Description |
|-------|---------|-------------|
| `max_operations` | 100,000 | Prevents infinite loops |
| `max_tool_calls` | 50 | Limits tool invocations |
| `timeout_ms` | 30,000 | Execution timeout |
| `max_string_size` | 10MB | Maximum string length |
| `max_array_size` | 10,000 | Maximum array elements |

```rust
// Preset profiles
let quick = ExecutionLimits::quick();      // 10k ops, 10 calls, 5s
let extended = ExecutionLimits::extended(); // 500k ops, 100 calls, 2m

// Custom limits
let limits = ExecutionLimits::default()
    .with_max_operations(50_000)
    .with_max_tool_calls(25)
    .with_timeout_ms(10_000);
```

## Security Considerations

### What the Sandbox Prevents

Rhai scripts executed by this crate **cannot**:
- Access the filesystem (no `std::fs`, no file I/O)
- Make network requests (no sockets, no HTTP)
- Execute shell commands (no `std::process`)
- Access environment variables
- Spawn threads or processes
- Access raw memory or use unsafe code

The only way scripts interact with the outside world is through **explicitly registered tools**.

### Tool Implementer Responsibility

**You are responsible for the security of tools you register.** If you register a tool that:
- Executes shell commands → scripts can run arbitrary commands
- Reads/writes files → scripts can access your filesystem
- Makes HTTP requests → scripts can exfiltrate data

Design your tools with the principle of least privilege.

### Timeout Behavior

The `timeout_ms` limit uses Rhai's `on_progress` callback for **real-time enforcement**:
- Timeout is checked after every Rhai operation (not just at the end)
- CPU-intensive loops will be terminated mid-execution when timeout is exceeded
- **Note:** Timeout checks don't occur *during* a tool call - if a registered tool blocks for 10 seconds, that time isn't interruptible
- For tools that may block, implement your own timeouts within the tool executor

### Recommended Limits for Untrusted Input

```rust
let limits = ExecutionLimits::default()
    .with_max_operations(10_000)      // Tight loop protection
    .with_max_tool_calls(5)           // Limit external interactions
    .with_timeout_ms(5_000)           // 5 second max
    .with_max_string_size(100_000)    // 100KB strings
    .with_max_array_size(1_000);      // 1K elements
```

## Why Rhai Instead of Python?

Anthropic uses Python because Claude is trained extensively on it. We chose [Rhai](https://rhai.rs/) for different reasons:

| Factor | Python | Rhai |
|--------|--------|------|
| **Safety** | Requires heavy sandboxing | Sandboxed by design, no filesystem/network access |
| **Embedding** | CPython runtime (large) | Pure Rust, compiles into your binary |
| **WASM** | Complex (Pyodide, etc.) | Native WASM support |
| **Syntax** | Python-specific | Rust-like (familiar to many LLMs) |
| **Performance** | Interpreter overhead | Optimized for embedding |
| **Dependencies** | Python ecosystem | Zero runtime dependencies |

LLMs have no trouble generating Rhai - it's syntactically similar to Rust/JavaScript:

```rhai
// Variables
let x = 42;
let name = "Claude";

// String interpolation (backticks)
let greeting = `Hello, ${name}!`;

// Arrays and loops
let items = [1, 2, 3, 4, 5];
let sum = 0;
for item in items {
    sum += item;
}

// Conditionals
if sum > 10 {
    "Large sum"
} else {
    "Small sum"
}

// Maps (objects)
let config = #{
    debug: true,
    limit: 100
};

// Tool calls (registered functions)
let content = read_file("README.md");
let files = list_directory("src");
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Thread-safe with `Arc<Mutex>` (for native Rust) |
| `wasm` | No | Single-threaded with `Rc<RefCell>` (for browser/Node.js) |

## Testing

### Native Tests

```bash
# Run all native tests
cargo test

# Run with verbose output
cargo test -- --nocapture
```

### WASM Tests

WASM tests require `wasm-pack`. Install it with:

```bash
cargo install wasm-pack
```

Run WASM tests:

```bash
# Test with Node.js (fastest)
wasm-pack test --node --features wasm --no-default-features

# Test with headless Chrome
wasm-pack test --headless --chrome --features wasm --no-default-features

# Test with headless Firefox
wasm-pack test --headless --firefox --features wasm --no-default-features
```

### Test Coverage

The test suite includes:

**Native tests (39 tests)**
- Orchestrator creation and configuration
- Tool registration and execution
- Script compilation and execution
- Error handling (compilation errors, tool errors, runtime errors)
- Execution limits (max operations, max tool calls, timeout)
- JSON type conversion
- Loop and conditional execution
- Timing and metrics recording

**WASM tests (25 tests)**
- ExecutionLimits constructors and setters
- WasmOrchestrator creation
- Script execution (simple, loops, conditionals, functions)
- Tool registration and execution
- JavaScript callback integration
- Error handling (compilation, runtime, tool errors)
- Max operations and tool call limits
- Complex data structures (arrays, maps, nested)

## Integration Example

The orchestrator integrates with AI agents via a tool definition:

```rust
// Register as "execute_script" tool for the LLM
Tool {
    name: "execute_script",
    description: "Execute a Rhai script for programmatic tool orchestration.
                  Write code that calls registered tools, processes results,
                  and returns only the final output. Use loops for batch
                  operations, conditionals for branching logic.",
    input_schema: /* script parameter */,
    requires_approval: false,  // Scripts are sandboxed
}
```

When the LLM needs to perform multi-step operations, it writes a Rhai script instead of making sequential individual tool calls. The script executes locally, and only the final result enters the context window.

## Instructing LLMs to Generate Rhai

To use programmatic tool calling, your LLM needs to know how to write Rhai scripts. Include something like this in your system prompt:

### System Prompt Template

```
You have access to a script execution tool that runs Rhai code. When you need to:
- Call multiple tools in sequence
- Process data from tool results
- Loop over items or aggregate results
- Apply conditional logic based on tool outputs

Write a Rhai script instead of making individual tool calls.

## Rhai Syntax Quick Reference

Variables and types:
  let x = 42;                    // integer
  let name = "hello";            // string
  let items = [1, 2, 3];         // array
  let config = #{ key: "value" }; // map (object)

String interpolation (use backticks):
  let msg = `Hello, ${name}!`;
  let result = `Found ${items.len()} items`;

Loops:
  for item in items { /* body */ }
  for i in 0..10 { /* 0 to 9 */ }

Conditionals:
  if x > 5 { "big" } else { "small" }

String methods:
  s.len(), s.contains("x"), s.starts_with("x"), s.ends_with("x")
  s.split(","), s.trim(), s.to_upper(), s.to_lower()
  s.sub_string(start, len), s.index_of("x")

Array methods:
  arr.push(item), arr.len(), arr.pop()
  arr.filter(|x| x > 5), arr.map(|x| x * 2)

Parsing:
  "42".parse_int(), "3.14".parse_float()

Available tools (call as functions):
  {TOOL_LIST}

## Important Rules

1. The LAST expression in your script is the return value
2. Use string interpolation with backticks for output: `Result: ${value}`
3. Process data locally - don't return intermediate results
4. Only return the final summary/answer

## Example

Task: Get total expenses for employees 1-3

Script:
let total = 0;
for id in [1, 2, 3] {
    let expenses = get_expenses(id);  // Returns JSON array
    // Parse and sum (simplified)
    total += expenses.len() * 100;    // Estimate
}
`Total across 3 employees: $${total}`
```

### Rhai Syntax Cheatsheet for LLMs

| Concept | Rhai Syntax | Notes |
|---------|-------------|-------|
| **Variables** | `let x = 5;` | Immutable by default |
| **Mutable** | `let x = 5; x = 10;` | Can reassign |
| **Strings** | `"hello"` or `` `hello` `` | Backticks allow interpolation |
| **Interpolation** | `` `Value: ${x}` `` | Only in backtick strings |
| **Arrays** | `[1, 2, 3]` | Dynamic, mixed types OK |
| **Maps** | `#{ a: 1, b: 2 }` | Like JSON objects |
| **For loops** | `for x in arr { }` | Iterates over arrays |
| **Ranges** | `for i in 0..5 { }` | 0, 1, 2, 3, 4 |
| **If/else** | `if x > 5 { a } else { b }` | Expression-based |
| **Functions** | `fn add(a, b) { a + b }` | Last expr is return |
| **Tool calls** | `tool_name(arg)` | Registered tools are functions |
| **Comments** | `// comment` | Single line |
| **Unit (null)** | `()` | Like None/null |

## Related Projects

- **[open-ptc-agent](https://github.com/Chen-zexi/open-ptc-agent)** - Python implementation using Daytona sandbox
- **[LangChain DeepAgents](https://github.com/langchain-ai/deepagents)** - LangChain's agent framework with code execution

## Acknowledgements

This project implements patterns from:

- [Anthropic's Advanced Tool Use](https://www.anthropic.com/engineering/advanced-tool-use) - The original Programmatic Tool Calling concept
- [Rhai](https://rhai.rs/) - The embedded scripting engine that makes this possible

## License

MIT
