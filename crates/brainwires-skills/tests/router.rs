//! Integration tests for the SkillRouter.
//!
//! Tests keyword matching, confidence thresholds, explicit matches,
//! suggestion formatting, and interaction with a populated registry.

use brainwires_agents::skills::{MatchSource, SkillMatch, SkillMetadata, SkillRegistry, SkillRouter};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Build a registry with several skills for router testing.
async fn populated_registry() -> Arc<RwLock<SkillRegistry>> {
    let mut registry = SkillRegistry::new();

    let mut review = SkillMetadata::new(
        "review-pr".to_string(),
        "Reviews pull requests for code quality, security issues, and best practices".to_string(),
    );
    review.allowed_tools = Some(vec!["Read".to_string(), "Grep".to_string()]);

    let commit = SkillMetadata::new(
        "commit".to_string(),
        "Creates well-formatted git commits following conventional commit standards".to_string(),
    );

    let explain = SkillMetadata::new(
        "explain-code".to_string(),
        "Explains code functionality in detail, breaking down complex logic step by step"
            .to_string(),
    );

    let deploy = SkillMetadata::new(
        "deploy-app".to_string(),
        "Deploys the application to staging or production using Docker containers".to_string(),
    );

    registry.register(review);
    registry.register(commit);
    registry.register(explain);
    registry.register(deploy);

    Arc::new(RwLock::new(registry))
}

// ---------------------------------------------------------------------------
// Keyword matching
// ---------------------------------------------------------------------------

#[tokio::test]
async fn match_by_skill_name_words() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = router.match_skills("review my pull request").await;
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.skill_name == "review-pr"));
}

#[tokio::test]
async fn match_by_description_keywords() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = router.match_skills("check code quality").await;
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.skill_name == "review-pr"));
}

#[tokio::test]
async fn match_commit_skill() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = router.match_skills("create a commit message").await;
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.skill_name == "commit"));
}

#[tokio::test]
async fn match_deploy_skill() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = router.match_skills("deploy the application").await;
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.skill_name == "deploy-app"));
}

#[tokio::test]
async fn matches_sorted_by_confidence_descending() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = router.match_skills("review code quality").await;
    if matches.len() >= 2 {
        for window in matches.windows(2) {
            assert!(window[0].confidence >= window[1].confidence);
        }
    }
}

// ---------------------------------------------------------------------------
// Empty / no-match cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn empty_query_returns_no_matches() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = router.match_skills("").await;
    assert!(matches.is_empty());
}

#[tokio::test]
async fn short_words_only_returns_no_matches() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    // All words are <= 2 chars, so they should be filtered out
    let matches = router.match_skills("do it").await;
    assert!(matches.is_empty());
}

#[tokio::test]
async fn empty_registry_returns_no_matches() {
    let reg = Arc::new(RwLock::new(SkillRegistry::new()));
    let router = SkillRouter::new(reg);

    let matches = router.match_skills("review my code").await;
    assert!(matches.is_empty());
}

// ---------------------------------------------------------------------------
// Confidence threshold
// ---------------------------------------------------------------------------

#[tokio::test]
async fn custom_confidence_threshold_filters_low_matches() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg).with_min_confidence(0.9);

    // With a very high threshold, fewer (or no) matches should pass
    let matches = router.match_skills("something vaguely related").await;
    for m in &matches {
        assert!(m.confidence >= 0.9);
    }
}

// ---------------------------------------------------------------------------
// Explicit match
// ---------------------------------------------------------------------------

#[tokio::test]
async fn explicit_match_has_full_confidence() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let m = router.explicit_match("review-pr");
    assert_eq!(m.skill_name, "review-pr");
    assert_eq!(m.confidence, 1.0);
    assert_eq!(m.source, MatchSource::Explicit);
}

// ---------------------------------------------------------------------------
// skill_exists
// ---------------------------------------------------------------------------

#[tokio::test]
async fn skill_exists_checks_registry() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    assert!(router.skill_exists("review-pr").await);
    assert!(router.skill_exists("commit").await);
    assert!(!router.skill_exists("nonexistent").await);
}

// ---------------------------------------------------------------------------
// Suggestion formatting
// ---------------------------------------------------------------------------

#[tokio::test]
async fn format_suggestions_single() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = vec![SkillMatch::keyword("review-pr".to_string(), 0.8)];
    let suggestion = router.format_suggestions(&matches);

    assert!(suggestion.is_some());
    let text = suggestion.unwrap();
    assert!(text.contains("/review-pr"));
    assert!(text.contains("skill ")); // singular
}

#[tokio::test]
async fn format_suggestions_multiple() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = vec![
        SkillMatch::keyword("review-pr".to_string(), 0.85),
        SkillMatch::keyword("commit".to_string(), 0.75),
        SkillMatch::keyword("explain-code".to_string(), 0.65),
    ];
    let suggestion = router.format_suggestions(&matches);

    assert!(suggestion.is_some());
    let text = suggestion.unwrap();
    assert!(text.contains("/review-pr"));
    assert!(text.contains("/commit"));
    assert!(text.contains("/explain-code"));
    assert!(text.contains("skills")); // plural
}

#[tokio::test]
async fn format_suggestions_limits_to_three() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    let matches = vec![
        SkillMatch::keyword("a".to_string(), 0.9),
        SkillMatch::keyword("b".to_string(), 0.8),
        SkillMatch::keyword("c".to_string(), 0.7),
        SkillMatch::keyword("d".to_string(), 0.6),
    ];
    let suggestion = router.format_suggestions(&matches).unwrap();

    // Should contain first 3 but not the 4th
    assert!(suggestion.contains("/a"));
    assert!(suggestion.contains("/b"));
    assert!(suggestion.contains("/c"));
    assert!(!suggestion.contains("/d"));
}

#[tokio::test]
async fn format_suggestions_empty_returns_none() {
    let reg = populated_registry().await;
    let router = SkillRouter::new(reg);

    assert!(router.format_suggestions(&[]).is_none());
}
