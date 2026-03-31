use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::comparator::ComparisonResult;
use super::safety::SafetyStop;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyMetrics {
    pub tasks_generated: u32,
    pub tasks_attempted: u32,
    pub tasks_succeeded: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub start_time: DateTime<Utc>,
    pub tasks_attempted: u32,
    pub tasks_succeeded: u32,
    pub tasks_failed: u32,
    pub per_strategy: HashMap<String, StrategyMetrics>,
    pub comparisons: Vec<ComparisonResult>,
    pub total_cost: f64,
    pub total_iterations: u32,
    pub commits: Vec<String>,
}

impl SessionMetrics {
    pub fn new() -> Self {
        Self {
            start_time: Utc::now(),
            tasks_attempted: 0,
            tasks_succeeded: 0,
            tasks_failed: 0,
            per_strategy: HashMap::new(),
            comparisons: Vec::new(),
            total_cost: 0.0,
            total_iterations: 0,
            commits: Vec::new(),
        }
    }

    pub fn record_attempt(&mut self, strategy: &str) {
        self.tasks_attempted += 1;
        self.per_strategy
            .entry(strategy.to_string())
            .or_default()
            .tasks_attempted += 1;
    }

    pub fn record_success(&mut self, strategy: &str, iterations: u32) {
        self.tasks_succeeded += 1;
        self.total_iterations += iterations;
        self.per_strategy
            .entry(strategy.to_string())
            .or_default()
            .tasks_succeeded += 1;
    }

    pub fn record_failure(&mut self, strategy: &str) {
        self.tasks_failed += 1;
        self.per_strategy
            .entry(strategy.to_string())
            .or_default()
            .tasks_attempted += 1;
    }

    pub fn record_generated(&mut self, strategy: &str, count: u32) {
        self.per_strategy
            .entry(strategy.to_string())
            .or_default()
            .tasks_generated += count;
    }

    pub fn record_comparison(&mut self, comparison: ComparisonResult) {
        self.comparisons.push(comparison);
    }

    pub fn record_commit(&mut self, hash: String) {
        self.commits.push(hash);
    }

    pub fn success_rate(&self) -> f64 {
        if self.tasks_attempted == 0 {
            0.0
        } else {
            self.tasks_succeeded as f64 / self.tasks_attempted as f64
        }
    }
}

impl Default for SessionMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionReport {
    pub metrics: SessionMetrics,
    #[serde(with = "duration_serde")]
    pub duration: Duration,
    pub safety_stop_reason: Option<String>,
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(d: &Duration, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        d.as_secs_f64().serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = f64::deserialize(d)?;
        Ok(Duration::from_secs_f64(secs))
    }
}

impl SessionReport {
    pub fn new(
        metrics: SessionMetrics,
        duration: Duration,
        stop_reason: Option<SafetyStop>,
    ) -> Self {
        Self {
            metrics,
            duration,
            safety_stop_reason: stop_reason.map(|r| r.to_string()),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Self-Improvement Session Report\n\n");
        md.push_str(&format!(
            "**Date**: {}\n",
            self.metrics.start_time.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        md.push_str(&format!(
            "**Duration**: {:.1}s\n",
            self.duration.as_secs_f64()
        ));
        md.push_str(&format!(
            "**Success Rate**: {:.1}%\n\n",
            self.metrics.success_rate() * 100.0
        ));

        md.push_str("## Summary\n\n");
        md.push_str("| Metric | Value |\n|--------|-------|\n");
        md.push_str(&format!(
            "| Tasks Attempted | {} |\n",
            self.metrics.tasks_attempted
        ));
        md.push_str(&format!(
            "| Tasks Succeeded | {} |\n",
            self.metrics.tasks_succeeded
        ));
        md.push_str(&format!(
            "| Tasks Failed | {} |\n",
            self.metrics.tasks_failed
        ));
        md.push_str(&format!(
            "| Total Iterations | {} |\n",
            self.metrics.total_iterations
        ));
        md.push_str(&format!(
            "| Estimated Cost | ${:.4} |\n",
            self.metrics.total_cost
        ));
        md.push_str(&format!("| Commits | {} |\n", self.metrics.commits.len()));

        if !self.metrics.per_strategy.is_empty() {
            md.push_str("\n## Per-Strategy Breakdown\n\n");
            md.push_str("| Strategy | Generated | Attempted | Succeeded |\n");
            md.push_str("|----------|-----------|-----------|----------|\n");
            for (name, stats) in &self.metrics.per_strategy {
                md.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    name, stats.tasks_generated, stats.tasks_attempted, stats.tasks_succeeded
                ));
            }
        }

        if !self.metrics.comparisons.is_empty() {
            md.push_str("\n## Dual-Path Comparisons\n\n");
            let both_ok = self
                .metrics
                .comparisons
                .iter()
                .filter(|c| c.both_succeeded)
                .count();
            let diffs_match = self
                .metrics
                .comparisons
                .iter()
                .filter(|c| c.diffs_match)
                .count();
            md.push_str(&format!(
                "- Both paths succeeded: {}/{}\n",
                both_ok,
                self.metrics.comparisons.len()
            ));
            md.push_str(&format!("- Diffs matched: {}/{}\n", diffs_match, both_ok));
        }

        if let Some(ref reason) = self.safety_stop_reason {
            md.push_str(&format!("\n## Stop Reason\n\n{reason}\n"));
        }

        if !self.metrics.commits.is_empty() {
            md.push_str("\n## Commits\n\n");
            for hash in &self.metrics.commits {
                md.push_str(&format!("- `{hash}`\n"));
            }
        }

        md
    }

    pub fn save(&self, output_dir: &str) -> anyhow::Result<()> {
        std::fs::create_dir_all(output_dir)?;
        let timestamp = self.metrics.start_time.format("%Y%m%d-%H%M%S").to_string();

        let json_path = format!("{output_dir}/session-{timestamp}.json");
        std::fs::write(&json_path, self.to_json()?)?;

        let md_path = format!("{output_dir}/session-{timestamp}.md");
        std::fs::write(&md_path, self.to_markdown())?;

        tracing::info!("Session report saved to {json_path} and {md_path}");
        Ok(())
    }
}
