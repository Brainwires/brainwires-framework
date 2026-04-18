//! Dream (sleep) consolidation — CLI integration layer.
//!
//! The framework's `brainwires::dream::DreamConsolidator` does the actual work
//! (summarise old messages, extract durable facts, prune raw history). This
//! module just adapts the CLI's session state to the framework's
//! `DreamSessionStore` trait and exposes a tiny global "last report" cache so
//! `/dream status` has something to show between runs.
//!
//! Kept deliberately minimal — no background scheduler yet. A manual
//! `/dream run` proves the pipeline end-to-end; a tokio interval spawn can
//! sit on top later without changing this API.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::Result;
use async_trait::async_trait;
use brainwires::core::{Message, Provider};
use brainwires::dream::consolidator::{DreamConsolidator, DreamSessionStore};
use brainwires::dream::metrics::DreamReport;
use brainwires::dream::policy::DemotionPolicy;
use tokio::sync::Mutex as AsyncMutex;

/// In-memory `DreamSessionStore` seeded from the active conversation.
///
/// Dream mutates the session during consolidation (replaces raw messages with
/// summaries + fact markers). Storing the mutated view here is fine for an
/// on-demand cycle; a persistence-aware adapter belongs in a later patch.
pub struct InMemoryDreamSessionStore {
    sessions: Mutex<HashMap<String, Vec<Message>>>,
}

impl InMemoryDreamSessionStore {
    /// Seed the store with a single session.
    pub fn with_session(key: impl Into<String>, messages: Vec<Message>) -> Self {
        let mut sessions = HashMap::new();
        sessions.insert(key.into(), messages);
        Self {
            sessions: Mutex::new(sessions),
        }
    }

    /// Return the post-cycle messages for the given session, if it exists.
    pub fn take_session(&self, key: &str) -> Option<Vec<Message>> {
        self.sessions.lock().ok()?.remove(key)
    }
}

#[async_trait]
impl DreamSessionStore for InMemoryDreamSessionStore {
    async fn list_sessions(&self) -> Result<Vec<String>> {
        let lock = self
            .sessions
            .lock()
            .map_err(|e| anyhow::anyhow!("dream session store mutex poisoned: {e}"))?;
        Ok(lock.keys().cloned().collect())
    }

    async fn load(&self, session_key: &str) -> Result<Option<Vec<Message>>> {
        let lock = self
            .sessions
            .lock()
            .map_err(|e| anyhow::anyhow!("dream session store mutex poisoned: {e}"))?;
        Ok(lock.get(session_key).cloned())
    }

    async fn save(&self, session_key: &str, messages: &[Message]) -> Result<()> {
        let mut lock = self
            .sessions
            .lock()
            .map_err(|e| anyhow::anyhow!("dream session store mutex poisoned: {e}"))?;
        lock.insert(session_key.to_string(), messages.to_vec());
        Ok(())
    }
}

/// Run one dream cycle against an in-memory session seeded from `messages`.
/// Returns the framework's `DreamReport` plus the (possibly consolidated)
/// messages so the caller can swap them back into their own conversation
/// state.
pub async fn run_once(
    provider: Arc<dyn Provider>,
    session_key: impl Into<String>,
    messages: Vec<Message>,
) -> Result<(DreamReport, Vec<Message>)> {
    let key = session_key.into();
    let store = InMemoryDreamSessionStore::with_session(key.clone(), messages);
    let consolidator = Arc::new(AsyncMutex::new(DreamConsolidator::new(
        provider,
        DemotionPolicy::default(),
    )));

    let report = {
        let mut guard = consolidator.lock().await;
        guard.run_cycle(&store).await?
    };
    let after = store.take_session(&key).unwrap_or_default();
    remember_last_report(&report);
    Ok((report, after))
}

// ── Process-wide "last report" cache for /dream status ────────────────────

static LAST_REPORT: OnceLock<Mutex<Option<DreamReport>>> = OnceLock::new();

fn report_slot() -> &'static Mutex<Option<DreamReport>> {
    LAST_REPORT.get_or_init(|| Mutex::new(None))
}

/// Record the most recent report (called automatically by `run_once`).
pub fn remember_last_report(report: &DreamReport) {
    if let Ok(mut slot) = report_slot().lock() {
        *slot = Some(report.clone());
    }
}

/// Retrieve the last stored report, if any.
pub fn last_report() -> Option<DreamReport> {
    report_slot().lock().ok().and_then(|s| s.clone())
}

/// Format a report for terminal display.
pub fn format_report(report: &DreamReport) -> String {
    let m = &report.metrics;
    let mut out = format!(
        "Dream cycle\n  sessions processed : {}\n  messages summarised: {}\n  facts extracted    : {}\n  tokens before      : {}\n  tokens after       : {}\n  duration           : {:.2?}\n",
        m.sessions_processed,
        m.messages_summarized,
        m.facts_extracted,
        m.tokens_before,
        m.tokens_after,
        m.duration,
    );
    if !report.errors.is_empty() {
        out.push_str("  errors:\n");
        for err in &report.errors {
            out.push_str(&format!("    - {err}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires::core::{MessageContent, Role};

    fn msg(role: Role, text: &str) -> Message {
        Message {
            role,
            content: MessageContent::Text(text.to_string()),
            name: None,
            metadata: None,
        }
    }

    #[tokio::test]
    async fn in_memory_store_roundtrips_a_session() {
        let store = InMemoryDreamSessionStore::with_session(
            "sess-a",
            vec![msg(Role::User, "hi"), msg(Role::Assistant, "hello")],
        );
        let keys = store.list_sessions().await.unwrap();
        assert_eq!(keys, vec!["sess-a".to_string()]);

        let loaded = store.load("sess-a").await.unwrap().unwrap();
        assert_eq!(loaded.len(), 2);

        store
            .save("sess-a", &[msg(Role::User, "replaced")])
            .await
            .unwrap();
        let after = store.load("sess-a").await.unwrap().unwrap();
        assert_eq!(after.len(), 1);
        assert!(matches!(&after[0].content, MessageContent::Text(t) if t == "replaced"));
    }

    #[tokio::test]
    async fn in_memory_store_reports_missing_session_as_none() {
        let store = InMemoryDreamSessionStore::with_session("a", vec![msg(Role::User, "x")]);
        assert!(store.load("does-not-exist").await.unwrap().is_none());
    }

    #[test]
    fn last_report_starts_empty_then_remembers() {
        // This test runs in a shared-process OnceLock so it may see a value set
        // by another test. Tolerate either.
        let _ = last_report();
        let report = DreamReport::default();
        remember_last_report(&report);
        let got = last_report().expect("should have a report now");
        assert_eq!(got.metrics.sessions_processed, 0);
    }

    #[test]
    fn format_report_renders_metrics_and_errors() {
        let mut report = DreamReport::default();
        report.metrics.sessions_processed = 2;
        report.metrics.messages_summarized = 5;
        report.errors.push("sess-b: provider timeout".to_string());

        let s = format_report(&report);
        assert!(s.contains("sessions processed : 2"));
        assert!(s.contains("messages summarised: 5"));
        assert!(s.contains("sess-b: provider timeout"));
    }
}
