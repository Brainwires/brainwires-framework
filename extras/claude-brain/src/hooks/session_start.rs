//! SessionStart hook — load relevant context from all memory tiers.

use anyhow::Result;

use crate::config::ClaudeBrainConfig;
use crate::context_manager::ContextManager;
use crate::hook_protocol::{self, SessionStartPayload};

/// Handle the SessionStart hook event.
pub async fn handle() -> Result<()> {
    let payload: SessionStartPayload = hook_protocol::read_payload().await?;
    // Log to file
    let log_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".brainwires")
        .join("claude-brain-hooks.log");
    let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(std::path::Path::new("/tmp")));
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let log_line = format!(
        "[{timestamp}] SESSION-START fired — cwd={} session={}\n",
        payload.cwd.as_deref().unwrap_or("?"),
        payload.session_id.as_deref().unwrap_or("?")
    );
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, log_line.as_bytes()));

    let config = ClaudeBrainConfig::load()?;
    let ctx = ContextManager::new(config).await?;

    let context = ctx
        .load_session_context(payload.cwd.as_deref(), payload.session_id.as_deref())
        .await?;

    if !context.is_empty() {
        hook_protocol::write_output(&context);
    }

    Ok(())
}
