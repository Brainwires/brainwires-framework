//! PreCompact hook — export conversation from transcript file to Brainwires before compaction.

use anyhow::Result;
use std::io::BufRead;

use crate::config::ClaudeBrainConfig;
use crate::context_manager::ContextManager;
use crate::hook_protocol::{self, PreCompactPayload};

/// Handle the PreCompact hook event.
///
/// Claude Code sends `transcript_path` pointing to the JSONL conversation file.
/// We read it, extract user/assistant messages, and store them in Brainwires
/// before compaction destroys the full context.
pub async fn handle() -> Result<()> {
    let payload: PreCompactPayload = hook_protocol::read_payload().await?;
    let config = ClaudeBrainConfig::load()?;
    let ctx = ContextManager::new(config).await?;

    let session_tag = payload
        .session_id
        .as_deref()
        .map(|id| format!("session:{id}"))
        .unwrap_or_else(|| "session:default".to_string());

    // Read messages from transcript file
    let messages = read_transcript_messages(payload.transcript_path.as_deref());
    let msg_count = messages.len();

    // Log
    let log_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".brainwires")
        .join("claude-brain-hooks.log");
    let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(std::path::Path::new("/tmp")));
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let log_line = format!(
        "[{timestamp}] PRE-COMPACT fired — {msg_count} messages from transcript (trigger={})\n",
        payload.trigger.as_deref().unwrap_or("?")
    );
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, log_line.as_bytes()));

    // Store each extracted message
    for (role, content) in &messages {
        if content.len() < 20 {
            continue;
        }

        let tagged_content = format!("[{role}] {content}");

        // Truncate very long messages for storage efficiency
        let store_content = if tagged_content.len() > 2000 {
            format!("{}...[truncated]", &tagged_content[..2000])
        } else {
            tagged_content
        };

        let mut client = ctx.client().lock_owned().await;
        let _ = client
            .capture_thought(
                brainwires_knowledge::knowledge::types::CaptureThoughtRequest {
                    content: store_content,
                    category: Some("conversation".to_string()),
                    tags: Some(vec![
                        "claude-code".to_string(),
                        "pre-compact".to_string(),
                        session_tag.clone(),
                    ]),
                    importance: Some(0.6),
                    source: Some("pre-compact-export".to_string()),
                },
            )
            .await;
    }

    tracing::info!(
        "Pre-compact: exported {} messages to Brainwires",
        msg_count
    );

    Ok(())
}

/// Read the JSONL transcript file and extract (role, content) pairs.
///
/// Each line is a JSON object. We look for objects with `role` and `content` fields
/// (the standard Claude API message format). Content can be a string or an array
/// of content blocks — we extract text from both.
fn read_transcript_messages(path: Option<&str>) -> Vec<(String, String)> {
    let Some(path) = path else {
        return Vec::new();
    };

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = std::io::BufReader::new(file);
    let mut messages = Vec::new();

    for line in reader.lines() {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }

        let Ok(obj) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };

        // Claude Code transcript format: messages nested under "message" key
        let msg_obj = if let Some(msg) = obj.get("message") {
            msg
        } else {
            &obj
        };

        let Some(role) = msg_obj.get("role").and_then(|v| v.as_str()) else {
            continue;
        };

        // Only capture user and assistant messages
        if role != "user" && role != "assistant" {
            continue;
        }

        let content = extract_text_content(msg_obj);
        if !content.is_empty() {
            messages.push((role.to_string(), content));
        }
    }

    messages
}

/// Extract text content from a message object.
/// Handles both `"content": "string"` and `"content": [{"type":"text","text":"..."}]`.
fn extract_text_content(msg: &serde_json::Value) -> String {
    let Some(content) = msg.get("content") else {
        return String::new();
    };

    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(blocks) => {
            let mut texts = Vec::new();
            for block in blocks {
                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                    texts.push(text);
                }
            }
            texts.join("\n")
        }
        _ => String::new(),
    }
}
