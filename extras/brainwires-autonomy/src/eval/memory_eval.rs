//! Empirical evaluation cases for [`MultiFactorScore`] and tier demotion.
//!
//! Validates that the scoring heuristics in `brainwires-storage` produce
//! correct relative orderings — not just that the math is internally
//! consistent, but that the rankings match reasonable real-world expectations.
//!
//! All cases are deterministic (no LLM calls, no I/O) and complete in <1 ms.
//!
//! ## Cases
//!
//! | Case | What it validates |
//! |------|------------------|
//! | [`MultiFactorRankingCase`] | 4 scenarios verifying similarity, recency, fast-decay, and importance ordering |
//! | [`TierDemotionCase`] | `TierMetadata::retention_score` orders demotion candidates correctly |

use std::sync::Arc;

use async_trait::async_trait;
use brainwires_agents::eval::{EvaluationCase, TrialResult, ndcg_at_k};
use brainwires_storage::tiered_memory::{
    MemoryAuthority, MemoryTier, MultiFactorScore, TierMetadata,
};
use chrono::Utc;

// ── Scenario helpers ──────────────────────────────────────────────────────────

struct Scenario {
    name: &'static str,
    /// (similarity, recency_hours, importance, use_fast_decay)
    entries: Vec<(f32, f32, f32, bool)>,
    /// Ground-truth relevance labels, highest = most relevant.
    ground_truth: Vec<usize>,
}

fn compute_scores(scenario: &Scenario) -> Vec<f64> {
    scenario
        .entries
        .iter()
        .map(|(sim, hours, imp, fast)| {
            let recency = if *fast {
                MultiFactorScore::recency_from_hours_fast(*hours)
            } else {
                MultiFactorScore::recency_from_hours(*hours)
            };
            MultiFactorScore::compute(*sim, recency, *imp).combined as f64
        })
        .collect()
}

// ── Case 1: MultiFactorScore ranking ─────────────────────────────────────────

/// Runs 4 deterministic scenarios and asserts NDCG >= 0.99 for each.
///
/// ## Scenarios
///
/// **A — Similarity Dominance**: Recency and importance are held equal;
/// similarity varies 0.95 → 0.05. Expected ordering is strictly by similarity.
/// Combined scores: A1=0.855, A2=0.680, A3=0.480, A4=0.405.
///
/// **B — Recency Decay** (`exp(-0.01 * hours)`): Similarity and importance
/// held equal; age varies 1h → 30d. Expected ordering is strictly by recency.
/// Combined scores: B1≈0.767, B2≈0.706, B3≈0.526, B4≈0.470.
///
/// **C — Fast Decay** (`exp(-0.05 * hours)`): Simulates a temporal query.
/// High-similarity old items collapse; freshness dominates.
/// Expected order: C1(1h) > C2(24h) > C4(168h) > C3(72h).
///
/// **D — Importance Tiebreaker**: Similarity and recency are equal; importance
/// varies 0.90 → 0.05. Expected ordering is strictly by importance.
/// Combined scores: D1=0.740, D2=0.680, D3=0.620, D4=0.570.
pub struct MultiFactorRankingCase;

#[async_trait]
impl EvaluationCase for MultiFactorRankingCase {
    fn name(&self) -> &str {
        "multi_factor_score_ranking"
    }
    fn category(&self) -> &str {
        "memory"
    }

    async fn run(&self, trial_id: usize) -> anyhow::Result<TrialResult> {
        let start = std::time::Instant::now();

        let scenarios = vec![
            // A: similarity dominance (recency_hours, importance held equal)
            Scenario {
                name: "A_similarity_dominance",
                entries: vec![
                    (0.95, 5.0, 0.70, false), // A1 combined ≈ 0.855
                    (0.60, 5.0, 0.70, false), // A2 combined ≈ 0.680
                    (0.20, 5.0, 0.70, false), // A3 combined ≈ 0.480
                    (0.05, 5.0, 0.70, false), // A4 combined ≈ 0.405
                ],
                ground_truth: vec![3, 2, 1, 0],
            },
            // B: recency decay (similarity, importance held equal)
            Scenario {
                name: "B_recency_decay",
                entries: vec![
                    (0.70, 1.0, 0.60, false),   // B1 ≈ 0.767
                    (0.70, 24.0, 0.60, false),  // B2 ≈ 0.706
                    (0.70, 168.0, 0.60, false), // B3 ≈ 0.526
                    (0.70, 720.0, 0.60, false), // B4 ≈ 0.470
                ],
                ground_truth: vec![3, 2, 1, 0],
            },
            // C: fast decay — temporal query collapses old items
            // Expected: C1(1h) > C2(24h) > C4(168h) > C3(72h)
            Scenario {
                name: "C_fast_decay_temporal",
                entries: vec![
                    (0.50, 1.0, 0.80, true),   // C1 ≈ 0.695  → rank 1
                    (0.85, 24.0, 0.80, true),  // C2 ≈ 0.675  → rank 2
                    (0.90, 72.0, 0.90, true),  // C3 ≈ 0.638  → rank 4
                    (0.95, 168.0, 0.95, true), // C4 ≈ 0.665  → rank 3
                ],
                ground_truth: vec![3, 2, 0, 1], // C1=3, C2=2, C3=0, C4=1
            },
            // D: importance tiebreaker (similarity, recency equal)
            Scenario {
                name: "D_importance_tiebreaker",
                entries: vec![
                    (0.70, 5.0, 0.90, false), // D1 ≈ 0.740
                    (0.70, 5.0, 0.60, false), // D2 ≈ 0.680
                    (0.70, 5.0, 0.30, false), // D3 ≈ 0.620
                    (0.70, 5.0, 0.05, false), // D4 ≈ 0.570
                ],
                ground_truth: vec![3, 2, 1, 0],
            },
        ];

        let mut all_ndcg = Vec::new();
        let mut failures = Vec::new();

        for scenario in &scenarios {
            let scores = compute_scores(scenario);
            let ndcg = ndcg_at_k(&scores, &scenario.ground_truth, 0);
            all_ndcg.push(ndcg);
            if ndcg < 0.99 {
                failures.push(format!(
                    "{}: NDCG={ndcg:.4} (scores={scores:?})",
                    scenario.name
                ));
            }
        }

        let mean_ndcg = all_ndcg.iter().sum::<f64>() / all_ndcg.len() as f64;
        let ms = start.elapsed().as_millis() as u64;

        if failures.is_empty() {
            Ok(TrialResult::success(trial_id, ms)
                .with_meta("ndcg_mean", serde_json::json!(mean_ndcg))
                .with_meta("ndcg_per_scenario", serde_json::json!(all_ndcg)))
        } else {
            Ok(TrialResult::failure(
                trial_id,
                ms,
                format!("MultiFactorScore ranking failures: {}", failures.join("; ")),
            )
            .with_meta("ndcg_mean", serde_json::json!(mean_ndcg))
            .with_meta("failures", serde_json::json!(failures)))
        }
    }
}

// ── Case 2: Tier demotion ordering ────────────────────────────────────────────

/// Validates that `TierMetadata::retention_score()` ranks entries so that the
/// lowest-retention entries are demoted first.
///
/// ## Scenario
///
/// Four entries with varying importance, age, and access count:
///
/// | Entry | importance | age    | accesses | expected retention |
/// |-------|-----------|--------|----------|-------------------|
/// | R1    | 0.90      | ~1h    | 10       | ≈ 0.795 (keep)    |
/// | R2    | 0.50      | ~24h   | 3        | ≈ 0.514 (medium)  |
/// | R3    | 0.20      | ~168h  | 1        | ≈ 0.170 (demote)  |
/// | R4    | 0.05      | ~720h  | 0        | ≈ 0.025 (demote first) |
///
/// Expected demotion order (ascending retention): R4 < R3 < R2 < R1.
///
/// ## Note on constant naming
///
/// `TierMetadata::retention_score` computes:
/// `importance * SIMILARITY_WEIGHT(0.50) + recency * RECENCY_WEIGHT(0.30) + access_factor * IMPORTANCE_WEIGHT(0.20)`
///
/// The `SIMILARITY_WEIGHT` constant (0.50) is borrowed from `MultiFactorScore`
/// rather than being a dedicated `IMPORTANCE_RETENTION_WEIGHT`. This is
/// semantically confusing (the importance term uses a constant named for
/// similarity) but numerically correct. A future refactor should rename it.
pub struct TierDemotionCase;

fn make_tier_metadata(importance: f32, age_secs: i64, access_count: u32) -> TierMetadata {
    let now = Utc::now().timestamp();
    TierMetadata {
        message_id: uuid::Uuid::new_v4().to_string(),
        tier: MemoryTier::Hot,
        importance,
        last_accessed: now - age_secs,
        access_count,
        created_at: now - age_secs,
        authority: MemoryAuthority::Session,
    }
}

#[async_trait]
impl EvaluationCase for TierDemotionCase {
    fn name(&self) -> &str {
        "tier_demotion_ordering"
    }
    fn category(&self) -> &str {
        "memory"
    }

    async fn run(&self, trial_id: usize) -> anyhow::Result<TrialResult> {
        let start = std::time::Instant::now();

        // Build entries with age expressed in seconds for precision.
        let entries = [
            ("R1_keep", make_tier_metadata(0.90, 3_600, 10)), // ~1h
            ("R2_medium", make_tier_metadata(0.50, 86_400, 3)), // ~24h
            ("R3_demote", make_tier_metadata(0.20, 604_800, 1)), // ~168h
            ("R4_demote_first", make_tier_metadata(0.05, 2_592_000, 0)), // ~720h
        ];

        // Ground truth: R1 should be kept (highest relevance=3), R4 demoted first (relevance=0).
        let ground_truth: Vec<usize> = vec![3, 2, 1, 0];

        // retention_score() is the "keep score" — higher = keep. Use it as ranking score.
        let scores: Vec<f64> = entries
            .iter()
            .map(|(_, meta)| meta.retention_score() as f64)
            .collect();

        let ndcg = ndcg_at_k(&scores, &ground_truth, 4);
        let ms = start.elapsed().as_millis() as u64;

        let score_detail: Vec<String> = entries
            .iter()
            .zip(scores.iter())
            .map(|((name, _), score)| format!("{name}={score:.4}"))
            .collect();

        let naming_note = "Note: TierMetadata::retention_score uses SIMILARITY_WEIGHT(0.50) \
            for the importance term — semantically confusing but numerically correct. \
            Consider renaming to IMPORTANCE_RETENTION_WEIGHT in a future refactor.";

        if ndcg >= 0.99 {
            Ok(TrialResult::success(trial_id, ms)
                .with_meta("ndcg", serde_json::json!(ndcg))
                .with_meta("retention_scores", serde_json::json!(score_detail))
                .with_meta("naming_note", serde_json::json!(naming_note)))
        } else {
            Ok(TrialResult::failure(
                trial_id,
                ms,
                format!(
                    "NDCG@4={ndcg:.4} < 0.99 — demotion ordering is incorrect. \
                     scores=[{}]",
                    score_detail.join(", ")
                ),
            )
            .with_meta("ndcg", serde_json::json!(ndcg)))
        }
    }
}

// ── Suite constructor ─────────────────────────────────────────────────────────

/// Return all memory eval cases ready for use with
/// [`brainwires_agents::eval::EvaluationSuite`] or
/// [`brainwires_autonomy::self_improve::AutonomousFeedbackLoop`].
pub fn multi_factor_suite() -> Vec<Arc<dyn EvaluationCase>> {
    vec![Arc::new(MultiFactorRankingCase), Arc::new(TierDemotionCase)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_factor_ranking_passes() {
        let case = MultiFactorRankingCase;
        let result = case.run(0).await.unwrap();
        assert!(
            result.success,
            "MultiFactorRankingCase failed: {:?}",
            result.error
        );
    }

    #[tokio::test]
    async fn test_tier_demotion_passes() {
        let case = TierDemotionCase;
        let result = case.run(0).await.unwrap();
        assert!(
            result.success,
            "TierDemotionCase failed: {:?}",
            result.error
        );
    }
}
