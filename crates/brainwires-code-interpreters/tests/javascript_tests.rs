//! Integration tests for JavaScript language execution (Boa engine).
#![cfg(feature = "javascript")]

use brainwires_code_interpreters::{ExecutionRequest, Executor, Language};

fn exec(code: &str) -> brainwires_code_interpreters::ExecutionResult {
    let executor = Executor::new();
    executor.execute(ExecutionRequest {
        language: Language::JavaScript,
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
        language: Language::JavaScript,
        code: code.to_string(),
        context: Some(ctx),
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// Simple expressions
// ---------------------------------------------------------------------------

#[test]
fn js_integer_arithmetic() {
    let r = exec("2 + 3");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("5"));
}

#[test]
fn js_float_arithmetic() {
    let r = exec("3.14 * 2");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("6.28"));
}

#[test]
fn js_boolean_expression() {
    let r = exec("10 > 5");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("true"));
}

#[test]
fn js_string_literal() {
    let r = exec(r#""hello world""#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello world"));
}

#[test]
fn js_template_literal() {
    let r = exec(
        r#"
        let name = "world";
        `hello ${name}`
    "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello world"));
}

// ---------------------------------------------------------------------------
// Variable declarations
// ---------------------------------------------------------------------------

#[test]
fn js_let_variable() {
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
fn js_const_variable() {
    let r = exec(
        r#"
        const PI = 3.14159;
        PI
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3.14159"));
}

// ---------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------

#[test]
fn js_for_loop() {
    let r = exec(
        r#"
        let sum = 0;
        for (let i = 1; i <= 5; i++) {
            sum += i;
        }
        sum
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("15"));
}

#[test]
fn js_while_loop() {
    let r = exec(
        r#"
        let n = 0;
        let count = 0;
        while (n < 100) {
            n += 10;
            count++;
        }
        count
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("10"));
}

#[test]
fn js_for_of_loop() {
    let r = exec(
        r#"
        let sum = 0;
        for (const x of [1, 2, 3, 4, 5]) {
            sum += x;
        }
        sum
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("15"));
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[test]
fn js_function_declaration() {
    let r = exec(
        r#"
        function add(a, b) {
            return a + b;
        }
        add(3, 4)
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("7"));
}

#[test]
fn js_arrow_function() {
    let r = exec(
        r#"
        const multiply = (a, b) => a * b;
        multiply(6, 7)
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

#[test]
fn js_recursive_function() {
    let r = exec(
        r#"
        function factorial(n) {
            if (n <= 1) return 1;
            return n * factorial(n - 1);
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
fn js_array_length() {
    let r = exec(
        r#"
        const arr = [10, 20, 30, 40];
        arr.length
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("4"));
}

#[test]
fn js_array_map_reduce() {
    let r = exec(
        r#"
        const arr = [1, 2, 3, 4, 5];
        arr.map(x => x * 2).reduce((a, b) => a + b, 0)
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("30"));
}

#[test]
fn js_array_filter() {
    let r = exec(
        r#"
        const evens = [1,2,3,4,5,6].filter(x => x % 2 === 0);
        evens.length
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3"));
}

#[test]
fn js_object_property_access() {
    let r = exec(
        r#"
        const obj = { name: "test", value: 42 };
        obj.value
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

#[test]
fn js_destructuring() {
    let r = exec(
        r#"
        const { a, b } = { a: 10, b: 20 };
        a + b
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("30"));
}

// ---------------------------------------------------------------------------
// Console output capture
// ---------------------------------------------------------------------------

#[test]
fn js_console_log_captured() {
    let r = exec(
        r#"
        console.log("line one");
        console.log("line two");
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("line one"));
    assert!(r.stdout.contains("line two"));
}

#[test]
fn js_console_log_multiple_args() {
    let r = exec(r#"console.log("a", "b", "c")"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("a"));
    assert!(r.stdout.contains("b"));
    assert!(r.stdout.contains("c"));
}

#[test]
fn js_console_error_captured_in_stderr() {
    let r = exec(r#"console.error("oops")"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stderr.contains("oops"));
}

#[test]
fn js_console_warn_captured() {
    let r = exec(r#"console.warn("warning!")"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("warning!"));
}

// ---------------------------------------------------------------------------
// String methods
// ---------------------------------------------------------------------------

#[test]
fn js_string_to_upper() {
    let r = exec(r#""hello world".toUpperCase()"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("HELLO WORLD"));
}

#[test]
fn js_string_includes() {
    let r = exec(r#""hello world".includes("world")"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("true"));
}

#[test]
fn js_string_split() {
    let r = exec(r#""a,b,c".split(",").length"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3"));
}

// ---------------------------------------------------------------------------
// JSON operations
// ---------------------------------------------------------------------------

#[test]
fn js_json_stringify() {
    let r = exec(r#"JSON.stringify({ a: 1, b: 2 })"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("a"));
    assert!(r.stdout.contains("b"));
}

#[test]
fn js_json_parse() {
    let r = exec(
        r#"
        const obj = JSON.parse('{"x": 42}');
        obj.x
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

// ---------------------------------------------------------------------------
// Context injection
// ---------------------------------------------------------------------------

#[test]
fn js_context_numeric_values() {
    let r = exec_with_context("x + y", serde_json::json!({ "x": 100, "y": 200 }));
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("300"));
}

#[test]
fn js_context_string_value() {
    let r = exec_with_context(
        "greeting",
        serde_json::json!({ "greeting": "hello from context" }),
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello from context"));
}

#[test]
fn js_context_array_value() {
    let r = exec_with_context("items.length", serde_json::json!({ "items": [1, 2, 3] }));
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3"));
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn js_syntax_error() {
    let r = exec("let x = ;");
    assert!(!r.success);
    assert!(r.error.is_some());
}

#[test]
fn js_reference_error() {
    let r = exec("undefined_variable");
    assert!(!r.success);
    let err = r.error.unwrap();
    assert!(
        err.contains("not defined") || err.contains("ReferenceError") || err.contains("not found"),
        "Unexpected error: {}",
        err,
    );
}

#[test]
fn js_type_error() {
    let r = exec("null.property");
    assert!(!r.success);
    assert!(r.error.is_some());
}

#[test]
fn js_throw_error() {
    let r = exec(r#"throw new Error("custom error")"#);
    assert!(!r.success);
    let err = r.error.unwrap();
    assert!(err.contains("custom error"), "Unexpected error: {}", err,);
}

// ---------------------------------------------------------------------------
// Modern JS features
// ---------------------------------------------------------------------------

#[test]
fn js_spread_operator() {
    let r = exec(
        r#"
        const a = [1, 2, 3];
        const b = [...a, 4, 5];
        b.length
        "#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("5"));
}

#[test]
fn js_ternary_operator() {
    let r = exec("true ? 42 : 0");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

#[test]
fn js_nullish_coalescing() {
    let r = exec("null ?? 42");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

// ---------------------------------------------------------------------------
// execute_str alias
// ---------------------------------------------------------------------------

#[test]
fn js_execute_str_alias() {
    let executor = Executor::new();
    let r = executor.execute_str("js", "7 * 6");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

#[test]
fn js_execute_str_javascript_alias() {
    let executor = Executor::new();
    let r = executor.execute_str("javascript", "7 * 6");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

// ---------------------------------------------------------------------------
// Result values
// ---------------------------------------------------------------------------

#[test]
fn js_result_json_integer() {
    let r = exec("42");
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!(42)));
}

#[test]
fn js_result_json_string() {
    let r = exec(r#""hello""#);
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!("hello")));
}

#[test]
fn js_result_json_bool() {
    let r = exec("true");
    assert!(r.success);
    assert_eq!(r.result, Some(serde_json::json!(true)));
}

#[test]
fn js_undefined_result_is_none() {
    let r = exec("undefined");
    assert!(r.success);
    assert!(r.result.is_none());
}

// ---------------------------------------------------------------------------
// Math object
// ---------------------------------------------------------------------------

#[test]
fn js_math_sqrt() {
    let r = exec("Math.sqrt(144)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("12"));
}

#[test]
fn js_math_abs() {
    let r = exec("Math.abs(-42)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

#[test]
fn js_math_max() {
    let r = exec("Math.max(1, 5, 3, 2, 4)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("5"));
}
