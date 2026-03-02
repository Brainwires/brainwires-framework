//! Example: Open Brain MCP Server
//!
//! Demonstrates both library and MCP server usage.
//!
//! Run with:
//!   cargo run --example mcp_server -p brainwires-brain

use brainwires_brain::{BrainClient, CaptureThoughtRequest, SearchMemoryRequest};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("=== Open Brain MCP Server Example ===\n");

    // Create a BrainClient with default paths
    let mut client = BrainClient::new().await?;
    println!("BrainClient initialized successfully.\n");

    // Capture a thought
    let response = client
        .capture_thought(CaptureThoughtRequest {
            content: "Decided to use LanceDB for the vector storage layer".into(),
            category: None, // Auto-detect
            tags: Some(vec!["architecture".into(), "storage".into()]),
            importance: Some(0.8),
            source: None,
        })
        .await?;

    println!("Captured thought:");
    println!("  ID:       {}", response.id);
    println!("  Category: {}", response.category);
    println!("  Tags:     {:?}", response.tags);
    println!("  Facts:    {} extracted", response.facts_extracted);
    println!();

    // Search memory
    let results = client
        .search_memory(SearchMemoryRequest {
            query: "database storage".into(),
            limit: 5,
            min_score: 0.0,
            category: None,
            sources: None,
        })
        .await?;

    println!("Search results for 'database storage':");
    for (i, r) in results.results.iter().enumerate() {
        println!(
            "  {}. [score={:.2}] [{}] {}",
            i + 1,
            r.score,
            r.source,
            &r.content[..r.content.len().min(80)]
        );
    }
    println!();

    // Show stats
    let stats = client.memory_stats().await?;
    println!("Memory stats:");
    println!("  Thoughts:   {}", stats.thoughts.total);
    println!("  PKS facts:  {}", stats.pks.total_facts);
    println!("  BKS truths: {}", stats.bks.total_truths);
    println!();

    println!("Available MCP Tools:");
    println!("  1. capture_thought  — Store a thought");
    println!("  2. search_memory    — Semantic search");
    println!("  3. list_recent      — Browse recent thoughts");
    println!("  4. get_thought      — Retrieve by ID");
    println!("  5. search_knowledge — Query PKS/BKS");
    println!("  6. memory_stats     — Statistics dashboard");
    println!("  7. delete_thought   — Remove a thought");
    println!();

    println!("MCP Prompts (slash commands):");
    println!("  /brain:capture   — Capture a new thought");
    println!("  /brain:search    — Semantic search");
    println!("  /brain:recent    — List recent thoughts");
    println!("  /brain:stats     — Show stats");
    println!("  /brain:knowledge — Search PKS/BKS");
    println!();

    println!("To run as MCP server: cargo run -p brainwires-brain");

    Ok(())
}
