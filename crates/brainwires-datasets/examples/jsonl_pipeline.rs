//! Example: Training data pipeline with validation, format conversion, and splitting
//!
//! Demonstrates creating training data in-memory, validating it with
//! `DataValidator`, converting between formats (OpenAI, Alpaca, ShareGPT),
//! performing a train/eval split, and computing dataset statistics.
//!
//! Run: cargo run -p brainwires-datasets --example jsonl_pipeline

use brainwires_datasets::{
    AlpacaFormat, DataValidator, Dataset, FormatConverter, OpenAiFormat, ShareGptFormat,
    SplitConfig, TrainingExample, TrainingMessage, ValidatorConfig, compute_stats,
    train_eval_split,
};

fn main() {
    println!("=== JSONL Training Data Pipeline Example ===\n");

    // ── 1. Create sample training data in-memory ────────────────────────
    println!("--- Step 1: Create Training Examples ---");
    let examples = vec![
        TrainingExample::with_id(
            "ex-001",
            vec![
                TrainingMessage::system("You are a helpful coding assistant."),
                TrainingMessage::user("What is a closure in Rust?"),
                TrainingMessage::assistant(
                    "A closure in Rust is an anonymous function that can capture variables \
                     from its enclosing scope. Closures are defined using |args| body syntax \
                     and implement one of the Fn, FnMut, or FnOnce traits.",
                ),
            ],
        ),
        TrainingExample::with_id(
            "ex-002",
            vec![
                TrainingMessage::system("You are a helpful coding assistant."),
                TrainingMessage::user("How do I handle errors in Rust?"),
                TrainingMessage::assistant(
                    "Rust uses the Result<T, E> type for recoverable errors and panic! for \
                     unrecoverable ones. Use the ? operator to propagate errors, or match \
                     on Result variants for explicit handling.",
                ),
            ],
        ),
        TrainingExample::with_id(
            "ex-003",
            vec![
                TrainingMessage::user("Explain ownership in Rust."),
                TrainingMessage::assistant(
                    "Ownership is Rust's memory management system. Each value has exactly \
                     one owner, and the value is dropped when the owner goes out of scope. \
                     Ownership can be transferred (moved) or borrowed via references.",
                ),
            ],
        ),
        TrainingExample::with_id(
            "ex-004",
            vec![
                TrainingMessage::system("You are an expert Rust developer."),
                TrainingMessage::user("What are lifetimes?"),
                TrainingMessage::assistant(
                    "Lifetimes are Rust's way of ensuring that references are valid for as \
                     long as they are used. They are denoted with 'a syntax and tell the \
                     borrow checker how long references should live.",
                ),
            ],
        ),
        // Intentionally problematic example for validation demo
        TrainingExample::with_id("ex-005", vec![TrainingMessage::user("What is async?")]),
    ];
    println!("  Created {} training examples\n", examples.len());

    // ── 2. Validate the dataset ─────────────────────────────────────────
    println!("--- Step 2: Validate Dataset ---");
    let validator = DataValidator::new(ValidatorConfig {
        min_messages: 2,
        max_messages: 100,
        max_tokens: 4096,
        require_assistant_last: true,
        require_system_message: false,
        reject_empty_content: true,
        require_alternating_turns: true,
    });

    let report = validator.validate_dataset(&examples).unwrap();
    println!("  Total examples:  {}", report.total_examples);
    println!("  Valid examples:  {}", report.valid_examples);
    println!("  Errors:          {}", report.error_count());
    println!("  Warnings:        {}", report.warning_count());

    if report.has_errors() {
        println!("  Issues found:");
        for issue in &report.issues {
            println!(
                "    [{}] {}: {}",
                match issue.severity {
                    brainwires_datasets::IssueSeverity::Error => "ERROR",
                    brainwires_datasets::IssueSeverity::Warning => "WARN ",
                },
                issue.example_id,
                issue.message,
            );
        }
    }

    // Filter to only valid examples for the rest of the pipeline
    let valid_examples: Vec<_> = examples
        .iter()
        .filter(|ex| {
            validator
                .validate_example(ex)
                .iter()
                .all(|i| i.severity != brainwires_datasets::IssueSeverity::Error)
        })
        .cloned()
        .collect();
    println!(
        "  Proceeding with {} valid examples\n",
        valid_examples.len()
    );

    // ── 3. Convert between formats ──────────────────────────────────────
    println!("--- Step 3: Format Conversion ---");
    let example = &valid_examples[0];

    // OpenAI format
    let openai = OpenAiFormat;
    let openai_json = openai.to_json(example).unwrap();
    println!("  OpenAI format:");
    println!(
        "    {}\n",
        serde_json::to_string_pretty(&openai_json).unwrap()
    );

    // Alpaca format
    let alpaca = AlpacaFormat;
    let alpaca_json = alpaca.to_json(example).unwrap();
    println!("  Alpaca format:");
    println!(
        "    {}\n",
        serde_json::to_string_pretty(&alpaca_json).unwrap()
    );

    // ShareGPT format
    let sharegpt = ShareGptFormat;
    let sharegpt_json = sharegpt.to_json(example).unwrap();
    println!("  ShareGPT format:");
    println!(
        "    {}\n",
        serde_json::to_string_pretty(&sharegpt_json).unwrap()
    );

    // Round-trip: convert to OpenAI JSON and parse back
    let parsed_back = openai.parse_json(&openai_json).unwrap();
    println!(
        "  Round-trip OK: parsed back {} messages from OpenAI format",
        parsed_back.messages.len()
    );

    // Auto-detect format from JSON
    let detected = brainwires_datasets::detect_format(&openai_json);
    println!("  Auto-detected format: {:?}", detected);

    let detected_alpaca = brainwires_datasets::detect_format(&alpaca_json);
    println!("  Auto-detected Alpaca format: {:?}\n", detected_alpaca);

    // ── 4. Train/eval split ─────────────────────────────────────────────
    println!("--- Step 4: Train/Eval Split ---");
    let split_config = SplitConfig {
        train_ratio: 0.75,
        seed: 42,
        shuffle: true,
    };

    let split = train_eval_split(&valid_examples, &split_config);
    println!("  Train set: {} examples", split.train.len());
    println!("  Eval set:  {} examples\n", split.eval.len());

    // ── 5. Compute dataset statistics ───────────────────────────────────
    println!("--- Step 5: Dataset Statistics ---");
    let stats = compute_stats(&valid_examples);
    println!("  Total examples:           {}", stats.total_examples);
    println!("  Total messages:           {}", stats.total_messages);
    println!(
        "  Total estimated tokens:   {}",
        stats.total_estimated_tokens
    );
    println!(
        "  Avg messages/example:     {:.1}",
        stats.avg_messages_per_example
    );
    println!(
        "  Avg tokens/example:       {:.1}",
        stats.avg_tokens_per_example
    );
    println!(
        "  Token range:              {} - {}",
        stats.min_tokens, stats.max_tokens
    );
    println!(
        "  Examples with system msg:  {}",
        stats.examples_with_system
    );
    println!("  Role counts:");
    println!("    system:    {}", stats.role_counts.system);
    println!("    user:      {}", stats.role_counts.user);
    println!("    assistant: {}", stats.role_counts.assistant);
    println!("    tool:      {}", stats.role_counts.tool);

    if !stats.token_histogram.is_empty() {
        println!("  Token histogram:");
        for bucket in &stats.token_histogram {
            println!(
                "    [{:>5} - {:>5}): {} examples",
                bucket.range_start, bucket.range_end, bucket.count,
            );
        }
    }

    println!("\nDone! This pipeline can be extended with JSONL file I/O,");
    println!("deduplication, and tokenizer-based exact token counts.");
}
