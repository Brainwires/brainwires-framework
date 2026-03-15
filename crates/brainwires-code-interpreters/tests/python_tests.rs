//! Integration tests for Python language execution (RustPython).
#![cfg(feature = "python")]

use brainwires_code_interpreters::{ExecutionRequest, Executor, Language};

fn exec(code: &str) -> brainwires_code_interpreters::ExecutionResult {
    let executor = Executor::new();
    executor.execute(ExecutionRequest {
        language: Language::Python,
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
        language: Language::Python,
        code: code.to_string(),
        context: Some(ctx),
        ..Default::default()
    })
}

// ---------------------------------------------------------------------------
// Simple expressions via print
// ---------------------------------------------------------------------------

#[test]
fn python_integer_arithmetic() {
    let r = exec("print(2 + 3)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("5"));
}

#[test]
fn python_float_arithmetic() {
    let r = exec("print(3.14 * 2)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("6.28"));
}

#[test]
fn python_boolean_expression() {
    let r = exec("print(10 > 5)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("True"));
}

#[test]
fn python_string_print() {
    let r = exec(r#"print("hello world")"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello world"));
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

#[test]
fn python_variable_assignment() {
    let r = exec(
        r#"
x = 42
y = x * 2
print(y)
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("84"));
}

#[test]
fn python_multiple_assignment() {
    let r = exec(
        r#"
a, b, c = 1, 2, 3
print(a + b + c)
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("6"));
}

// ---------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------

#[test]
fn python_for_loop_range() {
    let r = exec(
        r#"
total = 0
for i in range(1, 6):
    total += i
print(total)
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("15")); // 1+2+3+4+5
}

#[test]
fn python_while_loop() {
    let r = exec(
        r#"
n = 0
count = 0
while n < 100:
    n += 10
    count += 1
print(count)
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("10"));
}

#[test]
fn python_for_loop_over_list() {
    let r = exec(
        r#"
items = ["a", "b", "c"]
result = ""
for item in items:
    result += item
print(result)
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("abc"));
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

#[test]
fn python_function_definition() {
    let r = exec(
        r#"
def add(a, b):
    return a + b

print(add(3, 4))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("7"));
}

#[test]
fn python_recursive_function() {
    let r = exec(
        r#"
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

print(factorial(5))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("120"));
}

#[test]
fn python_lambda() {
    let r = exec(
        r#"
double = lambda x: x * 2
print(double(21))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

#[test]
fn python_default_arguments() {
    let r = exec(
        r#"
def greet(name, greeting="Hello"):
    return f"{greeting}, {name}!"

print(greet("World"))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("Hello, World!"));
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[test]
fn python_list_operations() {
    let r = exec(
        r#"
arr = [1, 2, 3]
arr.append(4)
print(len(arr))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("4"));
}

#[test]
fn python_list_comprehension() {
    let r = exec(
        r#"
squares = [x**2 for x in range(5)]
print(squares)
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("[0, 1, 4, 9, 16]"));
}

#[test]
fn python_dict_operations() {
    let r = exec(
        r#"
d = {"name": "test", "value": 42}
print(d["value"])
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

#[test]
fn python_dict_comprehension() {
    let r = exec(
        r#"
d = {x: x**2 for x in range(4)}
print(d[3])
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("9"));
}

#[test]
fn python_tuple() {
    let r = exec(
        r#"
t = (1, 2, 3)
print(len(t))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3"));
}

#[test]
fn python_set_operations() {
    let r = exec(
        r#"
s = {1, 2, 3, 2, 1}
print(len(s))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3"));
}

// ---------------------------------------------------------------------------
// String operations
// ---------------------------------------------------------------------------

#[test]
fn python_string_upper() {
    let r = exec(
        r#"
s = "hello world"
print(s.upper())
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("HELLO WORLD"));
}

#[test]
fn python_string_split() {
    let r = exec(
        r#"
parts = "a,b,c".split(",")
print(len(parts))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3"));
}

#[test]
fn python_fstring() {
    let r = exec(
        r#"
name = "World"
print(f"Hello, {name}!")
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("Hello, World!"));
}

// ---------------------------------------------------------------------------
// Print / output capture
// ---------------------------------------------------------------------------

#[test]
fn python_print_multiple_lines() {
    let r = exec(
        r#"
print("line one")
print("line two")
print("line three")
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("line one"));
    assert!(r.stdout.contains("line two"));
    assert!(r.stdout.contains("line three"));
}

#[test]
fn python_print_with_sep() {
    let r = exec(r#"print("a", "b", "c", sep="-")"#);
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("a-b-c"));
}

// ---------------------------------------------------------------------------
// Context injection
// ---------------------------------------------------------------------------

#[test]
fn python_context_numeric_values() {
    let r = exec_with_context("print(x + y)", serde_json::json!({ "x": 100, "y": 200 }));
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("300"));
}

#[test]
fn python_context_string_value() {
    let r = exec_with_context(
        "print(greeting)",
        serde_json::json!({ "greeting": "hello from context" }),
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("hello from context"));
}

#[test]
fn python_context_list_value() {
    let r = exec_with_context(
        "print(len(items))",
        serde_json::json!({ "items": [1, 2, 3, 4] }),
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("4"));
}

#[test]
fn python_context_dict_value() {
    let r = exec_with_context(
        r#"print(data["key"])"#,
        serde_json::json!({ "data": { "key": "value" } }),
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("value"));
}

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

#[test]
fn python_syntax_error() {
    let r = exec("def incomplete(");
    assert!(!r.success);
    assert!(r.error.is_some());
    let err = r.error.unwrap();
    assert!(
        err.contains("Syntax") || err.contains("syntax") || err.contains("SyntaxError"),
        "Unexpected error: {}",
        err,
    );
}

#[test]
fn python_name_error() {
    let r = exec("undefined_variable");
    assert!(!r.success);
    let err = r.error.unwrap();
    assert!(
        err.contains("NameError"),
        "Expected NameError, got: {}",
        err,
    );
}

#[test]
fn python_type_error() {
    let r = exec(r#"result = "hello" + 5"#);
    assert!(!r.success);
    let err = r.error.unwrap();
    assert!(
        err.contains("TypeError"),
        "Expected TypeError, got: {}",
        err,
    );
}

#[test]
fn python_index_error() {
    let r = exec(
        r#"
arr = [1, 2, 3]
print(arr[10])
"#,
    );
    assert!(!r.success);
    let err = r.error.unwrap();
    assert!(
        err.contains("IndexError"),
        "Expected IndexError, got: {}",
        err,
    );
}

#[test]
fn python_key_error() {
    let r = exec(
        r#"
d = {"a": 1}
print(d["missing"])
"#,
    );
    assert!(!r.success);
    let err = r.error.unwrap();
    assert!(err.contains("KeyError"), "Expected KeyError, got: {}", err,);
}

#[test]
fn python_zero_division_error() {
    let r = exec("print(1 / 0)");
    assert!(!r.success);
    let err = r.error.unwrap();
    assert!(
        err.contains("ZeroDivisionError"),
        "Expected ZeroDivisionError, got: {}",
        err,
    );
}

// ---------------------------------------------------------------------------
// Stdlib imports
// ---------------------------------------------------------------------------

#[test]
fn python_math_import() {
    let r = exec(
        r#"
import math
print(math.sqrt(144))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("12"));
}

#[test]
fn python_math_pi() {
    let r = exec(
        r#"
import math
print(math.pi)
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("3.14"));
}

// ---------------------------------------------------------------------------
// Classes
// ---------------------------------------------------------------------------

#[test]
fn python_class_definition() {
    let r = exec(
        r#"
class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def distance(self):
        return (self.x**2 + self.y**2) ** 0.5

p = Point(3, 4)
print(p.distance())
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("5"));
}

// ---------------------------------------------------------------------------
// Exception handling
// ---------------------------------------------------------------------------

#[test]
fn python_try_except() {
    let r = exec(
        r#"
try:
    x = 1 / 0
except ZeroDivisionError:
    print("caught")
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("caught"));
}

#[test]
fn python_try_except_as() {
    let r = exec(
        r#"
try:
    int("not_a_number")
except ValueError as e:
    print(f"Error: {e}")
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("Error:"));
}

// ---------------------------------------------------------------------------
// execute_str alias
// ---------------------------------------------------------------------------

#[test]
fn python_execute_str_alias() {
    let executor = Executor::new();
    let r = executor.execute_str("python", "print(7 * 6)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

#[test]
fn python_execute_str_py_alias() {
    let executor = Executor::new();
    let r = executor.execute_str("py", "print(7 * 6)");
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("42"));
}

// ---------------------------------------------------------------------------
// Built-in functions
// ---------------------------------------------------------------------------

#[test]
fn python_builtin_sorted() {
    let r = exec(
        r#"
print(sorted([3, 1, 4, 1, 5, 9, 2, 6]))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("[1, 1, 2, 3, 4, 5, 6, 9]"));
}

#[test]
fn python_builtin_enumerate() {
    let r = exec(
        r#"
result = []
for i, v in enumerate(["a", "b", "c"]):
    result.append(f"{i}:{v}")
print(",".join(result))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("0:a,1:b,2:c"));
}

#[test]
fn python_builtin_zip() {
    let r = exec(
        r#"
a = [1, 2, 3]
b = ["x", "y", "z"]
print(list(zip(a, b)))
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    assert!(r.stdout.contains("(1, 'x')"));
}

// ---------------------------------------------------------------------------
// Multiple outputs
// ---------------------------------------------------------------------------

#[test]
fn python_output_ordering() {
    let r = exec(
        r#"
print("first")
print("second")
print("third")
"#,
    );
    assert!(r.success, "Error: {:?}", r.error);
    let first_pos = r.stdout.find("first").unwrap();
    let second_pos = r.stdout.find("second").unwrap();
    let third_pos = r.stdout.find("third").unwrap();
    assert!(first_pos < second_pos);
    assert!(second_pos < third_pos);
}
