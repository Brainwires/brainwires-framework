//! Integration tests for Lua language execution.
#![cfg(feature = "lua")]

use brainwires_code_interpreters::{ExecutionRequest, Executor, Language};

fn exec(code: &str) -> brainwires_code_interpreters::ExecutionResult {
    let executor = Executor::new();
    executor.execute(ExecutionRequest {
        language: Language::Lua,
        code: code.to_string(),
        ..Default::default()
    })
}

fn exec_with_context(
    code: &str,
    ctx: serde_json::Value,
) -> brainwires_code_interpreters::ExecutionResult {
    let executor = Executor::new();
    executor.execute(ExecutionRequest {
        language: Language::Lua,
        code: code.to_string(),
        context: Some(ctx),
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// Simple expressions
// ---------------------------------------------------------------------------

#[test]
fn lua_integer_arithmetic() {
    let r = exec("return 2 + 3");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("5"));
}

#[test]
fn lua_float_arithmetic() {
    let r = exec("return 3.14 * 2");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("6.28"));
}

#[test]
fn lua_boolean_expression() {
    let r = exec("return 10 > 5");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("true"));
}

#[test]
fn lua_string_return() {
    let r = exec(r#"return "hello world""#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello world"));
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

#[test]
fn lua_local_variables() {
    let r = exec(
        r#"
        local x = 42
        local y = x * 2
        return y
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("84"));
}

#[test]
fn lua_variable_reassignment() {
    let r = exec(
        r#"
        local x = 1
        x = x + 10
        return x
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("11"));
}

// ---------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------

#[test]
fn lua_for_loop_numeric() {
    let r = exec(
        r#"
        local sum = 0
        for i = 1, 5 do
            sum = sum + i
        end
        return sum
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("15")); // 1+2+3+4+5
}

#[test]
fn lua_while_loop() {
    let r = exec(
        r#"
        local n = 0
        local count = 0
        while n < 100 do
            n = n + 10
            count = count + 1
        end
        return count
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("10"));
}

#[test]
fn lua_repeat_until_loop() {
    let r = exec(
        r#"
        local n = 0
        repeat
            n = n + 1
        until n >= 5
        return n
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("5"));
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[test]
fn lua_function_definition_and_call() {
    let r = exec(
        r#"
        local function factorial(n)
            if n <= 1 then return 1 end
            return n * factorial(n - 1)
        end
        return factorial(5)
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("120"));
}

#[test]
fn lua_closure() {
    let r = exec(
        r#"
        local function make_counter()
            local count = 0
            return function()
                count = count + 1
                return count
            end
        end
        local c = make_counter()
        c()
        c()
        return c()
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3"));
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[test]
fn lua_table_as_array() {
    let r = exec(
        r#"
        local t = {10, 20, 30, 40}
        return #t
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("4"));
}

#[test]
fn lua_table_as_dict() {
    let r = exec(
        r#"
        local t = { name = "test", value = 42 }
        return t.value
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

// ---------------------------------------------------------------------------
// Print / output capture
// ---------------------------------------------------------------------------

#[test]
fn lua_print_captured() {
    let r = exec(
        r#"
        print("line one")
        print("line two")
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("line one"));
    assert!(r.stdout.contains("line two"));
}

#[test]
fn lua_print_multiple_args() {
    let r = exec(r#"print("a", "b", "c")"#);
    assert!(r.success, "Error: {:?}", r.error);
    // Lua print separates args with tabs
    assert!(r.stdout.contains("a"));
    assert!(r.stdout.contains("b"));
    assert!(r.stdout.contains("c"));
}

// ---------------------------------------------------------------------------
// String operations
// ---------------------------------------------------------------------------

#[test]
fn lua_string_upper() {
    let r = exec(r#"return string.upper("hello")"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("HELLO"));
}

#[test]
fn lua_string_format() {
    let r = exec(r#"return string.format("value: %d", 42)"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("value: 42"));
}

// ---------------------------------------------------------------------------
// Context injection
// ---------------------------------------------------------------------------

#[test]
fn lua_context_numeric_values() {
    let r = exec_with_context("return x + y", serde_json::json!({ "x": 100, "y": 200 }));
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("300"));
}

#[test]
fn lua_context_string_value() {
    let r = exec_with_context(
        "return greeting",
        serde_json::json!({ "greeting": "hello from context" }),
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello from context"));
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn lua_syntax_error() {
    let r = exec("local x = ");
    assert!(!r.success);
    assert!(r.error.is_some());
    let err = r.error.unwrap();
    assert!(
        err.to_lowercase().contains("syntax") || err.contains("unexpected"),
        "Unexpected error: {}",
        err,
    );
}

#[test]
fn lua_runtime_error() {
    let r = exec(
        r#"
        local x = nil
        return x.field
        "#,
    );
    assert!(!r.success);
    assert!(r.error.is_some());
}

#[test]
fn lua_type_error() {
    let r = exec(
        r#"
        local x = "hello" + 5
        return x
        "#,
    );
    // Lua actually coerces "hello" + 5 to error since "hello" is not a number string
    assert!(!r.success);
    assert!(r.error.is_some());
}

// ---------------------------------------------------------------------------
// execute_str alias
// ---------------------------------------------------------------------------

#[test]
fn lua_execute_str_alias() {
    let executor = Executor::new();
    let r = executor.execute_str("lua", "return 7 * 6");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

// ---------------------------------------------------------------------------
// Memory tracking
// ---------------------------------------------------------------------------

#[test]
fn lua_memory_used_bytes_populated() {
    let r = exec("return 1");
    assert!(r.success);
    assert!(
        r.memory_used_bytes.is_some(),
        "Lua executor should report memory usage",
    );
}

// ---------------------------------------------------------------------------
// Result values
// ---------------------------------------------------------------------------

#[test]
fn lua_result_json_integer() {
    let r = exec("return 42");
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!(42)));
}

#[test]
fn lua_result_json_string() {
    let r = exec(r#"return "hello""#);
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!("hello")));
}

#[test]
fn lua_result_json_bool() {
    let r = exec("return true");
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!(true)));
}

#[test]
fn lua_nil_result_is_none() {
    let r = exec("local x = 1");
    assert!(r.success);
    // No return means nil, which maps to None
    assert!(r.result.is_none());
}

// ---------------------------------------------------------------------------
// Math library
// ---------------------------------------------------------------------------

#[test]
fn lua_math_sqrt() {
    let r = exec("return math.sqrt(144)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("12"));
}

#[test]
fn lua_math_abs() {
    let r = exec("return math.abs(-42)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}
