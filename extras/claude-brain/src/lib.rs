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
/// and `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` (percent threshold, e.g. 50 = 50%).
///
/// Target: keep post-compaction total at ≤ 70% of the compaction threshold so
/// many messages can accumulate before the next compaction fires.
///
///   threshold  = window_tokens × (pct / 100)
///   target     = threshold × 0.70
///   hook_share = target × 0.25   (system prompt + summary take the rest)
///   budget     = hook_share × 4  (chars, ~4 chars/token)
///
/// Floor: 2000 chars. Ceiling: 40000 chars.
pub fn compute_output_budget() -> usize {
    let window_tokens: usize = std::env::var("CLAUDE_CODE_AUTO_COMPACT_WINDOW")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);

    let compact_pct: f64 = std::env::var("CLAUDE_AUTOCOMPACT_PCT_OVERRIDE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50.0)
        / 100.0;

    let threshold = window_tokens as f64 * compact_pct;
    let target = threshold * 0.70; // 70% of trigger point
    let hook_share = target * 0.25; // hooks get 25%, rest is system prompt + summary
    let budget = (hook_share * 4.0) as usize; // tokens → chars
    budget.clamp(2_000, 40_000)
}
