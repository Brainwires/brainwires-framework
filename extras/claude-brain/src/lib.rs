pub mod config;
pub mod context_manager;
pub mod hook_protocol;
pub mod hooks;
pub mod mcp_server;
pub mod session_adapter;

/// Sanitize a value used inside `Filter::Raw` SQL-like expressions.
/// Strips everything except alphanumeric, hyphen, and underscore.
pub fn sanitize_tag_value(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// Compute a safe character budget for hook output based on Claude Code's
/// compaction window settings. Reads `CLAUDE_CODE_AUTO_COMPACT_WINDOW` (tokens)
/// and `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` (percent threshold). Returns the max
/// chars the hook should emit to avoid immediately re-triggering compaction.
///
/// Heuristic: use 1% of the total window (in chars) as the hook output budget.
/// At ~4 chars/token, this must be small enough that PostCompact + SessionStart +
/// compaction summary + system prompt don't re-trigger compaction.
///
/// The system prompt (instructions, CLAUDE.md, MCP tool schemas) can consume
/// 30-40% of the window alone. After compaction, we need ALL hook outputs
/// combined to fit in the remaining headroom. 1% per hook is conservative.
///
/// Floor: 2000 chars. Ceiling: 8000 chars.
pub fn compute_output_budget() -> usize {
    let window_tokens: usize = std::env::var("CLAUDE_CODE_AUTO_COMPACT_WINDOW")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1_000_000);

    // 1% of window in chars (~4 chars per token)
    let budget = (window_tokens as f64 * 4.0 * 0.01) as usize;
    budget.clamp(2_000, 8_000)
}
