//! Expense Aggregation Example
//!
//! This example demonstrates the core value proposition of Programmatic Tool Calling:
//! aggregating data from multiple sources without flooding the LLM context window.
//!
//! ## The Problem (Traditional Approach)
//!
//! With traditional tool calling, fetching expenses for 5 employees would require:
//! - 5 separate tool calls
//! - Each returning ~20 expense items to the context
//! - 100+ items polluting the context window
//! - ~15,000 tokens just for intermediate data
//!
//! ## The Solution (Programmatic Tool Calling)
//!
//! The LLM writes a Rhai script that:
//! - Loops through all employees
//! - Fetches and processes expenses locally
//! - Returns only the final summary
//! - Uses ~200 tokens total
//!
//! Run with: `cargo run --example expense_aggregation`

use tool_orchestrator::{ExecutionLimits, ToolOrchestrator};

fn main() {
    println!("=== Expense Aggregation Example ===\n");

    // Create the orchestrator
    let mut orchestrator = ToolOrchestrator::new();

    // Simulate an expense database
    // In a real app, this would be an API call
    orchestrator.register_executor("get_expenses", |input| {
        let employee_id = input.as_i64().unwrap_or(0);

        // Simulate expense data for each employee
        let expenses = match employee_id {
            1 => vec![
                ("Office supplies", 150.00),
                ("Software license", 299.99),
                ("Team lunch", 85.50),
                ("Conference ticket", 599.00),
            ],
            2 => vec![
                ("Travel - NYC", 1250.00),
                ("Hotel", 450.00),
                ("Client dinner", 180.00),
            ],
            3 => vec![
                ("Equipment", 899.00),
                ("Training course", 199.00),
            ],
            4 => vec![
                ("Marketing materials", 350.00),
                ("Advertising", 2500.00),
                ("Event sponsorship", 1000.00),
            ],
            5 => vec![
                ("Cloud services", 450.00),
                ("Domain renewal", 15.00),
            ],
            _ => vec![],
        };

        // Return as JSON string
        let json: Vec<String> = expenses
            .iter()
            .map(|(desc, amount)| format!(r#"{{"description":"{}","amount":{}}}"#, desc, amount))
            .collect();

        Ok(format!("[{}]", json.join(",")))
    });

    // Register a tool to get employee names
    orchestrator.register_executor("get_employee_name", |input| {
        let id = input.as_i64().unwrap_or(0);
        let name = match id {
            1 => "Alice",
            2 => "Bob",
            3 => "Carol",
            4 => "Dave",
            5 => "Eve",
            _ => "Unknown",
        };
        Ok(name.to_string())
    });

    // This is the script an LLM would generate
    // Notice how it processes all data locally and only returns the summary
    let script = r#"
        // Helper function to join array elements with a separator
        fn join_array(arr, sep) {
            let result = "";
            for i in 0..arr.len() {
                if i > 0 {
                    result += sep;
                }
                result += arr[i];
            }
            result
        }

        // Process expenses for all employees
        let employee_ids = [1, 2, 3, 4, 5];
        let total_expenses = 0.0;
        let expense_count = 0;
        let high_spenders = [];

        for id in employee_ids {
            let name = get_employee_name(id);
            let expenses_json = get_expenses(id);

            // Parse and sum expenses for this employee
            // (In real Rhai, we'd parse JSON - here we extract amounts)
            let employee_total = 0.0;
            let count = 0;

            // Simple parsing: count occurrences and extract amounts
            let parts = expenses_json.split("amount\":");
            for i in 1..parts.len() {
                let amount_parts = parts[i].split("}");
                if amount_parts.len() > 0 {
                    let amount_str = amount_parts[0];
                    let amount = amount_str.parse_float();
                    if amount != () {
                        employee_total += amount;
                        count += 1;
                    }
                }
            }

            total_expenses += employee_total;
            expense_count += count;

            // Track high spenders (>$1000)
            if employee_total > 1000.0 {
                high_spenders.push(`${name}: $${employee_total}`);
            }
        }

        // Build high spenders list
        let high_spenders_list = join_array(high_spenders, "\n  ");

        // Return only the summary - not all the raw data!
        `Expense Report Summary:
- Total employees processed: ${employee_ids.len()}
- Total expense items: ${expense_count}
- Total amount: $${total_expenses}
- High spenders (>$1000): ${high_spenders.len()}
  ${high_spenders_list}`
    "#;

    println!("Executing expense aggregation script...\n");

    let result = orchestrator
        .execute(script, ExecutionLimits::default())
        .expect("Script execution failed");

    println!("=== Result ===");
    println!("{}", result.output);
    println!("\n=== Metrics ===");
    println!("Success: {}", result.success);
    println!("Tool calls made: {}", result.tool_calls.len());
    println!("Execution time: {}ms", result.execution_time_ms);

    println!("\n=== Tool Call Details ===");
    for call in &result.tool_calls {
        println!(
            "  {} ({}) - {}ms",
            call.tool_name,
            if call.success { "ok" } else { "error" },
            call.duration_ms
        );
    }

    println!("\n=== Token Savings Analysis ===");
    println!("Traditional approach would have returned ~100 expense items to context");
    println!("PTC approach returned only the summary (~200 chars)");
    println!("Estimated token reduction: ~98%");
}
