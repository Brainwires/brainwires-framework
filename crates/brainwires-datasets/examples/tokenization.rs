//! Example: Tokenizer comparison — HuggingFace vs Tiktoken
//!
//! Demonstrates `TiktokenTokenizer` (OpenAI's BPE tokenizer) for token
//! counting and encoding/decoding. Also shows how `HfTokenizer` would be
//! used if a local tokenizer JSON file is available.
//!
//! Run: cargo run -p brainwires-datasets --example tokenization --features "hf-tokenizer,tiktoken"

use brainwires_datasets::{
    TiktokenTokenizer, Tokenizer, TrainingExample, TrainingMessage, compute_stats,
};

fn main() {
    println!("=== Tokenization Example ===\n");

    // ── 1. Create sample training data ──────────────────────────────────
    let examples = vec![
        TrainingExample::with_id(
            "ex-001",
            vec![
                TrainingMessage::system("You are a helpful AI assistant."),
                TrainingMessage::user("Explain the Rust borrow checker."),
                TrainingMessage::assistant(
                    "The borrow checker is Rust's compile-time system that enforces ownership \
                     rules. It ensures that references to data are always valid and prevents \
                     data races at compile time. The key rules are: each value has one owner, \
                     you can have either one mutable reference or many immutable references, \
                     and references must always be valid.",
                ),
            ],
        ),
        TrainingExample::with_id(
            "ex-002",
            vec![
                TrainingMessage::user("What is pattern matching in Rust?"),
                TrainingMessage::assistant(
                    "Pattern matching in Rust uses the `match` expression and `if let` syntax \
                     to destructure and compare values against patterns. It is exhaustive, \
                     meaning the compiler ensures all possible cases are handled.",
                ),
            ],
        ),
    ];

    // ── 2. Estimated token counts (built-in heuristic) ──────────────────
    println!("--- Estimated Tokens (built-in ~4 chars/token heuristic) ---");
    let stats = compute_stats(&examples);
    println!("  Total estimated tokens: {}", stats.total_estimated_tokens);
    println!(
        "  Avg tokens/example:    {:.1}",
        stats.avg_tokens_per_example
    );
    println!();

    // ── 3. Tiktoken tokenizer (cl100k_base — GPT-4/GPT-3.5) ────────────
    println!("--- Tiktoken cl100k_base (GPT-4 / GPT-3.5-turbo) ---");
    let tiktoken = TiktokenTokenizer::cl100k_base().expect("Failed to load cl100k_base");
    println!("  Vocab size: {}", tiktoken.vocab_size());

    let sample_texts = [
        "Hello, world!",
        "The borrow checker enforces ownership rules at compile time.",
        "fn main() { println!(\"Hello\"); }",
    ];

    for text in &sample_texts {
        let tokens = tiktoken.encode(text).expect("encoding failed");
        let count = tokens.len();
        println!("  \"{}\"", text);
        println!("    -> {} tokens: {:?}", count, &tokens[..count.min(10)]);

        // Round-trip decode
        let decoded = tiktoken.decode(&tokens).expect("decoding failed");
        println!("    -> decoded: \"{}\"", decoded);
        println!();
    }

    // ── 4. Tiktoken o200k_base (GPT-4o) ────────────────────────────────
    println!("--- Tiktoken o200k_base (GPT-4o) ---");
    let tiktoken_4o = TiktokenTokenizer::o200k_base().expect("Failed to load o200k_base");
    println!("  Vocab size: {}", tiktoken_4o.vocab_size());

    let text = "Pattern matching in Rust uses the match expression.";
    let cl100k_tokens = tiktoken.count_tokens(text).expect("count failed");
    let o200k_tokens = tiktoken_4o.count_tokens(text).expect("count failed");
    println!("  Text: \"{}\"", text);
    println!("    cl100k_base: {} tokens", cl100k_tokens);
    println!("    o200k_base:  {} tokens", o200k_tokens);
    println!();

    // ── 5. Batch encoding ──────────────────────────────────────────────
    println!("--- Batch Encoding ---");
    let batch: Vec<&str> = sample_texts.to_vec();
    let batch_results = tiktoken
        .encode_batch(&batch)
        .expect("batch encoding failed");
    for (text, tokens) in batch.iter().zip(batch_results.iter()) {
        println!("  \"{}\" -> {} tokens", text, tokens.len());
    }
    println!();

    // ── 6. Exact token counts for training examples ─────────────────────
    println!("--- Exact Token Counts per Training Example ---");
    for example in &examples {
        let mut total_tokens = 0;
        for msg in &example.messages {
            let count = tiktoken.count_tokens(&msg.content).expect("count failed");
            total_tokens += count;
        }
        let estimated = example.estimated_tokens();
        println!(
            "  {} | estimated: {:>3} | tiktoken: {:>3} | diff: {:+}",
            example.id,
            estimated,
            total_tokens,
            total_tokens as i64 - estimated as i64,
        );
    }
    println!();

    // ── 7. Special tokens ──────────────────────────────────────────────
    println!("--- Special Tokens ---");
    let special = tiktoken.special_tokens();
    for (name, id) in &special {
        println!("  {} -> {}", name, id);
    }
    println!();

    // ── 8. HfTokenizer usage pattern (requires a local tokenizer.json) ──
    println!("--- HuggingFace Tokenizer (usage pattern) ---");
    println!("  HfTokenizer::from_file(\"path/to/tokenizer.json\") loads a local");
    println!("  HuggingFace tokenizer file. It implements the same Tokenizer trait,");
    println!("  so you can swap it in anywhere TiktokenTokenizer is used.");
    println!("  Example:");
    println!("    let hf = HfTokenizer::from_file(\"tokenizer.json\")?;");
    println!("    let tokens = hf.encode(\"Hello, world!\")?;");
    println!("    let decoded = hf.decode(&tokens)?;");

    println!("\nDone! Use exact token counts from tiktoken or HuggingFace");
    println!("tokenizers to validate training data against provider limits.");
}
