//! Core context orchestration — wraps BrainClient and tiered stores.

use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::sync::Mutex;

use brainwires_knowledge::knowledge::brain_client::BrainClient;
use brainwires_knowledge::knowledge::types::*;

use crate::config::ClaudeBrainConfig;

/// Central context manager wrapping all Brainwires storage tiers.
pub struct ContextManager {
    client: Arc<Mutex<BrainClient>>,
    config: ClaudeBrainConfig,
}

impl ContextManager {
    /// Create a new ContextManager with default storage paths.
    pub async fn new(config: ClaudeBrainConfig) -> Result<Self> {
        let client = BrainClient::with_paths(
            &config.storage.brain_path,
            &config.storage.pks_path,
            &config.storage.bks_path,
        )
        .await
        .context("Failed to create BrainClient")?;

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
            config,
        })
    }

    /// Get a clone of the Arc<Mutex<BrainClient>> for sharing.
    pub fn client(&self) -> Arc<Mutex<BrainClient>> {
        self.client.clone()
    }

    /// Get the configuration.
    pub fn config(&self) -> &ClaudeBrainConfig {
        &self.config
    }

    /// Load relevant context for a session start.
    ///
    /// Queries knowledge base for facts relevant to the working directory
    /// and recent thoughts.
    pub async fn load_session_context(
        &self,
        cwd: Option<&str>,
        _session_id: Option<&str>,
    ) -> Result<String> {
        let client = self.client.lock().await;
        let mut sections: Vec<String> = Vec::new();

        // Search knowledge base for project-relevant facts
        if let Some(dir) = cwd {
            // Extract project name from cwd
            let project_name = std::path::Path::new(dir)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(dir);

            let knowledge_results = client.search_knowledge(SearchKnowledgeRequest {
                query: project_name.to_string(),
                limit: self.config.session_start.max_facts,
                min_confidence: 0.5,
                source: None,
                category: None,
            });

            if let Ok(resp) = knowledge_results {
                if !resp.results.is_empty() {
                    let mut facts_section = String::from("## Relevant Knowledge\n\n");
                    for result in &resp.results {
                        facts_section
                            .push_str(&format!("- {}: {}\n", result.key, result.value));
                    }
                    sections.push(facts_section);
                }
            }
        }

        // Load recent thoughts
        let recent = client
            .list_recent(ListRecentRequest {
                limit: self.config.session_start.max_summaries,
                category: None,
                since: None,
            })
            .await;

        if let Ok(resp) = recent {
            if !resp.thoughts.is_empty() {
                let mut recent_section = String::from("## Recent Context\n\n");
                for thought in &resp.thoughts {
                    // Truncate long content for context preview
                    let preview = if thought.content.len() > 200 {
                        format!("{}...", &thought.content[..200])
                    } else {
                        thought.content.clone()
                    };
                    recent_section.push_str(&format!(
                        "- [{}] {}\n",
                        thought.category, preview
                    ));
                }
                sections.push(recent_section);
            }
        }

        if sections.is_empty() {
            Ok(String::new())
        } else {
            Ok(format!(
                "# Claude Brain — Session Context\n\n{}",
                sections.join("\n")
            ))
        }
    }

    /// Capture a conversation turn into hot-tier storage.
    pub async fn capture_turn(
        &self,
        content: &str,
        source: &str,
    ) -> Result<CaptureThoughtResponse> {
        let mut client = self.client.lock().await;
        client
            .capture_thought(CaptureThoughtRequest {
                content: content.to_string(),
                category: Some("conversation".to_string()),
                tags: Some(vec!["claude-code".to_string(), "auto-capture".to_string()]),
                importance: Some(0.5),
                source: Some(source.to_string()),
            })
            .await
    }

    /// Search memory across all tiers.
    pub async fn search_memory(
        &self,
        query: &str,
        limit: usize,
        min_score: f32,
    ) -> Result<SearchMemoryResponse> {
        let client = self.client.lock().await;
        client
            .search_memory(SearchMemoryRequest {
                query: query.to_string(),
                limit,
                min_score,
                category: None,
                sources: None,
            })
            .await
    }

    /// Search the PKS/BKS knowledge base.
    pub async fn search_knowledge(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<SearchKnowledgeResponse> {
        let client = self.client.lock().await;
        client.search_knowledge(SearchKnowledgeRequest {
            query: query.to_string(),
            limit,
            min_confidence: 0.5,
            source: None,
            category: None,
        })
    }

    /// Get memory statistics.
    pub async fn memory_stats(&self) -> Result<MemoryStatsResponse> {
        let client = self.client.lock().await;
        client.memory_stats().await
    }
}
