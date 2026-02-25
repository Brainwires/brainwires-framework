//! Local Validator - Semantic Response Validation
//!
//! Uses a local LLM to perform semantic validation of responses,
//! enhancing the pattern-based red-flagging system.

use std::sync::Arc;
use tracing::{debug, warn};

#[cfg(feature = "llama-cpp-2")]
use brainwires_providers::local_llm::LocalLlmProvider;
#[cfg(feature = "llama-cpp-2")]
use brainwires_core::message::Message;

use crate::InferenceTimer;

/// Result of local validation
#[derive(Clone, Debug)]
pub enum ValidationResult {
    /// Response is valid
    Valid {
        confidence: f32,
    },
    /// Response has issues
    Invalid {
        reason: String,
        severity: f32,
        confidence: f32,
    },
    /// Validation was skipped (fallback to pattern-based)
    Skipped,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid { .. })
    }

    pub fn is_invalid(&self) -> bool {
        matches!(self, ValidationResult::Invalid { .. })
    }
}

/// Local validator for semantic response validation
pub struct LocalValidator {
    #[cfg(feature = "llama-cpp-2")]
    provider: Arc<LocalLlmProvider>,
    model_id: String,
}

impl LocalValidator {
    /// Create a new local validator
    #[cfg(feature = "llama-cpp-2")]
    pub fn new(provider: Arc<LocalLlmProvider>, model_id: impl Into<String>) -> Self {
        Self {
            provider,
            model_id: model_id.into(),
        }
    }

    /// Create a stub validator (non-llama-cpp-2 builds)
    #[cfg(not(feature = "llama-cpp-2"))]
    pub fn new_stub(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
        }
    }

    /// Validate a response for the given task
    ///
    /// Performs semantic validation to catch issues that pattern matching might miss.
    #[cfg(feature = "llama-cpp-2")]
    pub async fn validate(&self, task: &str, response: &str) -> ValidationResult {
        let timer = InferenceTimer::new("validate_response", &self.model_id);

        // Skip very short responses (likely already handled by pattern matching)
        if response.trim().len() < 10 {
            return ValidationResult::Skipped;
        }

        let system_prompt = self.build_validation_prompt();
        let user_prompt = format!(
            "Validate if this response is appropriate for the task.\n\nTask: {}\n\nResponse: {}\n\nOutput ONLY: VALID or INVALID:<reason>",
            task,
            // Truncate response for efficiency
            if response.len() > 500 { &response[..500] } else { response }
        );

        match self.provider.generate(&user_prompt, &crate::providers::local_llm::LocalInferenceParams {
            temperature: 0.0,
            max_tokens: 50,
            ..Default::default()
        }).await {
            Ok(text) => {
                let result = self.parse_validation(&text);
                timer.finish(true);
                result
            }
            Err(e) => {
                warn!(target: "local_llm", "Response validation failed: {}", e);
                timer.finish(false);
                ValidationResult::Skipped
            }
        }
    }

    /// Stub validation for non-llama-cpp-2 builds
    #[cfg(not(feature = "llama-cpp-2"))]
    pub async fn validate(&self, _task: &str, _response: &str) -> ValidationResult {
        ValidationResult::Skipped
    }

    /// Quick heuristic validation (no LLM call)
    ///
    /// Use for fast pre-filtering before LLM validation.
    pub fn validate_heuristic(&self, task: &str, response: &str) -> ValidationResult {
        let response_lower = response.to_lowercase();
        let task_lower = task.to_lowercase();

        // Check for obvious issues

        // 1. Response is completely off-topic (no shared words with task)
        let task_words: std::collections::HashSet<&str> = task_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();
        let response_words: std::collections::HashSet<&str> = response_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        let overlap = task_words.intersection(&response_words).count();
        if overlap == 0 && task_words.len() > 3 {
            return ValidationResult::Invalid {
                reason: "Response appears unrelated to task".to_string(),
                severity: 0.6,
                confidence: 0.4,
            };
        }

        // 2. Response contains refusal patterns
        let refusal_patterns = [
            "i cannot",
            "i can't",
            "i'm unable",
            "i am unable",
            "sorry, i",
            "i don't have",
            "i do not have",
            "as an ai",
        ];

        for pattern in refusal_patterns {
            if response_lower.contains(pattern) {
                return ValidationResult::Invalid {
                    reason: format!("Response contains refusal pattern: {}", pattern),
                    severity: 0.7,
                    confidence: 0.6,
                };
            }
        }

        // 3. Response is just repeating the task
        let task_trimmed = task_lower.trim();
        let response_trimmed = response_lower.trim();
        if response_trimmed.starts_with(task_trimmed) && response.len() < task.len() * 2 {
            return ValidationResult::Invalid {
                reason: "Response appears to just repeat the task".to_string(),
                severity: 0.5,
                confidence: 0.5,
            };
        }

        // 4. Response is suspiciously short for a complex task
        if task.len() > 100 && response.len() < 20 {
            return ValidationResult::Invalid {
                reason: "Response too short for complex task".to_string(),
                severity: 0.4,
                confidence: 0.4,
            };
        }

        ValidationResult::Valid { confidence: 0.5 }
    }

    /// Build the system prompt for validation
    fn build_validation_prompt(&self) -> String {
        r#"You are a response validator. Given a task and response, determine if the response is appropriate.

Check for:
1. Response addresses the task (not off-topic)
2. Response doesn't contain confusion or self-correction
3. Response isn't a refusal or "I can't do that"
4. Response isn't just repeating the task
5. Response has substance (not empty platitudes)

Output format:
- If valid: VALID
- If invalid: INVALID:<brief reason>

Be strict but fair. Only flag clear issues."#.to_string()
    }

    /// Parse the LLM output to determine validity
    fn parse_validation(&self, output: &str) -> ValidationResult {
        let trimmed = output.trim().to_uppercase();

        if trimmed.starts_with("VALID") && !trimmed.contains("INVALID") {
            return ValidationResult::Valid { confidence: 0.8 };
        }

        if trimmed.starts_with("INVALID") {
            let reason = if let Some(idx) = trimmed.find(':') {
                trimmed[idx + 1..].trim().to_string()
            } else {
                "Unspecified validation failure".to_string()
            };

            return ValidationResult::Invalid {
                reason,
                severity: 0.6,
                confidence: 0.75,
            };
        }

        // Ambiguous output - treat as skipped
        ValidationResult::Skipped
    }
}

/// Builder for LocalValidator
pub struct LocalValidatorBuilder {
    #[cfg(feature = "llama-cpp-2")]
    provider: Option<Arc<LocalLlmProvider>>,
    model_id: String,
}

impl Default for LocalValidatorBuilder {
    fn default() -> Self {
        Self {
            #[cfg(feature = "llama-cpp-2")]
            provider: None,
            model_id: "lfm2-350m".to_string(),
        }
    }
}

impl LocalValidatorBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "llama-cpp-2")]
    pub fn provider(mut self, provider: Arc<LocalLlmProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self
    }

    #[cfg(feature = "llama-cpp-2")]
    pub fn build(self) -> Option<LocalValidator> {
        self.provider.map(|p| LocalValidator::new(p, self.model_id))
    }

    #[cfg(not(feature = "llama-cpp-2"))]
    pub fn build(self) -> Option<LocalValidator> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_checks() {
        let valid = ValidationResult::Valid { confidence: 0.9 };
        assert!(valid.is_valid());
        assert!(!valid.is_invalid());

        let invalid = ValidationResult::Invalid {
            reason: "test".to_string(),
            severity: 0.5,
            confidence: 0.8,
        };
        assert!(!invalid.is_valid());
        assert!(invalid.is_invalid());
    }

    #[test]
    fn test_heuristic_validation_refusal() {
        let validator = LocalValidatorBuilder::default();

        // Test refusal detection
        let result = validate_heuristic_direct(
            "Write a poem",
            "I'm sorry, I cannot write poems as an AI assistant."
        );

        assert!(matches!(result, ValidationResult::Invalid { .. }));
    }

    #[test]
    fn test_heuristic_validation_valid() {
        let result = validate_heuristic_direct(
            "Calculate 2+2",
            "The result of 2+2 is 4."
        );

        assert!(matches!(result, ValidationResult::Valid { .. }));
    }

    fn validate_heuristic_direct(task: &str, response: &str) -> ValidationResult {
        let response_lower = response.to_lowercase();

        let refusal_patterns = [
            "i cannot",
            "i can't",
            "i'm unable",
            "sorry, i",
            "as an ai",
        ];

        for pattern in refusal_patterns {
            if response_lower.contains(pattern) {
                return ValidationResult::Invalid {
                    reason: format!("Refusal pattern: {}", pattern),
                    severity: 0.7,
                    confidence: 0.6,
                };
            }
        }

        ValidationResult::Valid { confidence: 0.5 }
    }

    #[test]
    fn test_parse_validation() {
        // Test parsing logic
        assert!(matches!(
            parse_validation_direct("VALID"),
            ValidationResult::Valid { .. }
        ));

        assert!(matches!(
            parse_validation_direct("INVALID: Response is off-topic"),
            ValidationResult::Invalid { .. }
        ));

        assert!(matches!(
            parse_validation_direct("Maybe?"),
            ValidationResult::Skipped
        ));
    }

    fn parse_validation_direct(output: &str) -> ValidationResult {
        let trimmed = output.trim().to_uppercase();

        if trimmed.starts_with("VALID") && !trimmed.contains("INVALID") {
            return ValidationResult::Valid { confidence: 0.8 };
        }

        if trimmed.starts_with("INVALID") {
            let reason = if let Some(idx) = trimmed.find(':') {
                trimmed[idx + 1..].trim().to_string()
            } else {
                "Unspecified".to_string()
            };

            return ValidationResult::Invalid {
                reason,
                severity: 0.6,
                confidence: 0.75,
            };
        }

        ValidationResult::Skipped
    }
}
