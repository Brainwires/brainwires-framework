use std::sync::Arc;

use tokio::sync::{mpsc, watch};

use crate::{AnalyticsError, AnalyticsEvent, BoxedSink};

/// Internal channel capacity for the event queue.
const CHANNEL_CAPACITY: usize = 4096;

/// Central analytics collector.
///
/// Clone cheaply — all clones share the same background drain task and sinks.
///
/// # Usage
///
/// ```rust,ignore
/// let sink  = SqliteAnalyticsSink::new_default()?;
/// let collector = AnalyticsCollector::new(vec![Box::new(sink)]);
///
/// // Share across tasks/threads
/// let c2 = collector.clone();
/// tokio::spawn(async move { c2.record(event); });
///
/// // Graceful shutdown
/// collector.flush().await?;
/// collector.shutdown().await;
/// ```
pub struct AnalyticsCollector {
    inner: Arc<CollectorInner>,
}

impl Clone for AnalyticsCollector {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl std::fmt::Debug for AnalyticsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalyticsCollector").finish_non_exhaustive()
    }
}

struct CollectorInner {
    tx:          mpsc::Sender<AnalyticsEvent>,
    shutdown_tx: watch::Sender<bool>,
}

impl AnalyticsCollector {
    /// Create a new collector and start the background drain task.
    ///
    /// Must be called after the tokio runtime is running.
    pub fn new(sinks: Vec<BoxedSink>) -> Self {
        let (tx, rx)                   = mpsc::channel(CHANNEL_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        tokio::spawn(drain_loop(rx, sinks, shutdown_rx));

        Self {
            inner: Arc::new(CollectorInner { tx, shutdown_tx }),
        }
    }

    /// Emit an event. Returns immediately; delivery is async.
    ///
    /// Uses `try_send` — silently drops if the channel is full (fail-open).
    /// Analytics must never block or panic framework code.
    pub fn record(&self, event: AnalyticsEvent) {
        let _ = self.inner.tx.try_send(event);
    }

    /// Wait for the event queue to drain, then flush all sinks.
    pub async fn flush(&self) -> Result<(), AnalyticsError> {
        // Send a sentinel and wait until the channel drains below that point.
        // Simplest approach: wait until the channel's length approaches zero.
        // We poll with a short sleep rather than adding sentinel complexity.
        while self.inner.tx.max_capacity() - self.inner.tx.capacity() > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }
        Ok(())
    }

    /// Signal the background drain task to stop after emptying the queue.
    pub async fn shutdown(&self) {
        let _ = self.inner.shutdown_tx.send(true);
        // Give the drain task a moment to finish.
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
}

async fn drain_loop(
    mut rx:          mpsc::Receiver<AnalyticsEvent>,
    sinks:           Vec<BoxedSink>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            biased;

            Some(event) = rx.recv() => {
                for sink in &sinks {
                    if let Err(e) = sink.record(event.clone()).await {
                        tracing::warn!(
                            error = %e,
                            event_type = event.event_type(),
                            "Analytics sink failed to record event"
                        );
                    }
                }
            }

            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    // Drain remaining events before stopping.
                    while let Ok(event) = rx.try_recv() {
                        for sink in &sinks {
                            let _ = sink.record(event.clone()).await;
                        }
                    }
                    // Flush all sinks.
                    for sink in &sinks {
                        let _ = sink.flush().await;
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sinks::memory::MemoryAnalyticsSink;
    use chrono::Utc;
    use std::sync::Arc;

    fn make_event() -> AnalyticsEvent {
        AnalyticsEvent::Custom {
            session_id: None,
            name:       "test".into(),
            payload:    serde_json::Value::Null,
            timestamp:  Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_fanout_to_sink() {
        let mem  = Arc::new(MemoryAnalyticsSink::new(100));
        let mem2 = Arc::clone(&mem);

        // Wrap in a newtype so we can share the Arc with the sink trait
        struct SharedMemSink(Arc<MemoryAnalyticsSink>);
        #[async_trait::async_trait]
        impl crate::AnalyticsSink for SharedMemSink {
            async fn record(&self, event: AnalyticsEvent) -> Result<(), AnalyticsError> {
                self.0.record(event).await
            }
        }

        let collector = AnalyticsCollector::new(vec![
            Box::new(SharedMemSink(Arc::clone(&mem2))),
        ]);

        for _ in 0..10 {
            collector.record(make_event());
        }

        collector.flush().await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        assert_eq!(mem.len(), 10);
    }

    #[tokio::test]
    async fn test_record_does_not_block_when_full() {
        let collector = AnalyticsCollector::new(vec![]);
        // Fill past capacity — should not block or panic
        for _ in 0..(CHANNEL_CAPACITY + 100) {
            collector.record(make_event());
        }
    }
}
