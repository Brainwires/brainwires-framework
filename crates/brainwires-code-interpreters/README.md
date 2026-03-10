# brainwires-code-interpreters

[![Crates.io](https://img.shields.io/crates/v/brainwires-code-interpreters.svg)](https://crates.io/crates/brainwires-code-interpreters)
[![Documentation](https://img.shields.io/docsrs/brainwires-code-interpreters)](https://docs.rs/brainwires-code-interpreters)
[![License](https://img.shields.io/crates/l/brainwires-code-interpreters.svg)](LICENSE)

Sandboxed code execution for multiple languages (Rhai, Lua, JavaScript, Python).

## Overview

`brainwires-code-interpreters` provides a unified `Executor` that dispatches code to language-specific interpreters, all running in-process with configurable safety limits. Each interpreter captures stdout/stderr, accepts JSON context injection, and returns structured results — making it suitable for AI agent tool use, REPL environments, and WASM targets.

**Design principles:**

- **Unified API** — one `Executor::execute()` method for all languages
- **Sandboxed** — configurable limits on time, memory, operations, output size, call depth, and data structure sizes
- **Context injection** — pass JSON values into the execution scope as native variables
- **WASM-ready** — full `wasm-bindgen` bindings with the `wasm` feature

```text
                  ┌────────────────────────────────────────┐
                  │              Executor                   │
                  │                                        │
  ExecutionRequest│  language ──► dispatch                  │
  ────────────────►                │                        │
                  │    ┌───────────┼───────────┐           │
                  │    ▼           ▼           ▼           │
                  │  ┌─────┐  ┌──────┐  ┌──────────┐      │
                  │  │ Rhai │  │ Lua  │  │JavaScript│ ...  │
                  │  │      │  │ 5.4  │  │  (Boa)   │      │
                  │  └──┬───┘  └──┬───┘  └────┬─────┘      │
                  │     │         │           │            │
                  │     └─────────┼───────────┘            │
                  │               ▼                        │
                  │        ExecutionResult                  │
  ◄───────────────│   { success, stdout, stderr,           │
                  │     result, timing_ms, ... }           │
                  │                                        │
                  │  ┌──────────────────────────┐          │
                  │  │    ExecutionLimits        │          │
                  │  │  timeout · memory · ops   │          │
                  │  │  output · depth · sizes   │          │
                  │  └──────────────────────────┘          │
                  └────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-code-interpreters = "0.1"
```

Execute code:

```rust
use brainwires_code_interpreters::{Executor, ExecutionRequest, Language};

let executor = Executor::new();

let result = executor.execute(ExecutionRequest {
    language: Language::Rhai,
    code: r#"let x = 40 + 2; print(x); x"#.into(),
    ..Default::default()
});

assert!(result.success);
assert!(result.stdout.contains("42"));
```

Or use the convenience method:

```rust
let result = executor.execute_str("lua", r#"print("Hello from Lua!")"#);
assert!(result.success);
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Native target support (marker feature) |
| `rhai` | Yes | Rhai scripting language |
| `lua` | Yes | Lua 5.4 via mlua (vendored) |
| `javascript` | No | JavaScript ES2022+ via Boa engine |
| `python` | No | Python 3.12 compatible via RustPython |
| `all-languages` | No | Enables all four language interpreters |
| `wasm` | No | WASM target with `wasm-bindgen` exports |

Enable features in `Cargo.toml`:

```toml
# Default (Rhai + Lua)
brainwires-code-interpreters = "0.1"

# All languages
brainwires-code-interpreters = { version = "0.2", features = ["all-languages"] }

# WASM target
brainwires-code-interpreters = { version = "0.2", default-features = false, features = ["wasm", "rhai", "lua"] }
```

## Architecture

### Executor

The central dispatch point. Routes `ExecutionRequest` to the correct language interpreter and enforces limits.

```rust
pub struct Executor { /* limits */ }

impl Executor {
    pub fn new() -> Self;
    pub fn with_limits(limits: ExecutionLimits) -> Self;
    pub fn execute(&self, request: ExecutionRequest) -> ExecutionResult;
    pub fn execute_str(&self, language: &str, code: &str) -> ExecutionResult;
    pub fn supported_languages(&self) -> Vec<Language>;
    pub fn is_supported(&self, language: Language) -> bool;
    pub fn limits(&self) -> &ExecutionLimits;
}
```

### Language Trait

All interpreters implement `LanguageExecutor`:

```rust
pub trait LanguageExecutor {
    fn execute(&self, request: &ExecutionRequest) -> ExecutionResult;
    fn language_name(&self) -> &'static str;
    fn language_version(&self) -> String;
}
```

**Built-in implementations:**

| Struct | Feature | Language | Version |
|--------|---------|----------|---------|
| `RhaiExecutor` | `rhai` | Rhai | 1.20 |
| `LuaExecutor` | `lua` | Lua | 5.4 |
| `JavaScriptExecutor` | `javascript` | JavaScript | ES2022+ (Boa 0.20) |
| `PythonExecutor` | `python` | Python | 3.12 (RustPython 0.4) |

### Language Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rhai,
    Lua,
    JavaScript,
    Python,
}
```

| Method | Description |
|--------|-------------|
| `as_str()` | `"rhai"`, `"lua"`, `"javascript"`, `"python"` |
| `from_str(s)` | Case-insensitive, accepts `"js"` and `"py"` aliases |
| `extension()` | File extension: `"rhai"`, `"lua"`, `"js"`, `"py"` |

### Execution Request & Result

**`ExecutionRequest`:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `language` | `Language` | `Rhai` | Target language |
| `code` | `String` | `""` | Source code to execute |
| `stdin` | `Option<String>` | `None` | Standard input |
| `timeout_ms` | `u64` | 30,000 | Execution timeout |
| `memory_limit_mb` | `u32` | 256 | Memory ceiling |
| `context` | `Option<Value>` | `None` | JSON values injected as native variables |
| `limits` | `Option<ExecutionLimits>` | `None` | Override executor-level limits |

**`ExecutionResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `success` | `bool` | Whether execution completed without error |
| `stdout` | `String` | Captured standard output |
| `stderr` | `String` | Captured standard error |
| `result` | `Option<Value>` | Return value as JSON |
| `error` | `Option<String>` | Error message on failure |
| `timing_ms` | `u64` | Wall-clock execution time |
| `memory_used_bytes` | `Option<u64>` | Memory consumed (when available) |
| `operations_count` | `Option<u64>` | Operations executed (when tracked) |

Constructors: `ExecutionResult::success(stdout, result, timing_ms)`, `ExecutionResult::error(msg, timing_ms)`, `ExecutionResult::error_with_output(msg, stdout, stderr, timing_ms)`.

### Execution Limits

Configurable safety boundaries applied to every execution.

| Field | Default | Strict | Relaxed |
|-------|---------|--------|---------|
| `max_timeout_ms` | 30,000 | 5,000 | 120,000 |
| `max_memory_mb` | 256 | 64 | 512 |
| `max_output_bytes` | 1 MB | 64 KB | 10 MB |
| `max_operations` | 1,000,000 | 100,000 | 10,000,000 |
| `max_call_depth` | 64 | 32 | 128 |
| `max_string_length` | 10 MB | 1 MB | 100 MB |
| `max_array_length` | 100,000 | 10,000 | 1,000,000 |
| `max_map_size` | 10,000 | 1,000 | 100,000 |

Presets: `ExecutionLimits::default()`, `ExecutionLimits::strict()`, `ExecutionLimits::relaxed()`.

### Sandbox Profiles

Higher-level categorization of what standard library modules are available:

| Profile | Allowed Modules |
|---------|----------------|
| `Minimal` | `math` |
| `Standard` | `math`, `json`, `string`, `array`, `print` |
| `Extended` | `math`, `json`, `string`, `array`, `print`, `datetime`, `regex`, `base64` |

### Error Types

**`ExecutionError` variants:**

| Variant | Description |
|---------|-------------|
| `UnsupportedLanguage(String)` | Language not enabled or not recognized |
| `Timeout(u64)` | Execution exceeded timeout (ms) |
| `MemoryLimitExceeded(u32)` | Memory ceiling reached (MB) |
| `OperationLimitExceeded(u64)` | Operation count ceiling reached |
| `OutputTooLarge(usize)` | Output exceeded max bytes |
| `SyntaxError(String)` | Code failed to parse |
| `RuntimeError(String)` | Error during execution |
| `InternalError(String)` | Internal interpreter error |

All variants convert to `ExecutionResult` via `error.to_result(timing_ms)`.

### WASM Bindings

When the `wasm` feature is enabled, `WasmExecutor` is exported via `wasm-bindgen`:

```rust
#[wasm_bindgen]
impl WasmExecutor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self;
    pub fn with_strict_limits() -> Self;
    pub fn with_relaxed_limits() -> Self;
    pub fn execute(&self, language: &str, code: &str) -> Result<JsValue, JsValue>;
    pub fn execute_request(&self, request_json: &str) -> Result<JsValue, JsValue>;
    pub fn supported_languages(&self) -> Vec<String>;
    pub fn is_supported(&self, language: &str) -> bool;
}
```

Global functions are also exported: `execute_code(language, code)` and `get_supported_languages()`.

## Usage Examples

### Basic Execution

```rust
use brainwires_code_interpreters::{Executor, ExecutionRequest, Language};

let executor = Executor::new();

let result = executor.execute(ExecutionRequest {
    language: Language::Lua,
    code: r#"
        local sum = 0
        for i = 1, 100 do sum = sum + i end
        print("Sum: " .. sum)
        return sum
    "#.into(),
    ..Default::default()
});

assert!(result.success);
assert!(result.stdout.contains("Sum: 5050"));
```

### Context Injection

```rust
use brainwires_code_interpreters::{Executor, ExecutionRequest, Language};

let executor = Executor::new();

let result = executor.execute(ExecutionRequest {
    language: Language::Rhai,
    code: r#"let greeting = name + " is " + age + " years old"; greeting"#.into(),
    context: Some(serde_json::json!({
        "name": "Alice",
        "age": 30
    })),
    ..Default::default()
});

assert!(result.success);
```

### Strict Limits for Untrusted Code

```rust
use brainwires_code_interpreters::{Executor, ExecutionRequest, ExecutionLimits, Language};

let executor = Executor::with_limits(ExecutionLimits::strict());

let result = executor.execute(ExecutionRequest {
    language: Language::Lua,
    code: "print('safe code')".into(),
    ..Default::default()
});

assert!(result.success);
```

### JavaScript with Console Output

```rust
use brainwires_code_interpreters::{Executor, ExecutionRequest, Language};

let executor = Executor::new();

let result = executor.execute(ExecutionRequest {
    language: Language::JavaScript,
    code: r#"
        const items = [1, 2, 3, 4, 5];
        const doubled = items.map(x => x * 2);
        console.log("Doubled:", JSON.stringify(doubled));
        doubled.reduce((a, b) => a + b, 0)
    "#.into(),
    ..Default::default()
});

assert!(result.success);
assert!(result.stdout.contains("Doubled:"));
```

### Python Execution

```rust
use brainwires_code_interpreters::{Executor, ExecutionRequest, Language};

let executor = Executor::new();

let result = executor.execute(ExecutionRequest {
    language: Language::Python,
    code: r#"
def fibonacci(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

result = fibonacci(10)
print(f"fib(10) = {result}")
    "#.into(),
    ..Default::default()
});

assert!(result.success);
assert!(result.stdout.contains("fib(10) = 55"));
```

### Language-Specific Executor

```rust
use brainwires_code_interpreters::{lang::RhaiExecutor, ExecutionRequest, Language};
use brainwires_code_interpreters::languages::LanguageExecutor;

let rhai = RhaiExecutor::new();

println!("{} v{}", rhai.language_name(), rhai.language_version());

let result = rhai.execute(&ExecutionRequest {
    language: Language::Rhai,
    code: "40 + 2".into(),
    ..Default::default()
});

assert!(result.success);
```

### Check Supported Languages

```rust
use brainwires_code_interpreters::{supported_languages, is_language_supported, Language};

let languages = supported_languages();
for lang in &languages {
    println!("{} (.{}) - supported: {}", lang, lang.extension(), true);
}

assert!(is_language_supported(Language::Rhai));
```

### Custom Execution Limits

```rust
use brainwires_code_interpreters::{Executor, ExecutionLimits};

let limits = ExecutionLimits {
    max_timeout_ms: 10_000,
    max_memory_mb: 128,
    max_output_bytes: 512_000,
    max_operations: 500_000,
    max_call_depth: 32,
    max_string_length: 5_242_880,
    max_array_length: 50_000,
    max_map_size: 5_000,
};

let executor = Executor::with_limits(limits);
let result = executor.execute_str("rhai", "40 + 2");
assert!(result.success);
```

## Configuration

### ExecutionLimits Presets

| Preset | Use Case | Timeout | Memory | Operations |
|--------|----------|---------|--------|------------|
| `default()` | General use | 30s | 256 MB | 1M |
| `strict()` | Untrusted code | 5s | 64 MB | 100K |
| `relaxed()` | Trusted / long-running | 120s | 512 MB | 10M |

### SandboxProfile

```rust
use brainwires_code_interpreters::SandboxProfile;

let modules = SandboxProfile::Extended.allowed_modules();
// ["math", "json", "string", "array", "print", "datetime", "regex", "base64"]
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.2", features = ["interpreter-all"] }
```

Or use standalone — `brainwires-code-interpreters` has no dependency on any other Brainwires crate.

## License

Licensed under the [MIT License](LICENSE-MIT).
