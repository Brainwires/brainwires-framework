//! Multi-Language Code Execution — sandboxed Rhai and Lua interpreters
//!
//! Demonstrates running code in multiple languages through the unified
//! Executor interface, with both default and custom execution limits.

use brainwires_code_interpreters::{ExecutionLimits, ExecutionRequest, Executor, Language};

fn main() {
    // 1. Setup — create executor, list supported languages
    println!("=== 1. Setup: Supported languages ===\n");

    let executor = Executor::new();
    let languages = executor.supported_languages();
    for lang in &languages {
        println!("  - {lang}");
    }
    println!("  Total: {}", languages.len());

    // 2. Rhai — sum 1 to 10
    println!("\n=== 2. Rhai: Sum 1..=10 ===\n");

    let rhai_result = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: "let sum = 0; for i in 1..=10 { sum += i; } sum".to_string(),
        ..Default::default()
    });

    println!("  Success: {}", rhai_result.success);
    println!("  Output:  {}", rhai_result.stdout.trim());
    println!("  Time:    {}ms", rhai_result.timing_ms);
    if let Some(err) = &rhai_result.error {
        println!("  Error:   {err}");
    }

    // 3. Lua — sum 1 to 10
    println!("\n=== 3. Lua: Sum 1..=10 ===\n");

    let lua_result = executor.execute(ExecutionRequest {
        language: Language::Lua,
        code: "local sum = 0; for i = 1, 10 do sum = sum + i end return sum".to_string(),
        ..Default::default()
    });

    println!("  Success: {}", lua_result.success);
    println!("  Output:  {}", lua_result.stdout.trim());
    println!("  Time:    {}ms", lua_result.timing_ms);
    if let Some(err) = &lua_result.error {
        println!("  Error:   {err}");
    }

    // 4. Strict limits — execute with custom tight constraints
    println!("\n=== 4. Strict limits: Rhai with tight sandbox ===\n");

    let strict_executor = Executor::with_limits(ExecutionLimits::strict());
    let limits = strict_executor.limits();
    println!("  Timeout:    {}ms", limits.max_timeout_ms);
    println!("  Memory:     {}MB", limits.max_memory_mb);
    println!("  Operations: {}", limits.max_operations);
    println!("  Call depth: {}", limits.max_call_depth);

    let strict_result = strict_executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: "let x = 2; let y = 3; x * y".to_string(),
        ..Default::default()
    });

    println!("  Success: {}", strict_result.success);
    println!("  Output:  {}", strict_result.stdout.trim());
    println!("  Time:    {}ms", strict_result.timing_ms);

    // 5. Timing comparison
    println!("\n=== 5. Timing comparison ===\n");

    println!("  Rhai: {}ms", rhai_result.timing_ms);
    println!("  Lua:  {}ms", lua_result.timing_ms);

    println!("\nDone.");
}
