//! Question Parser
//!
//! Parses AI responses to extract clarifying questions and formats user answers.
//! Includes AT-CoT (Ambiguity Type-Chain of Thought) metadata extraction.

use crate::types::question::{AmbiguityAnalysis, ClarifyingQuestion, QuestionAnswerState, QuestionBlock};
use regex::Regex;
use tracing::{debug, warn};

/// Result of parsing an AI response
pub struct ParsedResponse {
    /// The clean content with question block removed
    pub content: String,
    /// Extracted questions, if any
    pub questions: Option<QuestionBlock>,
}

/// Parse an AI response to extract clarifying questions
///
/// Looks for `<clarifying_questions>JSON</clarifying_questions>` blocks
/// at the end of the response and extracts them.
pub fn parse_response(response: &str) -> ParsedResponse {
    // Regex to find the clarifying_questions block
    let re = Regex::new(r"<clarifying_questions>\s*([\s\S]*?)\s*</clarifying_questions>")
        .expect("Invalid regex");

    if let Some(captures) = re.captures(response) {
        let json_str = captures.get(1).map(|m| m.as_str()).unwrap_or("");

        // Try to parse the JSON
        match serde_json::from_str::<QuestionBlock>(json_str) {
            Ok(block) => {
                // Validate the questions (including AT-CoT metadata if present)
                if validate_question_block(&block) {
                    // Log AT-CoT metadata for observability
                    if let Some(ref analysis) = block.ambiguity_analysis {
                        debug!("AT-CoT predicted types: {:?}", analysis.predicted_types);
                        debug!("AT-CoT reasoning: {}", analysis.reasoning);

                        // Validate that question ambiguity types match predicted types
                        for question in &block.questions {
                            if let Some(ref amb_type) = question.ambiguity_type {
                                if !analysis.predicted_types.contains(amb_type) {
                                    warn!(
                                        "Question '{}' has ambiguity type {:?} not in predicted types: {:?}",
                                        question.id, amb_type, analysis.predicted_types
                                    );
                                }
                                debug!(
                                    "Question '{}' ({}) is type: {:?}",
                                    question.id, question.question, amb_type
                                );
                            }
                        }
                    } else {
                        debug!("No AT-CoT metadata in question block (backward compatible mode)");
                    }

                    // Remove the block from the content
                    let clean_content = re.replace(response, "").trim().to_string();
                    return ParsedResponse {
                        content: clean_content,
                        questions: Some(block),
                    };
                }
            }
            Err(e) => {
                // Log parse error but continue without questions
                eprintln!("Failed to parse clarifying questions: {}", e);
            }
        }
    }

    // No valid questions found, return original content
    ParsedResponse {
        content: response.to_string(),
        questions: None,
    }
}

/// Validate a question block meets our requirements
fn validate_question_block(block: &QuestionBlock) -> bool {
    // Must have at least one question, max 4
    if block.questions.is_empty() || block.questions.len() > 4 {
        return false;
    }

    // Validate AT-CoT ambiguity analysis if present
    if let Some(ref analysis) = block.ambiguity_analysis {
        if !validate_ambiguity_analysis(analysis) {
            warn!("Invalid ambiguity analysis in question block");
            // Don't fail validation - AT-CoT is optional
        }
    }

    for q in &block.questions {
        // Must have 2-4 options
        if q.options.len() < 2 || q.options.len() > 4 {
            return false;
        }

        // Header should be <= 12 chars
        if q.header.len() > 12 {
            return false;
        }

        // Each option label should be 1-5 words (rough check)
        for opt in &q.options {
            let word_count = opt.label.split_whitespace().count();
            if word_count == 0 || word_count > 5 {
                return false;
            }
        }
    }

    true
}

/// Validate ambiguity analysis metadata
fn validate_ambiguity_analysis(analysis: &AmbiguityAnalysis) -> bool {
    // Must have at least one predicted type
    if analysis.predicted_types.is_empty() {
        warn!("Ambiguity analysis has empty predicted_types");
        return false;
    }

    // Reasoning should not be empty
    if analysis.reasoning.trim().is_empty() {
        warn!("Ambiguity analysis has empty reasoning");
        return false;
    }

    true
}

/// Format user answers as natural language for sending back to the AI
pub fn format_answers_natural(questions: &QuestionBlock, state: &QuestionAnswerState) -> String {
    let mut parts = Vec::new();

    for (q_idx, question) in questions.questions.iter().enumerate() {
        let mut answer_parts = Vec::new();

        // Collect selected options
        if let Some(selected) = state.selected_options.get(q_idx) {
            for (opt_idx, &is_selected) in selected.iter().enumerate() {
                if is_selected {
                    if let Some(opt) = question.options.get(opt_idx) {
                        answer_parts.push(opt.label.clone());
                    }
                }
            }
        }

        // Check for "Other" selection
        let other_selected = state.other_selected.get(q_idx).copied().unwrap_or(false);
        let other_text = state
            .other_text
            .get(q_idx)
            .map(|s| s.trim())
            .unwrap_or("");

        if other_selected && !other_text.is_empty() {
            answer_parts.push(other_text.to_string());
        }

        // Format this question's answer
        if !answer_parts.is_empty() {
            if answer_parts.len() == 1 {
                parts.push(format!(
                    "For \"{}\": {}",
                    truncate_question(&question.question),
                    answer_parts[0]
                ));
            } else {
                let last = answer_parts.pop().unwrap();
                parts.push(format!(
                    "For \"{}\": {} and {}",
                    truncate_question(&question.question),
                    answer_parts.join(", "),
                    last
                ));
            }
        }
    }

    if parts.is_empty() {
        "I'd like to proceed with your default recommendations.".to_string()
    } else {
        parts.join("\n\n")
    }
}

/// Truncate a question for display in the answer
fn truncate_question(question: &str) -> &str {
    if question.len() <= 50 {
        question
    } else {
        &question[..47]
    }
}

/// Generate message when user declines to answer questions
pub fn format_declined_message() -> String {
    "I'd prefer to skip these questions and have you proceed with your best judgment.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::question::QuestionOption;

    #[test]
    fn test_parse_response_with_questions() {
        let response = r#"Here's my analysis of the problem.

<clarifying_questions>
{
  "questions": [
    {
      "id": "q1",
      "question": "Which auth method?",
      "header": "Auth",
      "multi_select": false,
      "options": [
        {"id": "a", "label": "JWT tokens", "description": "Stateless"},
        {"id": "b", "label": "Sessions", "description": null}
      ]
    }
  ]
}
</clarifying_questions>"#;

        let result = parse_response(response);
        assert!(result.questions.is_some());
        assert_eq!(
            result.content,
            "Here's my analysis of the problem."
        );

        let questions = result.questions.unwrap();
        assert_eq!(questions.questions.len(), 1);
        assert_eq!(questions.questions[0].id, "q1");
    }

    #[test]
    fn test_parse_response_without_questions() {
        let response = "Just a regular response without questions.";
        let result = parse_response(response);
        assert!(result.questions.is_none());
        assert_eq!(result.content, response);
    }

    #[test]
    fn test_parse_invalid_json() {
        let response = r#"Some text
<clarifying_questions>
{ invalid json }
</clarifying_questions>"#;

        let result = parse_response(response);
        assert!(result.questions.is_none());
    }

    #[test]
    fn test_format_answers_single_select() {
        let questions = QuestionBlock {
            ambiguity_analysis: None,
            questions: vec![ClarifyingQuestion {
                id: "q1".to_string(),
                question: "Which auth method?".to_string(),
                header: "Auth".to_string(),
                multi_select: false,
                ambiguity_type: None,
                options: vec![
                    QuestionOption {
                        id: "a".to_string(),
                        label: "JWT tokens".to_string(),
                        description: None,
                    },
                    QuestionOption {
                        id: "b".to_string(),
                        label: "Sessions".to_string(),
                        description: None,
                    },
                ],
            }],
        };

        let mut state = QuestionAnswerState::new(&questions);
        state.selected_options[0][0] = true; // Select JWT

        let formatted = format_answers_natural(&questions, &state);
        assert!(formatted.contains("JWT tokens"));
        assert!(formatted.contains("Which auth method?"));
    }

    #[test]
    fn test_format_answers_multi_select() {
        let questions = QuestionBlock {
            ambiguity_analysis: None,
            questions: vec![ClarifyingQuestion {
                id: "q1".to_string(),
                question: "Which features?".to_string(),
                header: "Features".to_string(),
                multi_select: true,
                ambiguity_type: None,
                options: vec![
                    QuestionOption {
                        id: "a".to_string(),
                        label: "Logging".to_string(),
                        description: None,
                    },
                    QuestionOption {
                        id: "b".to_string(),
                        label: "Caching".to_string(),
                        description: None,
                    },
                ],
            }],
        };

        let mut state = QuestionAnswerState::new(&questions);
        state.selected_options[0][0] = true;
        state.selected_options[0][1] = true;

        let formatted = format_answers_natural(&questions, &state);
        assert!(formatted.contains("Logging"));
        assert!(formatted.contains("Caching"));
        assert!(formatted.contains(" and "));
    }

    #[test]
    fn test_format_answers_with_other() {
        let questions = QuestionBlock {
            ambiguity_analysis: None,
            questions: vec![ClarifyingQuestion {
                id: "q1".to_string(),
                question: "Which auth method?".to_string(),
                header: "Auth".to_string(),
                multi_select: false,
                ambiguity_type: None,
                options: vec![
                    QuestionOption {
                        id: "a".to_string(),
                        label: "JWT".to_string(),
                        description: None,
                    },
                ],
            }],
        };

        let mut state = QuestionAnswerState::new(&questions);
        state.other_selected[0] = true;
        state.other_text[0] = "Custom OAuth2 implementation".to_string();

        let formatted = format_answers_natural(&questions, &state);
        assert!(formatted.contains("Custom OAuth2 implementation"));
    }

    #[test]
    fn test_validate_question_block() {
        // Valid block
        let valid = QuestionBlock {
            ambiguity_analysis: None,
            questions: vec![ClarifyingQuestion {
                id: "q1".to_string(),
                question: "Test?".to_string(),
                header: "Test".to_string(),
                multi_select: false,
                ambiguity_type: None,
                options: vec![
                    QuestionOption {
                        id: "a".to_string(),
                        label: "Option A".to_string(),
                        description: None,
                    },
                    QuestionOption {
                        id: "b".to_string(),
                        label: "Option B".to_string(),
                        description: None,
                    },
                ],
            }],
        };
        assert!(validate_question_block(&valid));

        // Too many questions
        let too_many = QuestionBlock {
            ambiguity_analysis: None,
            questions: vec![
                valid.questions[0].clone(),
                valid.questions[0].clone(),
                valid.questions[0].clone(),
                valid.questions[0].clone(),
                valid.questions[0].clone(), // 5th
            ],
        };
        assert!(!validate_question_block(&too_many));

        // Header too long
        let long_header = QuestionBlock {
            ambiguity_analysis: None,
            questions: vec![ClarifyingQuestion {
                id: "q1".to_string(),
                question: "Test?".to_string(),
                header: "This header is way too long".to_string(),
                multi_select: false,
                ambiguity_type: None,
                options: vec![
                    QuestionOption {
                        id: "a".to_string(),
                        label: "A".to_string(),
                        description: None,
                    },
                    QuestionOption {
                        id: "b".to_string(),
                        label: "B".to_string(),
                        description: None,
                    },
                ],
            }],
        };
        assert!(!validate_question_block(&long_header));
    }

    #[test]
    fn test_parse_at_cot_metadata() {
        use crate::types::question::AmbiguityType;

        let response = r#"Here's my analysis.

<clarifying_questions>
{
  "ambiguity_analysis": {
    "predicted_types": ["semantic", "specify"],
    "reasoning": "Query has unclear term 'cache' and broad goal 'optimize'"
  },
  "questions": [
    {
      "id": "q1",
      "question": "Which cache type?",
      "header": "Cache",
      "ambiguity_type": "semantic",
      "multi_select": false,
      "options": [
        {"id": "a", "label": "LRU", "description": "Least Recently Used"},
        {"id": "b", "label": "LFU", "description": "Least Frequently Used"}
      ]
    }
  ]
}
</clarifying_questions>"#;

        let result = parse_response(response);
        assert!(result.questions.is_some());

        let block = result.questions.unwrap();
        assert!(block.ambiguity_analysis.is_some());

        let analysis = block.ambiguity_analysis.unwrap();
        assert_eq!(analysis.predicted_types.len(), 2);
        assert!(analysis.predicted_types.contains(&AmbiguityType::Semantic));
        assert!(analysis.predicted_types.contains(&AmbiguityType::Specify));
        assert!(analysis.reasoning.contains("cache"));

        assert_eq!(block.questions.len(), 1);
        assert_eq!(block.questions[0].ambiguity_type, Some(AmbiguityType::Semantic));
    }

    #[test]
    fn test_backward_compatible_without_at_cot() {
        // Old format without AT-CoT fields should still work
        let response = r#"Analysis.

<clarifying_questions>
{
  "questions": [
    {
      "id": "q1",
      "question": "Which option?",
      "header": "Option",
      "multi_select": false,
      "options": [
        {"id": "a", "label": "Option A", "description": null},
        {"id": "b", "label": "Option B", "description": null}
      ]
    }
  ]
}
</clarifying_questions>"#;

        let result = parse_response(response);
        assert!(result.questions.is_some());

        let block = result.questions.unwrap();
        assert!(block.ambiguity_analysis.is_none());
        assert_eq!(block.questions.len(), 1);
        assert!(block.questions[0].ambiguity_type.is_none());
    }

    #[test]
    fn test_validate_ambiguity_analysis() {
        use crate::types::question::{AmbiguityAnalysis, AmbiguityType};

        // Valid analysis
        let valid = AmbiguityAnalysis {
            predicted_types: vec![AmbiguityType::Semantic],
            reasoning: "Valid reasoning here".to_string(),
        };
        assert!(validate_ambiguity_analysis(&valid));

        // Empty predicted_types
        let empty_types = AmbiguityAnalysis {
            predicted_types: vec![],
            reasoning: "Valid reasoning".to_string(),
        };
        assert!(!validate_ambiguity_analysis(&empty_types));

        // Empty reasoning
        let empty_reasoning = AmbiguityAnalysis {
            predicted_types: vec![AmbiguityType::Specify],
            reasoning: "".to_string(),
        };
        assert!(!validate_ambiguity_analysis(&empty_reasoning));
    }
}
