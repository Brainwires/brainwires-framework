use std::fmt;

use super::config::SelfImprovementConfig;

#[derive(Debug, Clone)]
pub enum SafetyStop {
    BudgetExceeded(f64),
    CycleLimitReached(u32),
    CircuitBreakerTripped(u32),
    DiffLimitExceeded(u32),
}

impl fmt::Display for SafetyStop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SafetyStop::BudgetExceeded(cost) => {
                write!(f, "Budget exceeded: ${cost:.2}")
            }
            SafetyStop::CycleLimitReached(cycles) => {
                write!(f, "Cycle limit reached: {cycles}")
            }
            SafetyStop::CircuitBreakerTripped(failures) => {
                write!(f, "Circuit breaker tripped after {failures} consecutive failures")
            }
            SafetyStop::DiffLimitExceeded(lines) => {
                write!(f, "Total diff limit exceeded: {lines} lines")
            }
        }
    }
}

pub struct SafetyGuard {
    config: SelfImprovementConfig,
    consecutive_failures: u32,
    total_cost: f64,
    total_diff_lines: u32,
    cycles_completed: u32,
}

impl SafetyGuard {
    pub fn new(config: &SelfImprovementConfig) -> Self {
        Self {
            config: config.clone(),
            consecutive_failures: 0,
            total_cost: 0.0,
            total_diff_lines: 0,
            cycles_completed: 0,
        }
    }

    pub fn check_can_continue(&self) -> Result<(), SafetyStop> {
        if self.total_cost >= self.config.max_budget {
            return Err(SafetyStop::BudgetExceeded(self.total_cost));
        }
        if self.cycles_completed >= self.config.max_cycles {
            return Err(SafetyStop::CycleLimitReached(self.cycles_completed));
        }
        if self.consecutive_failures >= self.config.circuit_breaker_threshold {
            return Err(SafetyStop::CircuitBreakerTripped(self.consecutive_failures));
        }
        if self.total_diff_lines >= self.config.max_total_diff {
            return Err(SafetyStop::DiffLimitExceeded(self.total_diff_lines));
        }
        Ok(())
    }

    pub fn record_success(&mut self, diff_lines: u32) {
        self.consecutive_failures = 0;
        self.total_diff_lines += diff_lines;
        self.cycles_completed += 1;
    }

    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.cycles_completed += 1;
    }

    pub fn record_cost(&mut self, cost: f64) {
        self.total_cost += cost;
    }

    pub fn cycles_completed(&self) -> u32 {
        self.cycles_completed
    }

    pub fn total_cost(&self) -> f64 {
        self.total_cost
    }

    pub fn total_diff_lines(&self) -> u32 {
        self.total_diff_lines
    }
}
