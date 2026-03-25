/**
 * Anomaly detection for the audit system.
 *
 * Tracks statistical baselines for tool call frequency, policy violation rate,
 * and trust level changes. When observed values exceed configurable thresholds
 * an AnomalyEvent is emitted and held in an in-memory queue.
 *
 * Rust equivalent: `brainwires-permissions/src/anomaly.rs`
 * @module
 */

import type { AuditEvent, AuditEventType } from "./audit.ts";

// ── Anomaly Kind ────────────────────────────────────────────────────

/**
 * The kind of anomaly that was detected.
 *
 * Rust equivalent: `AnomalyKind` enum (serde `rename_all = "snake_case"`, tagged)
 */
export type AnomalyKind =
  | { kind: "repeated_policy_violation"; count: number; window_secs: number }
  | { kind: "high_frequency_tool_calls"; count: number; window_secs: number }
  | { kind: "unusual_file_scope_request"; path: string }
  | { kind: "rapid_trust_change"; changes: number; window_secs: number };

// ── Anomaly Event ───────────────────────────────────────────────────

/**
 * A single anomaly event produced by the detector.
 *
 * Rust equivalent: `AnomalyEvent` struct
 */
export interface AnomalyEvent {
  /** Unique identifier for this anomaly occurrence. */
  id: string;
  /** When the anomaly was detected (ISO 8601). */
  detected_at: string;
  /** Agent involved (if known). */
  agent_id: string | undefined;
  /** Structured kind with supporting metrics. */
  kind: AnomalyKind;
  /** Human-readable description suitable for logging or alerting. */
  description: string;
}

function createAnomalyEvent(
  agentId: string | undefined,
  kind: AnomalyKind,
  description: string,
): AnomalyEvent {
  return {
    id: crypto.randomUUID(),
    detected_at: new Date().toISOString(),
    agent_id: agentId,
    kind,
    description,
  };
}

// ── Anomaly Config ──────────────────────────────────────────────────

/**
 * Configuration for the anomaly detector.
 *
 * Rust equivalent: `AnomalyConfig` struct
 */
export interface AnomalyConfig {
  /** Sliding window duration for policy-violation counting (seconds). */
  violation_window_secs: number;
  /** Number of violations within the window that triggers an anomaly. */
  violation_threshold: number;
  /** Sliding window duration for tool-call rate counting (seconds). */
  tool_call_window_secs: number;
  /** Number of tool calls within the window that triggers an anomaly. */
  tool_call_threshold: number;
  /** Sliding window duration for trust-change counting (seconds). */
  trust_change_window_secs: number;
  /** Number of trust changes within the window that triggers an anomaly. */
  trust_change_threshold: number;
  /** Optional set of "expected" path prefixes. */
  expected_path_prefixes: string[];
}

/** Create default anomaly config. Rust equivalent: `AnomalyConfig::default()` */
export function defaultAnomalyConfig(): AnomalyConfig {
  return {
    violation_window_secs: 60,
    violation_threshold: 3,
    tool_call_window_secs: 10,
    tool_call_threshold: 20,
    trust_change_window_secs: 60,
    trust_change_threshold: 3,
    expected_path_prefixes: [],
  };
}

// ── Sliding Window Counter ──────────────────────────────────────────

/** Tracks event timestamps in a sliding window. Rust equivalent: `WindowCounter`. */
class WindowCounter {
  #timestamps: number[] = [];
  #windowSecs: number;

  constructor(windowSecs: number) {
    this.#windowSecs = windowSecs;
  }

  /** Record now and evict stale entries; returns in-window count. */
  recordAndCount(nowSecs: number): number {
    this.#timestamps.push(nowSecs);
    const cutoff = nowSecs - this.#windowSecs;
    while (this.#timestamps.length > 0 && this.#timestamps[0] <= cutoff) {
      this.#timestamps.shift();
    }
    return this.#timestamps.length;
  }
}

// ── Anomaly Detector ────────────────────────────────────────────────

/**
 * Stateful anomaly detector for the audit system.
 *
 * JS is single-threaded, so no Mutex is needed (unlike the Rust version).
 *
 * Rust equivalent: `AnomalyDetector` struct
 */
export class AnomalyDetector {
  #config: AnomalyConfig;
  #violationWindows: Map<string, WindowCounter> = new Map();
  #toolCallWindows: Map<string, WindowCounter> = new Map();
  #trustChangeWindows: Map<string, WindowCounter> = new Map();
  #pending: AnomalyEvent[] = [];

  /** Create a new detector with the given configuration. Rust equivalent: `AnomalyDetector::new()` */
  constructor(config: AnomalyConfig) {
    this.#config = config;
  }

  /**
   * Observe an AuditEvent and emit anomaly events if thresholds are breached.
   *
   * Rust equivalent: `AnomalyDetector::observe()`
   */
  observe(event: AuditEvent): void {
    const nowSecs = Math.floor(new Date(event.timestamp).getTime() / 1000);
    const agentKey = event.agent_id ?? "unknown";

    const eventType: AuditEventType = event.event_type;

    if (eventType === "policy_violation") {
      let window = this.#violationWindows.get(agentKey);
      if (!window) {
        window = new WindowCounter(this.#config.violation_window_secs);
        this.#violationWindows.set(agentKey, window);
      }
      const count = window.recordAndCount(nowSecs);
      if (count >= this.#config.violation_threshold) {
        this.#pending.push(createAnomalyEvent(
          event.agent_id,
          { kind: "repeated_policy_violation", count, window_secs: this.#config.violation_window_secs },
          `Agent '${agentKey}' triggered ${count} policy violations in ${this.#config.violation_window_secs}s`,
        ));
      }
    } else if (eventType === "tool_execution") {
      // Rate check
      let window = this.#toolCallWindows.get(agentKey);
      if (!window) {
        window = new WindowCounter(this.#config.tool_call_window_secs);
        this.#toolCallWindows.set(agentKey, window);
      }
      const count = window.recordAndCount(nowSecs);
      if (count >= this.#config.tool_call_threshold) {
        this.#pending.push(createAnomalyEvent(
          event.agent_id,
          { kind: "high_frequency_tool_calls", count, window_secs: this.#config.tool_call_window_secs },
          `Agent '${agentKey}' made ${count} tool calls in ${this.#config.tool_call_window_secs}s`,
        ));
      }

      // Path-scope check
      if (this.#config.expected_path_prefixes.length > 0 && event.target) {
        const isExpected = this.#config.expected_path_prefixes.some((prefix) =>
          event.target!.startsWith(prefix)
        );
        if (!isExpected) {
          this.#pending.push(createAnomalyEvent(
            event.agent_id,
            { kind: "unusual_file_scope_request", path: event.target },
            `Agent '${agentKey}' requested path '${event.target}' outside expected scope`,
          ));
        }
      }
    } else if (eventType === "trust_change") {
      let window = this.#trustChangeWindows.get(agentKey);
      if (!window) {
        window = new WindowCounter(this.#config.trust_change_window_secs);
        this.#trustChangeWindows.set(agentKey, window);
      }
      const count = window.recordAndCount(nowSecs);
      if (count >= this.#config.trust_change_threshold) {
        this.#pending.push(createAnomalyEvent(
          event.agent_id,
          { kind: "rapid_trust_change", changes: count, window_secs: this.#config.trust_change_window_secs },
          `Agent '${agentKey}' had ${count} trust changes in ${this.#config.trust_change_window_secs}s`,
        ));
      }
    }
  }

  /** Drain all pending anomaly events. Rust equivalent: `AnomalyDetector::drain_anomalies()` */
  drainAnomalies(): AnomalyEvent[] {
    const result = this.#pending;
    this.#pending = [];
    return result;
  }

  /** Return the number of pending anomaly events without draining. Rust equivalent: `AnomalyDetector::pending_count()` */
  pendingCount(): number {
    return this.#pending.length;
  }
}
