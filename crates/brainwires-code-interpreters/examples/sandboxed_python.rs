//! Sandboxed Python Execution — output capture, variables, and error handling
//!
//! Demonstrates running Python code through the sandboxed `Executor` interface:
//! output capture via `print()`, context variable injection, multi-line scripts,
//! error handling, and execution limits that constrain untrusted code.
//!
//! **Note:** Requires the `python` feature, which depends on RustPython.
//! The dependency may be temporarily disabled; see Cargo.toml for status.
//!
//! Run:
//! ```sh
//! cargo run -p brainwires-code-interpreters --example sandboxed_python --features python
//! ```

use brainwires_code_interpreters::{ExecutionLimits, ExecutionRequest, Executor, Language};

fn main() {
    // 1. Basic print and output capture
    println!("=== 1. Basic output capture ===\n");

    let executor = Executor::new();

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"print("Hello from sandboxed Python!")"#.to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    println!("  Output:  {}", result.stdout.trim());
    println!("  Time:    {}ms", result.timing_ms);

    // 2. Variable injection via context
    println!("\n=== 2. Variable injection via context ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"
greeting = f"Hello, {name}! You are {age} years old."
print(greeting)
print(f"Next year you will be {age + 1}")
"#
        .to_string(),
        context: Some(serde_json::json!({
            "name": "Alice",
            "age": 30
        })),
        ..Default::default()
    });

    println!("  Context: name=\"Alice\", age=30");
    println!("  Success: {}", result.success);
    for line in result.stdout.lines() {
        println!("  Output:  {line}");
    }

    // 3. Multi-line script with functions and loops
    println!("\n=== 3. Functions and loops ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

def fibonacci(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

print(f"factorial(6) = {factorial(6)}")
print(f"fibonacci(10) = {fibonacci(10)}")

# List comprehension
squares = [x**2 for x in range(1, 6)]
print(f"squares(1..5) = {squares}")
"#
        .to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    for line in result.stdout.lines() {
        println!("  Output:  {line}");
    }

    // 4. Data structures and string methods
    println!("\n=== 4. Data structures and string methods ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"
# Dictionaries
student = {"name": "Bob", "scores": [85, 92, 78, 95, 88]}
avg = sum(student["scores"]) / len(student["scores"])
print(f"{student['name']}'s average: {avg}")

# String methods
message = "hello world"
print(f"upper: {message.upper()}")
print(f"title: {message.title()}")
print(f"words: {message.split()}")
"#
        .to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    for line in result.stdout.lines() {
        println!("  Output:  {line}");
    }

    // 5. Context with complex data (lists, nested objects)
    println!("\n=== 5. Complex context injection ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"
total = sum(values)
print(f"Values: {values}")
print(f"Sum:    {total}")
print(f"Config: host={config['host']}, port={config['port']}")
"#
        .to_string(),
        context: Some(serde_json::json!({
            "values": [10, 20, 30, 40],
            "config": {
                "host": "localhost",
                "port": 8080
            }
        })),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    for line in result.stdout.lines() {
        println!("  Output:  {line}");
    }

    // 6. Error handling — syntax error
    println!("\n=== 6. Error handling: syntax error ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"def incomplete("#.to_string(),
        ..Default::default()
    });

    println!("  Code:    def incomplete(");
    println!("  Success: {}", result.success);
    if let Some(ref err) = result.error {
        println!("  Error:   {err}");
    }

    // 7. Error handling — runtime error
    println!("\n=== 7. Error handling: runtime error (NameError) ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"print(undefined_variable)"#.to_string(),
        ..Default::default()
    });

    println!("  Code:    print(undefined_variable)");
    println!("  Success: {}", result.success);
    if let Some(ref err) = result.error {
        println!("  Error:   {err}");
    }

    // 8. Error handling — type error
    println!("\n=== 8. Error handling: TypeError ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"result = "hello" + 42"#.to_string(),
        ..Default::default()
    });

    println!("  Code:    result = \"hello\" + 42");
    println!("  Success: {}", result.success);
    if let Some(ref err) = result.error {
        println!("  Error:   {err}");
    }

    // 9. Sandboxed execution with strict limits
    println!("\n=== 9. Strict sandbox limits ===\n");

    let strict_executor = Executor::with_limits(ExecutionLimits::strict());
    let limits = strict_executor.limits();
    println!("  Max timeout:    {}ms", limits.max_timeout_ms);
    println!("  Max memory:     {}MB", limits.max_memory_mb);
    println!("  Max operations: {}", limits.max_operations);

    let result = strict_executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"
total = sum(range(1, 101))
print(f"Sum 1..100 = {total}")
"#
        .to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    println!("  Output:  {}", result.stdout.trim());

    // 10. Demonstrating the sandbox — no file system access
    println!("\n=== 10. Sandbox: restricted I/O ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Python,
        code: r#"
try:
    with open("/etc/passwd") as f:
        print(f.read())
except Exception as e:
    print(f"Blocked: {type(e).__name__}: {e}")
"#
        .to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    println!("  Output:  {}", result.stdout.trim());

    println!("\nDone.");
}
