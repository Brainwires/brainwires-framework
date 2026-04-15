//! Session-level budget enforcement across all agents in a session.
//!
//! `SessionBudget` is shared via `Arc` across all `TaskAgent` instances spawned within a
//! single user session. It provides a hard cap on total tokens and cost so that a runaway
//! agent or a large parallel batch cannot exhaust the user's budget silently.
//!
//! # Design
//!
//! - Atomic counters for lock-free updates from concurrent agents.
//! - `cost_used` is stored as **microcents** (u64) to avoid floating-point atomics while
//!   still supporting sub-cent precision.
//! - `check_before_spawn()` is called before creating a new agent; `record_run()` is
//!   called after each provider response is received.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use thiserror::Error;

/// Errors returned when a session budget is exceeded.
#[derive(Debug, Error, Clone)]
pub enum BudgetError {
    #[error("token budget exceeded: used {used} of {limit} tokens")]
    TokensExceeded { used: u64, limit: u64 },

    #[error("cost budget exceeded: used ${used_usd:.4} of ${limit_usd:.4}")]
    CostExceeded { used_usd: f64, limit_usd: f64 },

    #[error("agent budget exceeded: spawned {spawned} of {limit} agents")]
    AgentsExceeded { spawned: u32, limit: u32 },
}

/// Microcents per dollar — used to represent cost as an integer for atomics.
const MICROCENTS_PER_DOLLAR: f64 = 1_000_000.0;

/// Session-level budget shared across all agents spawned in one user session.
///
/// Construct with [`SessionBudget::new`] and wrap in `Arc` before passing to
/// [`crate::agents::task_agent::TaskAgentConfig`].
#[derive(Debug)]
pub struct SessionBudget {
    /// Maximum total tokens allowed across all agents (None = unlimited).
    pub max_total_tokens: Option<u64>,
    /// Maximum total cost in USD across all agents (None = unlimited).
    pub max_total_cost_usd: Option<f64>,
    /// Maximum number of agents that may be spawned (None = unlimited).
    pub max_agents: Option<u32>,

    // --- shared atomic counters ---
    tokens_used: AtomicU64,
    /// Stored as microcents to avoid floating-point atomics.
    cost_used_microcents: AtomicU64,
    agents_spawned: AtomicU32,
}

impl SessionBudget {
    /// Create a new session budget. All limits default to `None` (unlimited).
    pub fn new() -> Self {
        Self {
            max_total_tokens: None,
            max_total_cost_usd: None,
            max_agents: None,
            tokens_used: AtomicU64::new(0),
            cost_used_microcents: AtomicU64::new(0),
            agents_spawned: AtomicU32::new(0),
        }
    }

    /// Builder: set token limit.
    pub fn with_max_tokens(mut self, tokens: u64) -> Self {
        self.max_total_tokens = Some(tokens);
        self
    }

    /// Builder: set cost limit in USD.
    pub fn with_max_cost_usd(mut self, usd: f64) -> Self {
        self.max_total_cost_usd = Some(usd);
        self
    }

    /// Builder: set agent spawn limit.
    pub fn with_max_agents(mut self, agents: u32) -> Self {
        self.max_agents = Some(agents);
        self
    }

    /// Wrap in an `Arc` for sharing across agents.
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Current accumulated token usage.
    pub fn tokens_used(&self) -> u64 {
        self.tokens_used.load(Ordering::Relaxed)
    }

    /// Current accumulated cost in USD.
    pub fn cost_used_usd(&self) -> f64 {
        self.cost_used_microcents.load(Ordering::Relaxed) as f64 / MICROCENTS_PER_DOLLAR
    }

    /// Number of agents spawned so far.
    pub fn agents_spawned(&self) -> u32 {
        self.agents_spawned.load(Ordering::Relaxed)
    }

    /// Check whether spawning a new agent is allowed. Call this **before** constructing a
    /// `TaskAgent`. Returns `Err` if any limit would be exceeded.
    ///
    /// # Design: pre-spawn is stricter than mid-run
    ///
    /// This function uses `>=` comparisons: it denies when the budget is **already at** the
    /// limit, not just above it. This is intentional — we do not want to start another agent
    /// if there is no headroom left. See [`check_limits`](Self::check_limits) which uses `>`
    /// and therefore allows a mid-run agent to finish when exactly at the limit.
    pub fn check_before_spawn(&self) -> Result<(), BudgetError> {
        if let Some(limit) = self.max_agents {
            let spawned = self.agents_spawned.load(Ordering::Acquire);
            if spawned >= limit {
                return Err(BudgetError::AgentsExceeded { spawned, limit });
            }
        }
        if let Some(limit) = self.max_total_tokens {
            let used = self.tokens_used.load(Ordering::Acquire);
            if used >= limit {
                return Err(BudgetError::TokensExceeded { used, limit });
            }
        }
        if let Some(limit_usd) = self.max_total_cost_usd {
            let used_usd = self.cost_used_usd();
            if used_usd >= limit_usd {
                return Err(BudgetError::CostExceeded {
                    used_usd,
                    limit_usd,
                });
            }
        }
        Ok(())
    }

    /// Record a completed provider call. Call this after every successful `call_provider`.
    ///
    /// Also increments the agent-spawn counter the first time it is called from a given
    /// agent; callers that want to pre-increment (before the first call) should call
    /// [`increment_agent_count`] separately.
    pub fn record_run(&self, tokens: u64, cost_usd: f64) {
        self.tokens_used.fetch_add(tokens, Ordering::Relaxed);
        let microcents = (cost_usd * MICROCENTS_PER_DOLLAR) as u64;
        self.cost_used_microcents
            .fetch_add(microcents, Ordering::Relaxed);
    }

    /// Increment the spawned-agent counter. Call once when a `TaskAgent` starts executing.
    pub fn increment_agent_count(&self) {
        self.agents_spawned.fetch_add(1, Ordering::Relaxed);
    }

    /// Check whether the session limits are currently exceeded. Call this after
    /// `record_run` to decide whether to abort the current agent.
    ///
    /// # Design: mid-run allows exact-equal, aborts above
    ///
    /// This function uses `>` comparisons: a running agent is allowed to complete the
    /// current call when the budget is **exactly at** the limit. Only exceeding the limit
    /// triggers an abort. Compare with [`check_before_spawn`](Self::check_before_spawn)
    /// which uses `>=` and is therefore stricter.
    pub fn check_limits(&self) -> Result<(), BudgetError> {
        if let Some(limit) = self.max_total_tokens {
            let used = self.tokens_used.load(Ordering::Acquire);
            if used > limit {
                return Err(BudgetError::TokensExceeded { used, limit });
            }
        }
        if let Some(limit_usd) = self.max_total_cost_usd {
            let used_usd = self.cost_used_usd();
            if used_usd > limit_usd {
                return Err(BudgetError::CostExceeded {
                    used_usd,
                    limit_usd,
                });
            }
        }
        Ok(())
    }
}

impl Default for SessionBudget {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn unlimited_budget_always_allows_spawn() {
        let b = SessionBudget::new();
        assert!(b.check_before_spawn().is_ok());
        b.record_run(1_000_000, 100.0);
        assert!(b.check_before_spawn().is_ok());
    }

    #[test]
    fn agent_limit_blocks_at_exact_capacity() {
        let b = SessionBudget::new().with_max_agents(2);
        b.increment_agent_count();
        b.increment_agent_count();
        // Design: check_before_spawn uses >= so it blocks when already AT limit
        match b.check_before_spawn() {
            Err(BudgetError::AgentsExceeded {
                spawned: 2,
                limit: 2,
            }) => {}
            other => panic!("expected AgentsExceeded, got {:?}", other),
        }
    }

    #[test]
    fn token_limit_blocks_before_spawn_at_limit() {
        let b = SessionBudget::new().with_max_tokens(500);
        b.record_run(500, 0.0);
        // Design: check_before_spawn uses >= (denies when AT limit)
        assert!(matches!(
            b.check_before_spawn(),
            Err(BudgetError::TokensExceeded { .. })
        ));
    }

    #[test]
    fn token_limit_allows_running_at_limit_aborts_above() {
        let b = SessionBudget::new().with_max_tokens(500);
        b.record_run(500, 0.0);
        // Design: check_limits uses > (allows AT limit, aborts ABOVE)
        assert!(
            b.check_limits().is_ok(),
            "exact-equal should still be allowed mid-run"
        );
        b.record_run(1, 0.0);
        assert!(matches!(
            b.check_limits(),
            Err(BudgetError::TokensExceeded { .. })
        ));
    }

    #[test]
    fn cost_limit_blocks_before_spawn_at_limit() {
        let b = SessionBudget::new().with_max_cost_usd(1.0);
        b.record_run(0, 1.0);
        assert!(matches!(
            b.check_before_spawn(),
            Err(BudgetError::CostExceeded { .. })
        ));
    }

    #[test]
    fn cost_limit_allows_running_at_limit_aborts_above() {
        let b = SessionBudget::new().with_max_cost_usd(1.0);
        b.record_run(0, 1.0);
        // Design: check_limits uses > (allows AT limit, aborts ABOVE)
        assert!(b.check_limits().is_ok());
        b.record_run(0, 0.0001);
        assert!(matches!(
            b.check_limits(),
            Err(BudgetError::CostExceeded { .. })
        ));
    }

    #[test]
    fn record_run_accumulates_correctly() {
        let b = SessionBudget::new();
        b.record_run(100, 0.5);
        b.record_run(200, 0.25);
        assert_eq!(b.tokens_used(), 300);
        // 0.75 USD stored as microcents: 750_000
        let cost = b.cost_used_usd();
        assert!((cost - 0.75).abs() < 1e-6, "cost was {}", cost);
    }

    #[test]
    fn cost_microcent_precision_4_decimal_places() {
        let b = SessionBudget::new();
        b.record_run(0, 0.0001); // 1 ten-thousandth of a dollar = 100 microcents
        let cost = b.cost_used_usd();
        assert!((cost - 0.0001).abs() < 1e-9, "cost was {}", cost);
    }

    #[test]
    fn increment_agent_count_and_read_back() {
        let b = SessionBudget::new();
        assert_eq!(b.agents_spawned(), 0);
        b.increment_agent_count();
        b.increment_agent_count();
        assert_eq!(b.agents_spawned(), 2);
    }

    #[test]
    fn concurrent_record_run_does_not_corrupt_state() {
        let b = Arc::new(SessionBudget::new());
        std::thread::scope(|s| {
            for _ in 0..8 {
                let b = Arc::clone(&b);
                s.spawn(move || {
                    for _ in 0..100 {
                        b.record_run(1, 0.000_001);
                    }
                });
            }
        });
        assert_eq!(b.tokens_used(), 800);
        let cost = b.cost_used_usd();
        assert!((cost - 0.000_800).abs() < 1e-9, "cost was {}", cost);
    }
}
