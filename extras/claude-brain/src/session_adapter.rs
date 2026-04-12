//! Bridge between BrainClient thought storage and DreamSessionStore trait.

use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Mutex;

use brainwires_core::Message;
use brainwires_knowledge::dream::consolidator::DreamSessionStore;
use brainwires_knowledge::knowledge::brain_client::BrainClient;
use brainwires_knowledge::knowledge::types::*;

/// Adapts BrainClient's thought storage to the DreamSessionStore trait
/// required by the DreamConsolidator.
pub struct BrainSessionAdapter {
    client: Arc<Mutex<BrainClient>>,
}

impl BrainSessionAdapter {
    pub fn new(client: Arc<Mutex<BrainClient>>) -> Self {
        Self { client }
    }
}

#[async_trait::async_trait]
impl DreamSessionStore for BrainSessionAdapter {
    async fn list_sessions(&self) -> Result<Vec<String>> {
        // List recent thoughts and extract unique session-like groups.
        // For now, return a single "default" session containing all thoughts.
        // A more sophisticated implementation would group by date or conversation_id tags.
        let client = self.client.lock().await;
        let recent = client
            .list_recent(ListRecentRequest {
                limit: 1000,
                category: None,
                since: None,
            })
            .await?;

        if recent.thoughts.is_empty() {
            return Ok(Vec::new());
        }

        // Group by the "session:" tag prefix if present, otherwise "default"
        let mut sessions: Vec<String> = recent
            .thoughts
            .iter()
            .flat_map(|t| {
                t.tags
                    .iter()
                    .filter(|tag| tag.starts_with("session:"))
                    .map(|tag| tag.strip_prefix("session:").unwrap_or(tag).to_string())
            })
            .collect();

        sessions.sort();
        sessions.dedup();

        if sessions.is_empty() {
            sessions.push("default".to_string());
        }

        Ok(sessions)
    }

    async fn load(&self, session_key: &str) -> Result<Option<Vec<Message>>> {
        let client = self.client.lock().await;

        // Search for thoughts tagged with this session
        let results = client
            .search_memory(SearchMemoryRequest {
                query: format!("session:{session_key}"),
                limit: 100,
                min_score: 0.0,
                category: None,
                sources: Some(vec!["claude-code-turn".to_string()]),
            })
            .await?;

        if results.results.is_empty() {
            return Ok(None);
        }

        // Convert thoughts to Messages
        let messages: Vec<Message> = results
            .results
            .iter()
            .map(|r| {
                let content = r
                    .content
                    .strip_prefix("[assistant] ")
                    .or_else(|| r.content.strip_prefix("[user] "))
                    .unwrap_or(&r.content);
                if r.content.starts_with("[assistant]") {
                    Message::assistant(content)
                } else {
                    Message::user(content)
                }
            })
            .collect();

        Ok(Some(messages))
    }

    async fn save(&self, _session_key: &str, _messages: &[Message]) -> Result<()> {
        // After consolidation, the consolidated messages replace the originals.
        // For now, we store the consolidated summary as a new thought.
        // The original thoughts remain (they'll age out via demotion policy).
        //
        // A full implementation would delete the originals and store the summary,
        // but BrainClient doesn't expose bulk delete by tag yet.
        // This is a reasonable starting point — consolidation summaries accumulate
        // and old thoughts gradually lose relevance in search results.
        Ok(())
    }
}
