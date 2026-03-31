//! Tool Registry demonstration.
//!
//! Shows how to create a `ToolRegistry` with built-in tools, register custom
//! tools, list tools by `ToolCategory`, and inspect tool metadata.
//!
//! Run:
//!   cargo run -p brainwires-tool-system --example tool_registry

use brainwires_tool_system::{Tool, ToolCategory, ToolInputSchema, ToolRegistry};
use std::collections::HashMap;

/// Helper: create a custom tool definition with the given name and description.
fn make_custom_tool(name: &str, description: &str) -> Tool {
    // Build a simple input schema with one required "input" property.
    let mut props = HashMap::new();
    props.insert(
        "input".to_string(),
        serde_json::json!({
            "type": "string",
            "description": "The input value"
        }),
    );

    Tool {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: ToolInputSchema::object(props, vec!["input".to_string()]),
        requires_approval: false,
        defer_loading: false,
        ..Default::default()
    }
}

fn main() {
    println!("=== Tool Registry Example ===\n");

    // ── 1. Create a registry pre-populated with all built-in tools ──────────
    let mut registry = ToolRegistry::with_builtins();
    println!("Registry created with {} built-in tool(s).", registry.len());

    // ── 2. Register custom tools ────────────────────────────────────────────
    let custom_tools = vec![
        make_custom_tool("translate_text", "Translate text between languages"),
        make_custom_tool("summarize", "Summarize a block of text"),
    ];

    registry.register_tools(custom_tools);
    println!("After adding custom tools: {} total.", registry.len());

    // Register a single tool that requires approval before execution.
    let sensitive_tool = Tool {
        name: "deploy_production".to_string(),
        description: "Deploy the application to production".to_string(),
        input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
        requires_approval: true,
        defer_loading: true,
        ..Default::default()
    };
    registry.register(sensitive_tool);
    println!("Registered 'deploy_production' (requires approval, deferred).\n");

    // ── 3. List tools by category ───────────────────────────────────────────
    let categories = [
        ("FileOps", ToolCategory::FileOps),
        ("Git", ToolCategory::Git),
        ("Search", ToolCategory::Search),
        ("Bash", ToolCategory::Bash),
        ("Web", ToolCategory::Web),
        ("Validation", ToolCategory::Validation),
    ];

    println!("Tools by category:");
    for (label, cat) in &categories {
        let tools = registry.get_by_category(*cat);
        if tools.is_empty() {
            println!("  {label}: (none registered)");
        } else {
            let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
            println!("  {label}: {}", names.join(", "));
        }
    }

    // ── 4. Look up a tool by name and inspect its metadata ──────────────────
    println!("\nTool metadata lookup:");
    if let Some(tool) = registry.get("translate_text") {
        println!("  Name:             {}", tool.name);
        println!("  Description:      {}", tool.description);
        println!("  Requires approval: {}", tool.requires_approval);
        println!("  Defer loading:    {}", tool.defer_loading);
        println!(
            "  Schema:           {}",
            serde_json::to_string_pretty(&tool.input_schema).unwrap()
        );
    }

    // ── 5. Search tools by keyword ──────────────────────────────────────────
    let query = "file";
    let results = registry.search_tools(query);
    println!("\nSearch for \"{query}\": {} result(s)", results.len());
    for tool in &results {
        println!("  - {} : {}", tool.name, tool.description);
    }

    // ── 6. Initial vs. deferred tools ───────────────────────────────────────
    let initial = registry.get_initial_tools();
    let deferred = registry.get_deferred_tools();
    println!(
        "\nInitial tools: {}, Deferred tools: {}",
        initial.len(),
        deferred.len()
    );

    // ── 7. Core tools subset ────────────────────────────────────────────────
    let core = registry.get_core();
    println!(
        "Core tools ({}): {}",
        core.len(),
        core.iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    println!("\nDone.");
}
