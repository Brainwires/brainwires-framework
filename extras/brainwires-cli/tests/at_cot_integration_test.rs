//! AT-CoT Integration Tests
//!
//! Tests for Ambiguity Type-Chain of Thought integration with the
//! clarifying questions system.

use brainwires_cli::tui::question_parser::parse_response;
use brainwires_cli::types::question::{AmbiguityType, QuestionAnswerState};

#[test]
fn test_full_at_cot_flow() {
    // Simulate AI response with complete AT-CoT metadata
    let ai_response = r#"I've analyzed your request for implementing a cache with optimization.

<clarifying_questions>
{
  "ambiguity_analysis": {
    "predicted_types": ["semantic", "specify"],
    "reasoning": "The term 'cache' is ambiguous (could be LRU, LFU, TTL, Redis) and 'optimize' needs specification (speed vs memory vs hit rate)"
  },
  "questions": [
    {
      "id": "q1",
      "question": "Which cache type should we implement?",
      "header": "Cache Type",
      "ambiguity_type": "semantic",
      "multi_select": false,
      "options": [
        {"id": "lru", "label": "LRU cache", "description": "Least Recently Used eviction"},
        {"id": "lfu", "label": "LFU cache", "description": "Least Frequently Used eviction"},
        {"id": "ttl", "label": "TTL cache", "description": "Time-To-Live expiration"}
      ]
    },
    {
      "id": "q2",
      "question": "What optimization goal is most important?",
      "header": "Goal",
      "ambiguity_type": "specify",
      "multi_select": false,
      "options": [
        {"id": "speed", "label": "Lookup speed", "description": "Minimize access time"},
        {"id": "memory", "label": "Memory usage", "description": "Minimize footprint"},
        {"id": "hit_rate", "label": "Hit rate", "description": "Maximize effectiveness"}
      ]
    }
  ]
}
</clarifying_questions>"#;

    // Parse the response
    let result = parse_response(ai_response);

    // Verify content extraction
    assert_eq!(
        result.content,
        "I've analyzed your request for implementing a cache with optimization."
    );

    // Verify questions extracted
    assert!(result.questions.is_some());
    let block = result.questions.unwrap();

    // Verify AT-CoT ambiguity analysis
    assert!(block.ambiguity_analysis.is_some());
    let analysis = block.ambiguity_analysis.as_ref().unwrap();

    assert_eq!(analysis.predicted_types.len(), 2);
    assert!(analysis.predicted_types.contains(&AmbiguityType::Semantic));
    assert!(analysis.predicted_types.contains(&AmbiguityType::Specify));
    assert!(analysis.reasoning.contains("cache"));
    assert!(analysis.reasoning.contains("optimize"));

    // Verify questions have ambiguity types
    assert_eq!(block.questions.len(), 2);
    assert_eq!(
        block.questions[0].ambiguity_type,
        Some(AmbiguityType::Semantic)
    );
    assert_eq!(
        block.questions[1].ambiguity_type,
        Some(AmbiguityType::Specify)
    );

    // Verify question details
    assert_eq!(block.questions[0].id, "q1");
    assert_eq!(block.questions[0].header, "Cache Type");
    assert_eq!(block.questions[0].options.len(), 3);

    // Verify state can be created
    let state = QuestionAnswerState::new(&block);
    assert_eq!(state.current_question_idx, 0);
    assert_eq!(state.selected_options.len(), 2);
}

#[test]
fn test_backward_compatibility_without_at_cot() {
    // Old format without AT-CoT fields should still work
    let ai_response = r#"Here's my analysis.

<clarifying_questions>
{
  "questions": [
    {
      "id": "q1",
      "question": "Which authentication method should we use?",
      "header": "Auth",
      "multi_select": false,
      "options": [
        {"id": "jwt", "label": "JWT tokens", "description": "Stateless"},
        {"id": "session", "label": "Sessions", "description": "Traditional"}
      ]
    }
  ]
}
</clarifying_questions>"#;

    let result = parse_response(ai_response);
    assert!(result.questions.is_some());

    let block = result.questions.unwrap();

    // No AT-CoT metadata
    assert!(block.ambiguity_analysis.is_none());
    assert_eq!(block.questions.len(), 1);
    assert!(block.questions[0].ambiguity_type.is_none());

    // But questions still work normally
    assert_eq!(block.questions[0].id, "q1");
    assert_eq!(block.questions[0].options.len(), 2);
}

#[test]
fn test_semantic_ambiguity_example() {
    let ai_response = r#"I need clarification on the cache implementation.

<clarifying_questions>
{
  "ambiguity_analysis": {
    "predicted_types": ["semantic"],
    "reasoning": "The term 'cache' has multiple technical meanings - could be in-memory LRU, Redis distributed cache, or browser cache"
  },
  "questions": [
    {
      "id": "q1",
      "question": "Which cache implementation should we use?",
      "header": "Cache Type",
      "ambiguity_type": "semantic",
      "multi_select": false,
      "options": [
        {"id": "lru", "label": "LRU in-memory", "description": null},
        {"id": "redis", "label": "Redis", "description": null},
        {"id": "browser", "label": "Browser cache", "description": null}
      ]
    }
  ]
}
</clarifying_questions>"#;

    let result = parse_response(ai_response);
    let block = result.questions.unwrap();
    let analysis = block.ambiguity_analysis.unwrap();

    assert_eq!(analysis.predicted_types.len(), 1);
    assert!(analysis.predicted_types.contains(&AmbiguityType::Semantic));
    assert_eq!(
        block.questions[0].ambiguity_type,
        Some(AmbiguityType::Semantic)
    );
}

#[test]
fn test_generalize_ambiguity_example() {
    let ai_response = r#"Let me clarify the scope.

<clarifying_questions>
{
  "ambiguity_analysis": {
    "predicted_types": ["generalize"],
    "reasoning": "User mentions specific endpoint but error handling is likely needed across all endpoints"
  },
  "questions": [
    {
      "id": "q1",
      "question": "Should error handling be added to just the login endpoint, or all API endpoints?",
      "header": "Scope",
      "ambiguity_type": "generalize",
      "multi_select": false,
      "options": [
        {"id": "login", "label": "Login only", "description": null},
        {"id": "auth", "label": "Auth endpoints", "description": null},
        {"id": "all", "label": "All endpoints", "description": null}
      ]
    }
  ]
}
</clarifying_questions>"#;

    let result = parse_response(ai_response);
    let block = result.questions.unwrap();
    let analysis = block.ambiguity_analysis.unwrap();

    assert_eq!(analysis.predicted_types.len(), 1);
    assert!(
        analysis
            .predicted_types
            .contains(&AmbiguityType::Generalize)
    );
    assert_eq!(
        block.questions[0].ambiguity_type,
        Some(AmbiguityType::Generalize)
    );
}

#[test]
fn test_specify_ambiguity_example() {
    let ai_response = r#"I need more specific requirements.

<clarifying_questions>
{
  "ambiguity_analysis": {
    "predicted_types": ["specify"],
    "reasoning": "Goal is too broad - 'make faster' needs concrete constraints on which queries, what metrics"
  },
  "questions": [
    {
      "id": "q1",
      "question": "Which database queries need optimization?",
      "header": "Queries",
      "ambiguity_type": "specify",
      "multi_select": false,
      "options": [
        {"id": "profile", "label": "User profile", "description": null},
        {"id": "search", "label": "Search", "description": null},
        {"id": "all", "label": "All queries", "description": null}
      ]
    },
    {
      "id": "q2",
      "question": "What performance target?",
      "header": "Target",
      "ambiguity_type": "specify",
      "multi_select": false,
      "options": [
        {"id": "100ms", "label": "Under 100ms", "description": null},
        {"id": "50ms", "label": "Under 50ms", "description": null}
      ]
    }
  ]
}
</clarifying_questions>"#;

    let result = parse_response(ai_response);
    let block = result.questions.unwrap();
    let analysis = block.ambiguity_analysis.unwrap();

    assert_eq!(analysis.predicted_types.len(), 1);
    assert!(analysis.predicted_types.contains(&AmbiguityType::Specify));
    assert_eq!(block.questions.len(), 2);
    assert_eq!(
        block.questions[0].ambiguity_type,
        Some(AmbiguityType::Specify)
    );
    assert_eq!(
        block.questions[1].ambiguity_type,
        Some(AmbiguityType::Specify)
    );
}

#[test]
fn test_multiple_ambiguity_types() {
    let ai_response = r#"I need clarification on multiple aspects.

<clarifying_questions>
{
  "ambiguity_analysis": {
    "predicted_types": ["semantic", "generalize", "specify"],
    "reasoning": "Query has unclear term 'optimize' (semantic), mentions specific component that may need broader scope (generalize), and lacks concrete constraints (specify)"
  },
  "questions": [
    {
      "id": "q1",
      "question": "What do you mean by 'optimize'?",
      "header": "Meaning",
      "ambiguity_type": "semantic",
      "multi_select": false,
      "options": [
        {"id": "speed", "label": "Speed", "description": null},
        {"id": "memory", "label": "Memory", "description": null}
      ]
    },
    {
      "id": "q2",
      "question": "Should this apply to just the login component or all components?",
      "header": "Scope",
      "ambiguity_type": "generalize",
      "multi_select": false,
      "options": [
        {"id": "login", "label": "Login only", "description": null},
        {"id": "all", "label": "All", "description": null}
      ]
    },
    {
      "id": "q3",
      "question": "What specific metric should we target?",
      "header": "Metric",
      "ambiguity_type": "specify",
      "multi_select": false,
      "options": [
        {"id": "latency", "label": "Latency", "description": null},
        {"id": "throughput", "label": "Throughput", "description": null}
      ]
    }
  ]
}
</clarifying_questions>"#;

    let result = parse_response(ai_response);
    let block = result.questions.unwrap();
    let analysis = block.ambiguity_analysis.unwrap();

    // All three ambiguity types present
    assert_eq!(analysis.predicted_types.len(), 3);
    assert!(analysis.predicted_types.contains(&AmbiguityType::Semantic));
    assert!(
        analysis
            .predicted_types
            .contains(&AmbiguityType::Generalize)
    );
    assert!(analysis.predicted_types.contains(&AmbiguityType::Specify));

    // Each question has correct type
    assert_eq!(block.questions.len(), 3);
    assert_eq!(
        block.questions[0].ambiguity_type,
        Some(AmbiguityType::Semantic)
    );
    assert_eq!(
        block.questions[1].ambiguity_type,
        Some(AmbiguityType::Generalize)
    );
    assert_eq!(
        block.questions[2].ambiguity_type,
        Some(AmbiguityType::Specify)
    );
}

#[test]
fn test_partial_at_cot_metadata() {
    // Test case where ambiguity_analysis is present but questions don't have ambiguity_type
    let ai_response = r#"Here's my analysis.

<clarifying_questions>
{
  "ambiguity_analysis": {
    "predicted_types": ["semantic"],
    "reasoning": "Term 'cache' is ambiguous"
  },
  "questions": [
    {
      "id": "q1",
      "question": "Which cache type?",
      "header": "Cache",
      "multi_select": false,
      "options": [
        {"id": "lru", "label": "LRU", "description": null},
        {"id": "lfu", "label": "LFU", "description": null}
      ]
    }
  ]
}
</clarifying_questions>"#;

    let result = parse_response(ai_response);
    let block = result.questions.unwrap();

    // ambiguity_analysis present
    assert!(block.ambiguity_analysis.is_some());

    // But individual questions don't have ambiguity_type (still valid)
    assert!(block.questions[0].ambiguity_type.is_none());
}

#[test]
fn test_ambiguity_type_serialization() {
    use serde_json;

    // Test that AmbiguityType serializes as snake_case
    let semantic = AmbiguityType::Semantic;
    let generalize = AmbiguityType::Generalize;
    let specify = AmbiguityType::Specify;

    assert_eq!(serde_json::to_string(&semantic).unwrap(), "\"semantic\"");
    assert_eq!(
        serde_json::to_string(&generalize).unwrap(),
        "\"generalize\""
    );
    assert_eq!(serde_json::to_string(&specify).unwrap(), "\"specify\"");
}

#[test]
fn test_ambiguity_type_emoji() {
    assert_eq!(AmbiguityType::Semantic.to_emoji(), "🔍");
    assert_eq!(AmbiguityType::Generalize.to_emoji(), "📐");
    assert_eq!(AmbiguityType::Specify.to_emoji(), "🎯");
}

#[test]
fn test_ambiguity_type_display_name() {
    assert_eq!(AmbiguityType::Semantic.to_display_name(), "Semantic");
    assert_eq!(AmbiguityType::Generalize.to_display_name(), "Generalize");
    assert_eq!(AmbiguityType::Specify.to_display_name(), "Specify");
}
