//! PostCompact hook — inject rich Brainwires context after compaction.

use anyhow::Result;

use crate::config::ClaudeBrainConfig;
use crate::context_manager::ContextManager;
use crate::hook_protocol::{self, PostCompactPayload};

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
    let log_line = format!("[{timestamp}] POST-COMPACT fired — summary {summary_len} chars\n");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, log_line.as_bytes()));

    let mut sections: Vec<String> = Vec::new();

    // If compaction produced a summary, use it as a search query to find related context
    let search_query = payload
        .compact_summary
        .as_deref()
        .unwrap_or("recent conversation context decisions");

    // Search for relevant facts from knowledge base
    let knowledge = ctx.search_knowledge(search_query, 15).await;
    if let Ok(resp) = knowledge {
        if !resp.results.is_empty() {
            let mut section = String::from("## Key Knowledge (from Brainwires)\n\n");
            for result in &resp.results {
                section.push_str(&format!("- {}: {}\n", result.key, result.value));
            }
            sections.push(section);
        }
    }

    // Search for relevant past conversation context
    let memory = ctx.search_memory(search_query, 10, 0.5).await;
    if let Ok(resp) = memory {
        if !resp.results.is_empty() {
            let mut section = String::from("## Recalled Context (from Brainwires)\n\n");
            for result in resp.results.iter().take(10) {
                let cat = result.category.as_deref().unwrap_or("general");
                section.push_str(&format!("- [{}] {}\n", cat, result.content));
            }
            sections.push(section);
        }
    }

    if !sections.is_empty() {
        let output = format!(
            "# Post-Compaction Context Restoration\n\n\
             The following context was restored from Brainwires persistent memory \
             after compaction. This supplements the compaction summary with durable \
             facts and prior conversation context.\n\n{}",
            sections.join("\n")
        );
        hook_protocol::write_output(&output);
    }

    Ok(())
}
