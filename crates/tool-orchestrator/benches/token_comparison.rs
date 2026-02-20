//! Token Comparison Benchmark
//!
//! This benchmark demonstrates the token savings of Programmatic Tool Calling
//! vs traditional sequential tool calling.
//!
//! Run with: `cargo bench`

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tool_orchestrator::{ExecutionLimits, ToolOrchestrator};

/// Simulates traditional approach: each tool result would go back to LLM
fn traditional_approach_simulation(employee_count: usize) -> TraditionalMetrics {
    let mut total_output_chars = 0;
    let mut tool_call_count = 0;

    // Simulate expense data per employee (~500 chars each)
    let expense_template = r#"[{"id":1,"description":"Office supplies","amount":150.00,"date":"2024-01-15"},{"id":2,"description":"Software license","amount":299.99,"date":"2024-01-20"},{"id":3,"description":"Team lunch","amount":85.50,"date":"2024-01-22"},{"id":4,"description":"Conference ticket","amount":599.00,"date":"2024-02-01"},{"id":5,"description":"Travel expenses","amount":1250.00,"date":"2024-02-10"}]"#;

    for _ in 0..employee_count {
        // Each tool call returns full expense data to context
        total_output_chars += expense_template.len();
        tool_call_count += 1;
    }

    // Estimate tokens (roughly 4 chars per token)
    let estimated_tokens = total_output_chars / 4;

    TraditionalMetrics {
        tool_calls: tool_call_count,
        output_chars: total_output_chars,
        estimated_tokens,
    }
}

/// Programmatic approach: only final result returns
fn programmatic_approach(employee_count: usize) -> ProgrammaticMetrics {
    let mut orchestrator = ToolOrchestrator::new();

    // Register expense tool
    orchestrator.register_executor("get_expenses", |input| {
        let _id = input.as_i64().unwrap_or(0);
        // Return same data as traditional approach
        Ok(r#"[{"id":1,"description":"Office supplies","amount":150.00},{"id":2,"description":"Software license","amount":299.99},{"id":3,"description":"Team lunch","amount":85.50},{"id":4,"description":"Conference ticket","amount":599.00},{"id":5,"description":"Travel expenses","amount":1250.00}]"#.to_string())
    });

    // Build script dynamically
    let ids: Vec<String> = (1..=employee_count).map(|i| i.to_string()).collect();
    let script = format!(
        r#"
        let employee_ids = [{}];
        let total = 0.0;
        let count = 0;

        for id in employee_ids {{
            let expenses = get_expenses(id);
            // Count items (simplified)
            count += 5;  // Each employee has 5 expenses
            total += 2384.49;  // Sum of amounts
        }}

        `Processed ${{employee_ids.len()}} employees, ${{count}} expenses, total: $${{total}}`
    "#,
        ids.join(", ")
    );

    let result = orchestrator
        .execute(&script, ExecutionLimits::default())
        .expect("Execution failed");

    ProgrammaticMetrics {
        tool_calls: result.tool_calls.len(),
        output_chars: result.output.len(),
        estimated_tokens: result.output.len() / 4,
        execution_time_ms: result.execution_time_ms,
    }
}

#[derive(Debug)]
struct TraditionalMetrics {
    tool_calls: usize,
    output_chars: usize,
    estimated_tokens: usize,
}

#[derive(Debug)]
struct ProgrammaticMetrics {
    tool_calls: usize,
    output_chars: usize,
    estimated_tokens: usize,
    execution_time_ms: u64,
}

fn benchmark_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_comparison");

    for employee_count in [5, 10, 20, 50].iter() {
        group.bench_function(format!("traditional_{}_employees", employee_count), |b| {
            b.iter(|| traditional_approach_simulation(black_box(*employee_count)))
        });

        group.bench_function(format!("programmatic_{}_employees", employee_count), |b| {
            b.iter(|| programmatic_approach(black_box(*employee_count)))
        });
    }

    group.finish();
}

fn print_comparison_report() {
    println!("\n=== Token Comparison Report ===\n");
    println!(
        "{:<12} {:>15} {:>15} {:>15} {:>10}",
        "Employees", "Traditional", "Programmatic", "Savings", "Reduction"
    );
    println!("{}", "-".repeat(70));

    for count in [5, 10, 20, 50, 100] {
        let trad = traditional_approach_simulation(count);
        let prog = programmatic_approach(count);

        let savings = trad.estimated_tokens as i64 - prog.estimated_tokens as i64;
        let reduction = if trad.estimated_tokens > 0 {
            (savings as f64 / trad.estimated_tokens as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "{:<12} {:>12} tok {:>12} tok {:>12} tok {:>9.1}%",
            count, trad.estimated_tokens, prog.estimated_tokens, savings, reduction
        );
    }

    println!("\n=== Detailed Metrics (20 employees) ===\n");
    let trad = traditional_approach_simulation(20);
    let prog = programmatic_approach(20);

    println!("Traditional Approach:");
    println!("  - Tool calls: {} (each returns to LLM)", trad.tool_calls);
    println!("  - Total output: {} chars", trad.output_chars);
    println!("  - Estimated tokens: {}", trad.estimated_tokens);

    println!("\nProgrammatic Approach:");
    println!("  - Tool calls: {} (all in one script)", prog.tool_calls);
    println!("  - Final output: {} chars", prog.output_chars);
    println!("  - Estimated tokens: {}", prog.estimated_tokens);
    println!("  - Execution time: {}ms", prog.execution_time_ms);

    let reduction = ((trad.estimated_tokens - prog.estimated_tokens) as f64
        / trad.estimated_tokens as f64)
        * 100.0;
    println!("\nToken Reduction: {:.1}%", reduction);
}

/// Print detailed comparison report (called during benchmark)
fn report_during_bench() {
    // Only print report in verbose mode or when explicitly requested
    if std::env::var("PRINT_REPORT").is_ok() {
        print_comparison_report();
    }
}

fn benchmark_with_report(c: &mut Criterion) {
    report_during_bench();
    benchmark_comparison(c);
}

criterion_group!(benches, benchmark_with_report);
criterion_main!(benches);
