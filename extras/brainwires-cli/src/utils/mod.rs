pub mod ambiguity_effectiveness;
pub mod brainwires_md;
pub mod checkpoint;
pub mod completion_detector;
pub mod context_builder;
pub mod conversation;
pub mod cost_tracker;
pub mod debug;
pub mod embeddings;
pub mod entity_extraction;
pub mod importance;
pub mod logger;
pub mod memory;
pub mod paths;

/// Test-only helpers. Keep truly shared state here so tests in different
/// modules can coordinate access to process-global resources (env vars,
/// CWD, etc.) without each rolling their own mutex.
#[cfg(test)]
pub mod test_util {
    use std::sync::Mutex;
    /// Serialise tests that set `BRAINWIRES_MEMORY_ROOT`. Env vars are
    /// process-global, and tokio's default test executor runs tests
    /// concurrently, so we must gate on this shared lock whenever a
    /// test wants deterministic behaviour from that env var.
    pub static ENV_LOCK: Mutex<()> = Mutex::new(());
}
/// Plan parser re-exported from framework core
pub mod plan_parser {
    pub use brainwires::core::plan_parser::*;
}
pub mod prompt_cache;
pub mod prompt_history;
pub mod question_instructions;
pub mod recovery;
pub mod retrieval_gate;
pub mod rich_output;
pub mod secret_redaction;
pub mod skills;
pub mod system_prompt;
pub mod tokenizer;
