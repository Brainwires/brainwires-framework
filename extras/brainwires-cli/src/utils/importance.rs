//! Importance Scoring for Messages

use regex::Regex;
use std::collections::HashSet;

/// Context for calculating importance
#[derive(Debug, Default)]
pub struct ImportanceContext {
    pub forward_references: usize,
    pub age_seconds: f64,
    pub user_marked_important: bool,
    pub future_entities: HashSet<String>,
}

/// Result of importance calculation
#[derive(Debug, Clone)]
pub struct ImportanceResult {
    pub score: f32,
    pub entity_score: f32,
    pub code_score: f32,
    pub decision_score: f32,
    pub reference_score: f32,
    pub recency_score: f32,
    pub entities_found: Vec<String>,
}

/// Calculate importance score for a message (0.0 to 1.0)
pub fn calculate_importance(content: &str, context: &ImportanceContext) -> ImportanceResult {
    let mut score = 0.0f32;

    let entities = extract_entities(content);
    let entity_score = (entities.len() as f32 * 0.08).min(0.3);
    score += entity_score;

    let code_score = if contains_code(content) { 0.25 } else { 0.0 };
    score += code_score;

    let decision_score = if contains_decision_language(content) { 0.2 } else { 0.0 };
    score += decision_score;

    let reference_score = (context.forward_references as f32 * 0.1).min(0.25);
    score += reference_score;

    if context.user_marked_important {
        score += 0.3;
    }

    let future_overlap = entities.iter().filter(|e| context.future_entities.contains(*e)).count();
    score += (future_overlap as f32 * 0.05).min(0.15);

    let age_hours = context.age_seconds / 3600.0;
    let recency_score = (-0.005 * age_hours as f32).exp();
    score *= 0.5 + (recency_score * 0.5);

    ImportanceResult {
        score: score.clamp(0.0, 1.0),
        entity_score,
        code_score,
        decision_score,
        reference_score,
        recency_score,
        entities_found: entities,
    }
}

/// Quick importance check without full analysis
pub fn quick_importance(content: &str) -> f32 {
    let mut score = 0.0f32;
    if contains_code(content) { score += 0.3; }
    if contains_decision_language(content) { score += 0.2; }
    let entity_count = count_entities_fast(content);
    score += (entity_count as f32 * 0.05).min(0.2);
    score.clamp(0.0, 1.0)
}

/// Extract named entities from content
pub fn extract_entities(content: &str) -> Vec<String> {
    let mut entities = Vec::new();

    if let Ok(func_regex) = Regex::new(r"\b(fn|function|def)\s+(\w+)") {
        for cap in func_regex.captures_iter(content) {
            if let Some(m) = cap.get(2) {
                entities.push(m.as_str().to_string());
            }
        }
    }

    if let Ok(var_regex) = Regex::new(r"\b(let|const|var|mut)\s+(\w+)") {
        for cap in var_regex.captures_iter(content) {
            if let Some(m) = cap.get(2) {
                entities.push(m.as_str().to_string());
            }
        }
    }

    if let Ok(type_regex) = Regex::new(r"\b(class|struct|type|interface|enum)\s+(\w+)") {
        for cap in type_regex.captures_iter(content) {
            if let Some(m) = cap.get(2) {
                entities.push(m.as_str().to_string());
            }
        }
    }

    entities.sort();
    entities.dedup();
    entities
}

fn count_entities_fast(content: &str) -> usize {
    let mut count = 0;
    count += content.matches("fn ").count();
    count += content.matches("function ").count();
    count += content.matches("def ").count();
    count
}

/// Check if content contains code
pub fn contains_code(content: &str) -> bool {
    let code_patterns = [
        "fn ", "pub fn", "async fn", "impl ", "struct ", "enum ",
        "function ", "const ", "let ", "var ",
        "def ", "class ", "import ", "from ",
    ];
    code_patterns.iter().any(|p| content.contains(p))
}

/// Check if content contains decision language
pub fn contains_decision_language(content: &str) -> bool {
    let lower = content.to_lowercase();
    let patterns = [
        "we decided", "the solution", "the approach", "i recommend",
        "we should", "we need to", "conclusion", "in summary",
    ];
    patterns.iter().any(|p| lower.contains(p))
}

/// Check if content contains question language
pub fn contains_question(content: &str) -> bool {
    content.contains('?')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_entities_functions() {
        let content = "fn calculate and function process";
        let entities = extract_entities(content);
        assert!(entities.contains(&"calculate".to_string()));
    }

    #[test]
    fn test_contains_code_keywords() {
        assert!(contains_code("fn test()"));
        assert!(!contains_code("just text"));
    }

    #[test]
    fn test_contains_decision_language() {
        assert!(contains_decision_language("We decided on X"));
        assert!(!contains_decision_language("Hello world"));
    }

    #[test]
    fn test_calculate_importance() {
        let content = "fn important() code here";
        let context = ImportanceContext::default();
        let result = calculate_importance(content, &context);
        assert!(result.score > 0.0);
    }

    #[test]
    fn test_quick_importance() {
        let code = "fn test()";
        let text = "hello";
        assert!(quick_importance(code) > quick_importance(text));
    }
}
