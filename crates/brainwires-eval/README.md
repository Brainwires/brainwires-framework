# brainwires-eval

[![Crates.io](https://img.shields.io/crates/v/brainwires-eval.svg)](https://crates.io/crates/brainwires-eval)
[![Documentation](https://img.shields.io/docsrs/brainwires-eval)](https://docs.rs/brainwires-eval)
[![License](https://img.shields.io/crates/l/brainwires-eval.svg)](LICENSE)

Evaluation framework for Brainwires agents — N-trial Monte Carlo runner, confidence intervals, tool sequence recording, and adversarial test cases.

## Overview

`brainwires-eval` provides a statistical evaluation framework for measuring agent reliability. Run any evaluation case N times across parallel trials, compute Wilson-score 95% confidence intervals, detect regressions against baselines, record and diff tool call sequences, and generate prioritized fault reports — all in a composable, async-native API.

**Design principles:**

- **Statistical rigor** — Wilson-score confidence intervals, not naive percentages
- **Composable** — implement `EvaluationCase` for any test, plug into the suite runner
- **Regression-aware** — track baselines over time, flag drops exceeding tolerance
- **Fault-prioritized** — regressions, consistent failures, and flaky tests are classified and ranked

```text
  ┌──────────────────────────────────────────────────────────────┐
  │                     EvaluationSuite                          │
  │                                                              │
  │  ┌─────────────┐    N trials     ┌────────────────────────┐ │
  │  │ Evaluation  │ ──────────────► │    Vec<TrialResult>    │ │
  │  │ Case (trait)│  (parallel)     │    EvaluationStats     │ │
  │  └─────────────┘                 │    95% CI (Wilson)     │ │
  │                                  └────────────────────────┘ │
  │                                             │               │
  │                    ┌────────────────────────┐│               │
  │                    │  RegressionSuite       ││               │
  │                    │  baseline comparison   │◄               │
  │                    └────────┬───────────────┘               │
  │                             │                               │
  │                    ┌────────▼───────────────┐               │
  │                    │  FaultReport           │               │
  │                    │  prioritized issues    │               │
  │                    └────────────────────────┘               │
  │                                                              │
  │  ┌────────────────┐  ┌──────────────┐  ┌────────────────┐  │
  │  │ToolSequence    │  │ Adversarial  │  │  Stability     │  │
  │  │Recorder        │  │ Test Cases   │  │  Tests         │  │
  │  │(Levenshtein)   │  │ (9 standard) │  │  (10 standard) │  │
  │  └────────────────┘  └──────────────┘  └────────────────┘  │
  └──────────────────────────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-eval = "0.1"
```

Run a simple evaluation:

```rust
use std::sync::Arc;
use brainwires_eval::{EvaluationSuite, AlwaysPassCase, StochasticCase};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let suite = EvaluationSuite::new(100); // 100 trials per case

    let cases: Vec<Arc<dyn brainwires_eval::EvaluationCase>> = vec![
        Arc::new(AlwaysPassCase::new("basic_sanity")),
        Arc::new(StochasticCase::new("flaky_test", 0.85)),
    ];

    let result = suite.run_suite(&cases).await;
    println!("Overall success rate: {:.1}%", result.overall_success_rate() * 100.0);

    for (name, stats) in &result.stats {
        println!(
            "{}: {:.1}% [{:.1}%, {:.1}%]",
            name,
            stats.success_rate * 100.0,
            stats.confidence_interval_95.lower * 100.0,
            stats.confidence_interval_95.upper * 100.0,
        );
    }

    Ok(())
}
```

## Architecture

### Trial Results & Statistics

Each trial produces a `TrialResult`; a batch of trials is summarized into `EvaluationStats`.

**`TrialResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `trial_id` | `usize` | Sequential index (0-based) |
| `success` | `bool` | Trial outcome |
| `duration_ms` | `u64` | Wall-clock duration |
| `error` | `Option<String>` | Error message on failure |
| `metadata` | `HashMap<String, Value>` | Arbitrary per-trial metadata |

Constructors: `TrialResult::success(id, ms)`, `TrialResult::failure(id, ms, error)`, plus `.with_meta(key, value)` builder.

**`EvaluationStats`:**

| Field | Type | Description |
|-------|------|-------------|
| `n_trials` | `usize` | Total trials run |
| `successes` | `usize` | Passing trials |
| `success_rate` | `f64` | Proportion passing |
| `confidence_interval_95` | `ConfidenceInterval95` | Wilson-score 95% CI |
| `mean_duration_ms` | `f64` | Average duration |
| `p50_duration_ms` | `f64` | Median duration |
| `p95_duration_ms` | `f64` | 95th percentile duration |

Computed via `EvaluationStats::from_trials(results)`.

**`ConfidenceInterval95`** — Wilson-score interval for binomial proportions:

```rust
let ci = ConfidenceInterval95::wilson(85, 100); // 85 successes out of 100
// ci.lower ≈ 0.766, ci.upper ≈ 0.912
```

### Evaluation Cases

Implement `EvaluationCase` for any test scenario:

```rust
#[async_trait]
pub trait EvaluationCase: Send + Sync {
    fn name(&self) -> &str;
    fn category(&self) -> &str;
    async fn run(&self, trial_id: usize) -> Result<TrialResult>;
}
```

**Built-in cases:**

| Case | Description |
|------|-------------|
| `AlwaysPassCase` | Always succeeds, configurable duration |
| `AlwaysFailCase` | Always fails with a fixed error message |
| `StochasticCase` | Deterministic per `trial_id` with a target success rate |

### Suite Runner

`EvaluationSuite` runs N trials per case with bounded parallelism.

**`SuiteConfig`:**

| Field | Default | Description |
|-------|---------|-------------|
| `n_trials` | 10 | Trials per case |
| `max_parallel` | 1 | Maximum concurrent trials |
| `catch_errors_as_failures` | `true` | Convert panics/errors to failed trials |

**`SuiteResult`:**

| Field | Type | Description |
|-------|------|-------------|
| `case_results` | `HashMap<String, Vec<TrialResult>>` | Raw results per case |
| `stats` | `HashMap<String, EvaluationStats>` | Computed statistics per case |

Methods: `overall_success_rate()`, `failing_cases(threshold)`.

### Tool Sequence Recording

Record and compare tool call sequences using Levenshtein edit distance.

**`ToolSequenceRecorder`:**

```rust
let recorder = ToolSequenceRecorder::new();

recorder.record("read_file", &serde_json::json!({"path": "src/main.rs"}));
recorder.record("write_file", &serde_json::json!({"path": "src/lib.rs"}));

let names = recorder.call_names(); // ["read_file", "write_file"]
let diff = recorder.diff_against(&["read_file", "edit_file", "write_file"]);
// diff.edit_distance = 1, diff.similarity ≈ 0.67
```

**`SequenceDiff`:**

| Field | Type | Description |
|-------|------|-------------|
| `expected` | `Vec<String>` | Expected tool sequence |
| `actual` | `Vec<String>` | Actual tool sequence |
| `edit_distance` | `usize` | Levenshtein distance |
| `similarity` | `f64` | `1.0 - edit_distance / max_len` |

Method: `is_exact_match()`.

**`ToolCallRecord`:**

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Tool name |
| `args_fingerprint` | `String` | 16-char FNV-style hash of args |
| `timestamp_ms` | `u64` | Unix epoch milliseconds |

### Adversarial Test Cases

Pre-built test cases for agent robustness.

**`AdversarialTestType` variants:**

| Variant | Key Fields | Description |
|---------|-----------|-------------|
| `PromptInjection` | `payload` | Injection payload to detect/reject |
| `AmbiguousInstruction` | `variants` | Multiple valid interpretations |
| `MissingContext` | `missing_key`, `expected_value` | Required context deliberately omitted |
| `BudgetExhaustion` | `max_steps`, `task_description` | Tasks that could exhaust iteration budget |

**Constructors:**

```rust
AdversarialTestCase::prompt_injection("name", "ignore previous instructions", true)
AdversarialTestCase::ambiguous_instruction("name", vec!["interp A", "interp B"])
AdversarialTestCase::missing_context("name", "api_key", None)
AdversarialTestCase::budget_exhaustion("name", 5, "complex recursive task")
```

**`standard_adversarial_suite()`** returns 9 pre-built cases: 3 prompt injection, 2 ambiguous, 2 missing context, 2 budget exhaustion.

### Regression Testing

Track baselines and detect performance regressions.

**`RegressionConfig`:**

| Field | Default | Description |
|-------|---------|-------------|
| `max_regression` | 0.05 | Maximum allowed drop (5 percentage points) |
| `min_trials` | 30 | Minimum trials before regression check applies |

**`RegressionSuite` workflow:**

```rust
let mut regression = RegressionSuite::new();

// Record baselines from a passing run
regression.record_baselines(&suite_result);
let json = regression.baselines_to_json()?;

// Later, check a new run against baselines
let regression = RegressionSuite::load_baselines_from_json(&json)?;
let result = regression.check(&new_suite_result);

if !result.passed {
    for cat in result.failing_categories() {
        println!("{}: {:.1}% → {:.1}% (regression: {:.1}pp)",
            cat.category,
            cat.baseline_success_rate * 100.0,
            cat.current_success_rate * 100.0,
            cat.regression * 100.0,
        );
    }
}
```

### Stability Tests

Long-horizon test cases for loop detection and goal preservation.

**`LoopDetectionSimCase`** — simulates tool call loops:

```rust
LoopDetectionSimCase::should_detect(100, "read_file", 20, 5)
// 100 steps, read_file loops starting at step 20, 5-call detection window

LoopDetectionSimCase::should_not_detect(50, 5)
// 50 unique steps, should NOT trigger detection
```

**`GoalPreservationCase`** — verifies goal reminders are injected:

```rust
GoalPreservationCase::new(30, 10)
// 30 iterations, goal reminder every 10 iterations
```

**`long_horizon_stability_suite()`** returns 10 pre-built cases: 4 loop detection (should detect), 2 loop detection (should not detect), 4 goal preservation (15/20/30/50 iterations).

### Fault Reports

Classify and prioritize issues from suite results.

**`FaultKind` variants:**

| Variant | Priority | Description |
|---------|----------|-------------|
| `Regression { previous_rate, current_rate, drop }` | 1–10 (scaled by drop) | Performance dropped vs baseline |
| `ConsistentFailure { success_rate }` | 8 | Consistently failing |
| `NewCapability { description }` | 5 | New test with no baseline |
| `Flaky { mean_rate, ci_width }` | 4 | Wide confidence interval |

**`analyze_suite_for_faults()`:**

```rust
let faults = analyze_suite_for_faults(
    &suite_result,
    Some(&regression_suite),
    0.5,    // consistent failure threshold
    0.3,    // flaky CI width threshold
);

for fault in &faults {
    println!("[P{}] {} ({}): {}",
        fault.priority(),
        fault.case_name,
        fault.fault_kind.label(),
        fault.suggested_task_description,
    );
}
```

Faults are returned sorted descending by priority.

## Usage Examples

### Custom Evaluation Case

```rust
use async_trait::async_trait;
use brainwires_eval::{EvaluationCase, TrialResult};

struct MyAgentTest {
    task: String,
}

#[async_trait]
impl EvaluationCase for MyAgentTest {
    fn name(&self) -> &str { &self.task }
    fn category(&self) -> &str { "agent/task_completion" }

    async fn run(&self, trial_id: usize) -> anyhow::Result<TrialResult> {
        let start = std::time::Instant::now();
        let success = run_agent_task(&self.task).await?;
        let duration = start.elapsed().as_millis() as u64;

        if success {
            Ok(TrialResult::success(trial_id, duration))
        } else {
            Ok(TrialResult::failure(trial_id, duration, "Task failed"))
        }
    }
}
```

### Parallel Suite with Regression Check

```rust
use std::sync::Arc;
use brainwires_eval::*;

let suite = EvaluationSuite::with_config(SuiteConfig {
    n_trials: 50,
    max_parallel: 10,
    catch_errors_as_failures: true,
});

let cases: Vec<Arc<dyn EvaluationCase>> = vec![
    Arc::new(my_test_a),
    Arc::new(my_test_b),
];

let result = suite.run_suite(&cases).await;

// Check against saved baselines
let regression = RegressionSuite::load_baselines_from_json(&saved_json)?;
let reg_result = regression.check(&result);

if !reg_result.passed {
    let faults = analyze_suite_for_faults(&result, Some(&regression), 0.5, 0.3);
    for f in &faults {
        eprintln!("[P{}] {}: {}", f.priority(), f.case_name, f.suggested_task_description);
    }
}
```

### Tool Sequence Verification

```rust
use brainwires_eval::ToolSequenceRecorder;

let recorder = ToolSequenceRecorder::new();

// Record calls during agent execution
recorder.record("read_file", &serde_json::json!({"path": "src/lib.rs"}));
recorder.record("edit_file", &serde_json::json!({"path": "src/lib.rs", "changes": "..."}));
recorder.record("verify_build", &serde_json::json!({"type": "cargo"}));

// Compare against expected sequence
let diff = recorder.diff_against(&["read_file", "edit_file", "verify_build"]);
assert!(diff.is_exact_match());
assert_eq!(diff.similarity, 1.0);
```

### Adversarial Testing

```rust
use std::sync::Arc;
use brainwires_eval::*;

let adversarial_cases = standard_adversarial_suite();
let suite = EvaluationSuite::new(20);

// Wrap adversarial cases as EvaluationCase implementations
// (requires adapter that runs the agent against each case)
```

### Stability Suite

```rust
use brainwires_eval::{EvaluationSuite, long_horizon_stability_suite};

let suite = EvaluationSuite::new(30);
let stability_cases = long_horizon_stability_suite();
let result = suite.run_suite(&stability_cases).await;

let failing = result.failing_cases(0.9); // cases below 90% pass rate
for name in failing {
    println!("Stability issue: {}", name);
}
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["eval"] }
```

Or use standalone — `brainwires-eval` depends only on `brainwires-core`.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
