use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::{AnalyticsError, AnalyticsEvent, AnalyticsSink};

/// In-memory ring-buffer analytics sink.
///
/// Stores up to `capacity` events, evicting the oldest when full.
/// Useful for testing and embedded scenarios where persistence is not needed.
pub struct MemoryAnalyticsSink {
    capacity: usize,
    events:   Mutex<VecDeque<AnalyticsEvent>>,
}

impl MemoryAnalyticsSink {
    /// Create a new sink with the given maximum capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            events: Mutex::new(VecDeque::with_capacity(capacity)),
        }
    }

    /// Drain and return all buffered events, clearing the buffer.
    pub fn drain(&self) -> Vec<AnalyticsEvent> {
        let mut events = self.events.lock().expect("MemoryAnalyticsSink lock poisoned");
        events.drain(..).collect()
    }

    /// Number of events currently in the buffer.
    pub fn len(&self) -> usize {
        self.events.lock().expect("MemoryAnalyticsSink lock poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Peek at a snapshot of all buffered events (cloned).
    pub fn snapshot(&self) -> Vec<AnalyticsEvent> {
        self.events.lock().expect("MemoryAnalyticsSink lock poisoned")
            .iter()
            .cloned()
            .collect()
    }
}

#[async_trait]
impl AnalyticsSink for MemoryAnalyticsSink {
    async fn record(&self, event: AnalyticsEvent) -> Result<(), AnalyticsError> {
        let mut events = self.events.lock().expect("MemoryAnalyticsSink lock poisoned");
        if events.len() >= self.capacity {
            events.pop_front();
        }
        events.push_back(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_event(name: &str) -> AnalyticsEvent {
        AnalyticsEvent::Custom {
            session_id: None,
            name: name.to_string(),
            payload: serde_json::Value::Null,
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_ring_buffer_capacity() {
        let sink = MemoryAnalyticsSink::new(3);
        for i in 0..5 {
            sink.record(make_event(&format!("e{i}"))).await.unwrap();
        }
        assert_eq!(sink.len(), 3);
        let events = sink.drain();
        // Oldest 2 should be evicted; only e2, e3, e4 remain
        assert!(matches!(&events[0], AnalyticsEvent::Custom { name, .. } if name == "e2"));
        assert!(matches!(&events[2], AnalyticsEvent::Custom { name, .. } if name == "e4"));
    }

    #[tokio::test]
    async fn test_drain_clears_buffer() {
        let sink = MemoryAnalyticsSink::new(10);
        sink.record(make_event("a")).await.unwrap();
        sink.record(make_event("b")).await.unwrap();
        assert_eq!(sink.len(), 2);
        let drained = sink.drain();
        assert_eq!(drained.len(), 2);
        assert!(sink.is_empty());
    }
}
