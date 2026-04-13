//! PostCompact hook — inject rich Brainwires context after compaction.

use anyhow::Result;

use crate::config::ClaudeBrainConfig;
use crate::context_manager::ContextManager;
use crate::hook_protocol::{self, PostCompactPayload};
use crate::sanitize_tag_value;

/// Handle the PostCompact hook event.
///
/// After Claude Code's compaction runs (only manual `/compact` since auto is disabled),
/// this hook injects rich consolidated context from Brainwires — facts, summaries,
/// and key decisions — so Claude doesn't lose critical information.
pub async fn handle() -> Result<()> {
    let payload: PostCompactPayload = hook_protocol::read_payload().await?;
    let config = ClaudeBrainConfig::load()?;
    let ctx = ContextManager::new(config).await?;

    // Log to file so we can verify the hook fired
    let log_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".brainwires")
        .join("claude-brain-hooks.log");
    let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(std::path::Path::new("/tmp")));
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let summary_len = payload.compact_summary.as_ref().map(|s| s.len()).unwrap_or(0);
    let budget = crate::compute_output_budget();
    let log_line = format!("[{timestamp}] POST-COMPACT fired — summary {summary_len} chars, budget {budget} chars\n");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, log_line.as_bytes()));

    let mut sections: Vec<String> = Vec::new();

    // First, look for a structured session digest created by PreCompact
    let session_tag = payload
        .session_id
        .as_deref()
        .map(|id| format!("session:{}", sanitize_tag_value(id)))
        .unwrap_or_else(|| "session:default".to_string());

    {
        use brainwires_storage::{Filter, FieldValue};
        let filter = Filter::And(vec![
            Filter::Eq("deleted".into(), FieldValue::Boolean(Some(false))),
            Filter::Raw("tags LIKE '%session-digest%'".to_string()),
            Filter::Raw(format!("tags LIKE '%{}%'", session_tag)),
        ]);
        let arc = ctx.client();
        let client = arc.lock().await;
        let digests = client.query_thought_contents(&filter, 1).await.unwrap_or_default();
        if let Some(digest) = digests.first() {
            let mut section = String::from("## Session Digest\n\n");
            section.push_str(digest);
            section.push('\n');
            sections.push(section);
        }
    }

    // Search query for supplemental context
    let search_query = payload
        .compact_summary
        .as_deref()
        .unwrap_or("recent conversation context decisions");

    // Search for relevant facts from knowledge base
    let knowledge = ctx.search_knowledge(search_query, 15).await;
    if let Ok(resp) = knowledge
        && !resp.results.is_empty() {
            let mut section = String::from("## Key Knowledge (from Brainwires)\n\n");
            for result in &resp.results {
                section.push_str(&format!("- {}: {}\n", result.key, result.value));
            }
            sections.push(section);
        }

    // Search for relevant past conversation context (only if no digest found)
    if sections.len() <= 1 {
        let memory = ctx.search_memory(search_query, 10, 0.3).await;
        if let Ok(resp) = memory
            && !resp.results.is_empty() {
                let mut section = String::from("## Recalled Context (from Brainwires)\n\n");
                for result in resp.results.iter().take(10) {
                    let cat = result.category.as_deref().unwrap_or("general");
                    section.push_str(&format!("- [{}] {}\n", cat, result.content));
                }
                sections.push(section);
            }
    }

    // IMPORTANT: Do NOT emit large context here. The compaction summary already
    // provides continuity. Emitting too much triggers a compaction loop because
    // system prompt + compaction summary + hook outputs exceed the threshold.
    //
    // Instead, emit a tiny reminder that Brainwires tools are available.
    // All context was already saved by PreCompact and can be retrieved on demand.
    let fact_count = sections.iter().filter(|s| s.contains("Knowledge")).count();
    let digest_found = sections.iter().any(|s| s.contains("Session Digest"));

    let mut hint = String::from("Context saved to Brainwires.");
    if digest_found {
        hint.push_str(" Session digest available.");
    }
    if fact_count > 0 {
        hint.push_str(" Knowledge facts indexed.");
    }
    hint.push_str(" Use recall_context/search_memory MCP tools to retrieve details.");

    hook_protocol::write_output(&hint);

    Ok(())
}
