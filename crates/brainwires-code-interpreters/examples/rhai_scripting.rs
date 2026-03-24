//! Rhai Scripting — variable injection, functions, loops, and error handling
//!
//! Demonstrates the Rhai scripting engine through the unified `Executor`
//! interface: injecting context variables, defining functions, using loops
//! and conditionals, and handling execution errors gracefully.
//!
//! Run:
//! ```sh
//! cargo run -p brainwires-code-interpreters --example rhai_scripting --features rhai
//! ```

use brainwires_code_interpreters::{ExecutionLimits, ExecutionRequest, Executor, Language};

fn main() {
    // 1. Basic expression evaluation
    println!("=== 1. Basic expression ===\n");

    let executor = Executor::new();

    let result = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"let x = 7; let y = 6; x * y"#.to_string(),
        ..Default::default()
    });

    println!("  Code:    let x = 7; let y = 6; x * y");
    println!("  Success: {}", result.success);
    println!("  Output:  {}", result.stdout.trim());
    println!("  Time:    {}ms", result.timing_ms);

    // 2. Variable injection via context
    println!("\n=== 2. Variable injection via context ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"
            let greeting = "Hello, " + name + "!";
            print(greeting);
            "Age next year: " + (age + 1)
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
    println!("  Output:  {}", result.stdout.trim());
    if let Some(ref val) = result.result {
        println!("  Result:  {val}");
    }

    // 3. Function definitions and calls
    println!("\n=== 3. Function definitions ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"
            fn factorial(n) {
                if n <= 1 { return 1; }
                n * factorial(n - 1)
            }

            fn fibonacci(n) {
                if n <= 1 { return n; }
                let a = 0;
                let b = 1;
                for _i in 2..=n {
                    let temp = b;
                    b = a + b;
                    a = temp;
                }
                b
            }

            print("factorial(6) = " + factorial(6));
            print("fibonacci(10) = " + fibonacci(10));
        "#
        .to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    for line in result.stdout.lines() {
        println!("  Output:  {line}");
    }

    // 4. Loops and conditionals
    println!("\n=== 4. Loops and conditionals ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"
            // FizzBuzz for 1..=15
            for i in 1..=15 {
                if i % 15 == 0 {
                    print("FizzBuzz");
                } else if i % 3 == 0 {
                    print("Fizz");
                } else if i % 5 == 0 {
                    print("Buzz");
                } else {
                    print(i);
                }
            }
        "#
        .to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    println!("  Output:  {}", result.stdout.replace('\n', ", "));

    // 5. Arrays and maps
    println!("\n=== 5. Arrays and maps ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"
            let scores = [85, 92, 78, 95, 88];
            let total = 0;
            for s in scores {
                total += s;
            }
            let average = total / scores.len();
            print("Scores:  " + scores);
            print("Count:   " + scores.len());
            print("Average: " + average);

            let student = #{
                name: "Bob",
                grade: "A",
                score: average
            };
            print("Student: " + student.name + " (" + student.grade + ")");
        "#
        .to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    for line in result.stdout.lines() {
        println!("  Output:  {line}");
    }

    // 6. Error handling — syntax error
    println!("\n=== 6. Error handling: syntax error ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"let x = ;"#.to_string(),
        ..Default::default()
    });

    println!("  Code:    let x = ;");
    println!("  Success: {}", result.success);
    if let Some(ref err) = result.error {
        println!("  Error:   {err}");
    }

    // 7. Error handling — undefined variable (strict mode)
    println!("\n=== 7. Error handling: undefined variable ===\n");

    let result = executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"print(missing_variable)"#.to_string(),
        ..Default::default()
    });

    println!("  Code:    print(missing_variable)");
    println!("  Success: {}", result.success);
    if let Some(ref err) = result.error {
        println!("  Error:   {err}");
    }

    // 8. Strict execution limits
    println!("\n=== 8. Strict execution limits ===\n");

    let strict_executor = Executor::with_limits(ExecutionLimits::strict());
    let limits = strict_executor.limits();
    println!("  Max operations: {}", limits.max_operations);
    println!("  Max call depth: {}", limits.max_call_depth);
    println!("  Max timeout:    {}ms", limits.max_timeout_ms);

    // This should succeed under strict limits
    let result = strict_executor.execute(ExecutionRequest {
        language: Language::Rhai,
        code: r#"
            let sum = 0;
            for i in 1..=100 {
                sum += i;
            }
            sum
        "#
        .to_string(),
        ..Default::default()
    });

    println!("  Success: {}", result.success);
    println!("  Output:  {}", result.stdout.trim());

    println!("\nDone.");
}
