//! Integration tests for `AnomalyDetector` threshold + sliding-window logic.
//!
//! Anomaly detection is a monitoring-only surface: a false negative means a
//! burst of suspicious behaviour goes unflagged. These tests exercise:
//!
//! - the exact boundary at which a threshold fires,
//! - that events outside the sliding window are forgotten,
//! - that the `expected_path_prefixes` allowlist flags out-of-scope targets.
//!
//! Events are fabricated with explicit timestamps so window aging is
//! deterministic — no `sleep` involved.

use brainwires_permissions::anomaly::{AnomalyConfig, AnomalyDetector, AnomalyKind};
use brainwires_permissions::audit::{ActionOutcome, AuditEvent, AuditEventType};
use chrono::{DateTime, TimeZone, Utc};

fn at(epoch_secs: i64, kind: AuditEventType, agent: &str) -> AuditEvent {
    let mut ev = AuditEvent::new(kind).with_agent(agent).with_outcome(ActionOutcome::Success);
    ev.timestamp = Utc.timestamp_opt(epoch_secs, 0).single().expect("valid epoch");
    ev
}

fn base_ts() -> DateTime<Utc> {
    // 2026-01-01T00:00:00Z — chosen arbitrarily; all test events pivot off it.
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).single().unwrap()
}

// ── Policy-violation window ──────────────────────────────────────────────

#[test]
fn violation_below_threshold_does_not_fire() {
    let cfg = AnomalyConfig {
        violation_threshold: 3,
        violation_window_secs: 60,
        ..Default::default()
    };
    let det = AnomalyDetector::new(cfg);
    let t = base_ts().timestamp();

    for i in 0..2 {
        det.observe(&at(t + i, AuditEventType::PolicyViolation, "a1"));
    }
    assert_eq!(det.pending_count(), 0);
    assert!(det.drain_anomalies().is_empty());
}

#[test]
fn violation_at_threshold_fires_and_keeps_firing_until_window_clears() {
    let cfg = AnomalyConfig {
        violation_threshold: 3,
        violation_window_secs: 60,
        ..Default::default()
    };
    let det = AnomalyDetector::new(cfg);
    let t = base_ts().timestamp();

    // Events 1 and 2: no fire. Event 3: hits threshold → fire. Event 4: still
    // inside window (count=4) → fire again. Event 5: count=5 → fire again.
    for i in 0..5 {
        det.observe(&at(t + i, AuditEventType::PolicyViolation, "a1"));
    }
    let anomalies = det.drain_anomalies();
    assert_eq!(anomalies.len(), 3, "events 3/4/5 should each emit");
    for a in &anomalies {
        assert!(
            matches!(a.kind, AnomalyKind::RepeatedPolicyViolation { .. }),
            "expected RepeatedPolicyViolation, got {:?}",
            a.kind,
        );
        assert_eq!(a.agent_id.as_deref(), Some("a1"));
    }
}

#[test]
fn violation_events_outside_window_are_forgotten() {
    let cfg = AnomalyConfig {
        violation_threshold: 3,
        violation_window_secs: 10, // short window
        ..Default::default()
    };
    let det = AnomalyDetector::new(cfg);
    let t = base_ts().timestamp();

    // Two violations, then a long gap, then two more. Second burst must not
    // see the first — only 2 events in the window, no anomaly.
    det.observe(&at(t, AuditEventType::PolicyViolation, "a1"));
    det.observe(&at(t + 1, AuditEventType::PolicyViolation, "a1"));
    det.observe(&at(t + 1000, AuditEventType::PolicyViolation, "a1"));
    det.observe(&at(t + 1001, AuditEventType::PolicyViolation, "a1"));
    assert_eq!(det.pending_count(), 0);
}

#[test]
fn violations_are_counted_per_agent() {
    let cfg = AnomalyConfig {
        violation_threshold: 3,
        violation_window_secs: 60,
        ..Default::default()
    };
    let det = AnomalyDetector::new(cfg);
    let t = base_ts().timestamp();

    // Two agents each at 2 violations — neither should cross the threshold
    // individually, even though the total is 4.
    for i in 0..2 {
        det.observe(&at(t + i, AuditEventType::PolicyViolation, "alice"));
        det.observe(&at(t + i, AuditEventType::PolicyViolation, "bob"));
    }
    assert_eq!(det.pending_count(), 0);
}

// ── Tool-call rate window ────────────────────────────────────────────────

#[test]
fn tool_call_rate_fires_only_once_window_threshold_reached() {
    let cfg = AnomalyConfig {
        tool_call_threshold: 5,
        tool_call_window_secs: 10,
        ..Default::default()
    };
    let det = AnomalyDetector::new(cfg);
    let t = base_ts().timestamp();

    for i in 0..4 {
        det.observe(&at(t + i, AuditEventType::ToolExecution, "a1"));
    }
    assert_eq!(det.pending_count(), 0);

    det.observe(&at(t + 4, AuditEventType::ToolExecution, "a1"));
    let anomalies = det.drain_anomalies();
    assert_eq!(anomalies.len(), 1);
    assert!(matches!(
        anomalies[0].kind,
        AnomalyKind::HighFrequencyToolCalls { .. }
    ));
}

// ── Path-scope check ─────────────────────────────────────────────────────

#[test]
fn unusual_path_scope_is_flagged_when_allowlist_is_set() {
    let cfg = AnomalyConfig {
        // Threshold so high it will never fire on the rate path; we only
        // want to exercise the path-scope branch.
        tool_call_threshold: 10_000,
        expected_path_prefixes: vec!["/workspace/".into(), "/tmp/".into()],
        ..Default::default()
    };
    let det = AnomalyDetector::new(cfg);
    let t = base_ts().timestamp();

    let mut ev = at(t, AuditEventType::ToolExecution, "a1");
    ev.target = Some("/etc/passwd".into());
    det.observe(&ev);

    let mut ok = at(t + 1, AuditEventType::ToolExecution, "a1");
    ok.target = Some("/workspace/src/main.rs".into());
    det.observe(&ok);

    let anomalies = det.drain_anomalies();
    assert_eq!(anomalies.len(), 1, "only the out-of-scope target should flag");
    assert!(
        matches!(
            &anomalies[0].kind,
            AnomalyKind::UnusualFileScopeRequest { path } if path == "/etc/passwd"
        ),
        "expected UnusualFileScopeRequest(/etc/passwd), got {:?}",
        anomalies[0].kind,
    );
}

#[test]
fn path_scope_check_is_noop_when_allowlist_empty() {
    // Default config has empty `expected_path_prefixes` — path-scope branch
    // must be entirely skipped, even for clearly weird targets.
    let det = AnomalyDetector::new(AnomalyConfig::default());
    let t = base_ts().timestamp();

    let mut ev = at(t, AuditEventType::ToolExecution, "a1");
    ev.target = Some("/etc/passwd".into());
    det.observe(&ev);

    assert_eq!(det.pending_count(), 0);
}

// ── Drain semantics ──────────────────────────────────────────────────────

#[test]
fn drain_empties_the_queue() {
    let det = AnomalyDetector::new(AnomalyConfig {
        violation_threshold: 1,
        ..Default::default()
    });
    let t = base_ts().timestamp();
    det.observe(&at(t, AuditEventType::PolicyViolation, "a1"));
    assert_eq!(det.pending_count(), 1);
    let drained = det.drain_anomalies();
    assert_eq!(drained.len(), 1);
    assert_eq!(det.pending_count(), 0);
}
