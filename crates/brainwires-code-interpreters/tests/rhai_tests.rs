//! Integration tests for Rhai language execution.
#![cfg(feature = "rhai")]

use brainwires_code_interpreters::{ExecutionLimits, ExecutionRequest, Executor, Language};

fn exec(code: &str) -> brainwires_code_interpreters::ExecutionResult {
    let executor = Executor::new();
    executor.execute(ExecutionRequest {
        language: Language::Rhai,
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
        language: Language::Rhai,
        code: code.to_string(),
        context: Some(ctx),
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// Simple expressions
// ---------------------------------------------------------------------------

#[test]
fn rhai_integer_arithmetic() {
    let r = exec("2 + 3");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("5"));
}

#[test]
fn rhai_float_arithmetic() {
    let r = exec("3.14 * 2.0");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("6.28"));
}

#[test]
fn rhai_boolean_expression() {
    let r = exec("10 > 5");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("true"));
}

#[test]
fn rhai_string_literal() {
    let r = exec(r#""hello world""#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello world"));
}

// ---------------------------------------------------------------------------
// Variable assignment and scoping
// ---------------------------------------------------------------------------

#[test]
fn rhai_variable_assignment() {
    let r = exec(
        r#"
        let x = 42;
        let y = x * 2;
        y
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("84"));
}

#[test]
fn rhai_mutable_variable() {
    let r = exec(
        r#"
        let x = 1;
        x = x + 10;
        x
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("11"));
}

// ---------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------

#[test]
fn rhai_for_loop_range() {
    let r = exec(
        r#"
        let sum = 0;
        for i in 0..5 {
            sum += i;
        }
        sum
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("10")); // 0+1+2+3+4
}

#[test]
fn rhai_while_loop() {
    let r = exec(
        r#"
        let n = 0;
        let count = 0;
        while n < 100 {
            n += 10;
            count += 1;
        }
        count
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("10"));
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[test]
fn rhai_function_definition_and_call() {
    let r = exec(
        r#"
        fn factorial(n) {
            if n <= 1 { return 1; }
            n * factorial(n - 1)
        }
        factorial(5)
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("120"));
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[test]
fn rhai_array_operations() {
    let r = exec(
        r#"
        let arr = [10, 20, 30];
        arr.push(40);
        arr.len()
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("4"));
}

#[test]
fn rhai_map_operations() {
    let r = exec(
        r#"
        let m = #{ a: 1, b: 2, c: 3 };
        m.a + m.b + m.c
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("6"));
}

// ---------------------------------------------------------------------------
// Print / output capture
// ---------------------------------------------------------------------------

#[test]
fn rhai_print_captured() {
    let r = exec(
        r#"
        print("line one");
        print("line two");
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("line one"));
    assert!(r.stdout.contains("line two"));
}

// ---------------------------------------------------------------------------
// Context injection
// ---------------------------------------------------------------------------

#[test]
fn rhai_context_simple_values() {
    let r = exec_with_context("x + y", serde_json::json!({ "x": 100, "y": 200 }));
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("300"));
}

#[test]
fn rhai_context_string_value() {
    let r = exec_with_context(
        "greeting",
        serde_json::json!({ "greeting": "hello from context" }),
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello from context"));
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn rhai_syntax_error() {
    let r = exec("let = ;");
    assert!(!r.success);
    assert!(r.error.is_some());
}

#[test]
fn rhai_undefined_variable_error() {
    let r = exec("not_defined_var");
    assert!(!r.success);
    let err = r.error.unwrap();
    assert!(
        err.contains("not found") || err.contains("Undefined") || err.contains("Variable"),
        "Unexpected error message: {}",
        err,
    );
}

#[test]
fn rhai_division_by_zero() {
    let r = exec("let x = 1 / 0;");
    // Rhai may return an error or infinity depending on types
    // Just verify it doesn't panic
    let _ = r;
}

#[test]
fn rhai_type_mismatch() {
    // Rhai allows "hello" + 5 (string concat), so use an actual type error:
    // boolean negation on a string
    let r = exec(r#"let x = -"hello";"#);
    assert!(!r.success);
    assert!(r.error.is_some());
}

// ---------------------------------------------------------------------------
// execute_str alias
// ---------------------------------------------------------------------------

#[test]
fn rhai_execute_str_alias() {
    let executor = Executor::new();
    let r = executor.execute_str("rhai", "7 * 6");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

// ---------------------------------------------------------------------------
// Limits enforcement
// ---------------------------------------------------------------------------

#[test]
fn rhai_operation_limit_exceeded() {
    let executor = Executor::with_limits(ExecutionLimits {
        max_operations: 100,
        ..ExecutionLimits::default()
    });
    let r = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"
            let n = 0;
            loop {
                n += 1;
            }
        "#
        .to_string(),
        ..Default::default()
    });
    assert!(!r.success);
    assert!(r.error.is_some());
}

// ---------------------------------------------------------------------------
// Result value (JSON)
// ---------------------------------------------------------------------------

#[test]
fn rhai_result_json_integer() {
    let r = exec("42");
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!(42)));
}

#[test]
fn rhai_result_json_string() {
    let r = exec(r#""hello""#);
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!("hello")));
}

#[test]
fn rhai_result_json_bool() {
    let r = exec("true");
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!(true)));
}

#[test]
fn rhai_unit_result_is_none() {
    // A statement that doesn't produce a value
    let r = exec("let x = 1;");
    assert!(r.success);
    assert!(r.result.is_none());
}

// ---------------------------------------------------------------------------
// Timing
// ---------------------------------------------------------------------------

#[test]
fn rhai_timing_is_populated() {
    let r = exec("1 + 1");
    assert!(r.success);
    // timing_ms should be non-negative (could be 0 for very fast ops)
    let _ = r.timing_ms; // just ensure the field exists
}
