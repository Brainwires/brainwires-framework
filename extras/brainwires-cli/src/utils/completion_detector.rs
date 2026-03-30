//! Task Completion Detection from AI Responses
//!
//! Detects when AI reports completing a task and extracts task references.
//! Uses conservative detection - only explicit signals like checkmarks,
//! "[DONE]", "completed" markers, etc.

use crate::types::agent::Task;
use regex::Regex;

/// Detects task completions in AI responses
pub struct CompletionDetector;

/// Result of completion detection
#[derive(Debug, Clone)]
pub struct CompletionMatch {
    /// Task ID that was matched
    pub task_id: String,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// The pattern that matched
    pub pattern: String,
    /// Optional summary extracted from the response
    pub summary: Option<String>,
}

impl CompletionDetector {
    /// Detect completed tasks from an AI response
    ///
    /// Returns task IDs that appear to have been completed based on explicit signals.
    /// Uses conservative matching - only explicit completion markers are recognized.
    pub fn detect_completed_tasks(response: &str, active_tasks: &[Task]) -> Vec<CompletionMatch> {
        let mut matches = Vec::new();

        // Skip if no active tasks
        if active_tasks.is_empty() {
            return matches;
        }

        // Pattern 1: Checkmark symbols (✓, ✅, ☑)
        let checkmark_pattern = Self::detect_checkmark_completions(response, active_tasks);
        matches.extend(checkmark_pattern);

        // Pattern 2: "[DONE]", "[COMPLETE]", "[COMPLETED]" markers
        let marker_pattern = Self::detect_marker_completions(response, active_tasks);
        matches.extend(marker_pattern);

        // Pattern 3: "completed step/task N" or "completed: X"
        let explicit_pattern = Self::detect_explicit_completions(response, active_tasks);
        matches.extend(explicit_pattern);

        // Pattern 4: "Task X is now complete" style
        let sentence_pattern = Self::detect_sentence_completions(response, active_tasks);
        matches.extend(sentence_pattern);

        // Deduplicate by task ID (keep highest confidence)
        Self::deduplicate_matches(matches)
    }

    /// Detect checkmark symbols followed by task references
    fn detect_checkmark_completions(response: &str, tasks: &[Task]) -> Vec<CompletionMatch> {
        let mut matches = Vec::new();

        // Look for checkmarks followed by task description text
        let checkmark_re = Regex::new(r"[✓✅☑]\s*(.{10,100})").unwrap();

        for cap in checkmark_re.captures_iter(response) {
            if let Some(text) = cap.get(1) {
                let text_str = text.as_str().to_lowercase();

                // Try to match against task descriptions
                for task in tasks {
                    if Self::text_matches_task(&text_str, task) {
                        matches.push(CompletionMatch {
                            task_id: task.id.clone(),
                            confidence: 0.9,
                            pattern: "checkmark".to_string(),
                            summary: Some(text.as_str().trim().to_string()),
                        });
                        break;
                    }
                }
            }
        }

        matches
    }

    /// Detect [DONE], [COMPLETE], [COMPLETED] markers
    fn detect_marker_completions(response: &str, tasks: &[Task]) -> Vec<CompletionMatch> {
        let mut matches = Vec::new();

        // Look for markers followed by or preceded by task references
        let marker_re = Regex::new(r"(?i)\[(DONE|COMPLETE|COMPLETED)\][\s:]*(.{0,100})").unwrap();

        for cap in marker_re.captures_iter(response) {
            if let Some(text) = cap.get(2) {
                let text_str = text.as_str().to_lowercase();

                for task in tasks {
                    if Self::text_matches_task(&text_str, task) {
                        matches.push(CompletionMatch {
                            task_id: task.id.clone(),
                            confidence: 0.95,
                            pattern: "marker".to_string(),
                            summary: Some(text.as_str().trim().to_string()),
                        });
                        break;
                    }
                }
            }
        }

        // Also check for markers at line end (task text before marker)
        let reverse_re = Regex::new(r"(.{10,100})\s*\[(DONE|COMPLETE|COMPLETED)\]").unwrap();

        for cap in reverse_re.captures_iter(response) {
            if let Some(text) = cap.get(1) {
                let text_str = text.as_str().to_lowercase();

                for task in tasks {
                    if Self::text_matches_task(&text_str, task) {
                        // Check if we already matched this task
                        if !matches.iter().any(|m| m.task_id == task.id) {
                            matches.push(CompletionMatch {
                                task_id: task.id.clone(),
                                confidence: 0.95,
                                pattern: "marker".to_string(),
                                summary: Some(text.as_str().trim().to_string()),
                            });
                            break;
                        }
                    }
                }
            }
        }

        matches
    }

    /// Detect explicit completion phrases like "completed step 1", "completed: setup database"
    fn detect_explicit_completions(response: &str, tasks: &[Task]) -> Vec<CompletionMatch> {
        let mut matches = Vec::new();

        // "completed step N", "completed task N"
        let step_re = Regex::new(r"(?i)completed\s+(step|task)\s*#?(\d+)").unwrap();
        for cap in step_re.captures_iter(response) {
            if let Some(num_match) = cap.get(2) {
                if let Ok(step_num) = num_match.as_str().parse::<usize>() {
                    // Match by position in task list
                    if step_num > 0 && step_num <= tasks.len() {
                        let task = &tasks[step_num - 1];
                        matches.push(CompletionMatch {
                            task_id: task.id.clone(),
                            confidence: 0.85,
                            pattern: "step_number".to_string(),
                            summary: Some(format!("Step {} completed", step_num)),
                        });
                    }
                }
            }
        }

        // "completed: <text>" or "completed the <text>"
        let completed_re = Regex::new(r"(?i)completed[:.]?\s+(?:the\s+)?(.{5,80})").unwrap();
        for cap in completed_re.captures_iter(response) {
            if let Some(text) = cap.get(1) {
                let text_str = text.as_str().to_lowercase();

                for task in tasks {
                    if Self::text_matches_task(&text_str, task) {
                        if !matches.iter().any(|m| m.task_id == task.id) {
                            matches.push(CompletionMatch {
                                task_id: task.id.clone(),
                                confidence: 0.8,
                                pattern: "explicit".to_string(),
                                summary: Some(text.as_str().trim().to_string()),
                            });
                            break;
                        }
                    }
                }
            }
        }

        matches
    }

    /// Detect sentence-style completions like "Task X is now complete"
    fn detect_sentence_completions(response: &str, tasks: &[Task]) -> Vec<CompletionMatch> {
        let mut matches = Vec::new();

        // "X is now complete", "X has been completed", "finished X"
        let patterns = [
            r"(?i)(.{5,60})\s+is\s+now\s+complete",
            r"(?i)(.{5,60})\s+has\s+been\s+completed",
            r"(?i)finished\s+(.{5,60})",
            r"(?i)done\s+with\s+(.{5,60})",
        ];

        for pattern in &patterns {
            let re = Regex::new(pattern).unwrap();
            for cap in re.captures_iter(response) {
                if let Some(text) = cap.get(1) {
                    let text_str = text.as_str().to_lowercase();

                    for task in tasks {
                        if Self::text_matches_task(&text_str, task) {
                            if !matches.iter().any(|m: &CompletionMatch| m.task_id == task.id) {
                                matches.push(CompletionMatch {
                                    task_id: task.id.clone(),
                                    confidence: 0.75,
                                    pattern: "sentence".to_string(),
                                    summary: Some(text.as_str().trim().to_string()),
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        matches
    }

    /// Check if text matches a task description
    fn text_matches_task(text: &str, task: &Task) -> bool {
        let task_desc = task.description.to_lowercase();
        let text_lower = text.to_lowercase();

        // Exact substring match
        if text_lower.contains(&task_desc) || task_desc.contains(&text_lower) {
            return true;
        }

        // Check for significant word overlap (at least 50% of task words)
        let task_words: Vec<&str> = task_desc
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        let text_words: Vec<&str> = text_lower
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();

        if task_words.is_empty() {
            return false;
        }

        let matching_words = task_words
            .iter()
            .filter(|w| text_words.contains(w))
            .count();

        let match_ratio = matching_words as f32 / task_words.len() as f32;

        // Require more than 50% word match, or 100% for very short descriptions
        // This avoids false positives when tasks share common words
        if task_words.len() <= 2 {
            // For 1-2 word tasks, require all words to match
            matching_words == task_words.len()
        } else {
            // For longer tasks, require >50% word match
            match_ratio > 0.5 || matching_words >= 3
        }
    }

    /// Deduplicate matches, keeping highest confidence for each task
    fn deduplicate_matches(matches: Vec<CompletionMatch>) -> Vec<CompletionMatch> {
        use std::collections::HashMap;

        let mut best_matches: HashMap<String, CompletionMatch> = HashMap::new();

        for m in matches {
            let entry = best_matches.entry(m.task_id.clone()).or_insert(m.clone());
            if m.confidence > entry.confidence {
                *entry = m;
            }
        }

        best_matches.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::agent::TaskStatus;

    fn create_test_task(id: &str, description: &str) -> Task {
        Task {
            id: id.to_string(),
            description: description.to_string(),
            status: TaskStatus::InProgress,
            plan_id: None,
            parent_id: None,
            children: vec![],
            depends_on: vec![],
            priority: crate::types::agent::TaskPriority::Normal,
            assigned_to: None,
            iterations: 0,
            summary: None,
            created_at: 0,
            updated_at: 0,
            started_at: Some(0),
            completed_at: None,
        }
    }

    #[test]
    fn test_detect_checkmark() {
        let tasks = vec![
            create_test_task("task-1", "Set up database schema"),
            create_test_task("task-2", "Implement user authentication"),
        ];

        let response = "✓ Set up database schema\nNow working on authentication...";
        let matches = CompletionDetector::detect_completed_tasks(response, &tasks);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].task_id, "task-1");
        assert_eq!(matches[0].pattern, "checkmark");
    }

    #[test]
    fn test_detect_done_marker() {
        let tasks = vec![
            create_test_task("task-1", "Create API endpoints"),
        ];

        let response = "[DONE] Create API endpoints for user management";
        let matches = CompletionDetector::detect_completed_tasks(response, &tasks);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].task_id, "task-1");
        assert_eq!(matches[0].pattern, "marker");
    }

    #[test]
    fn test_detect_completed_step() {
        let tasks = vec![
            create_test_task("task-1", "First step"),
            create_test_task("task-2", "Second step"),
            create_test_task("task-3", "Third step"),
        ];

        let response = "I've completed step 2 as requested.";
        let matches = CompletionDetector::detect_completed_tasks(response, &tasks);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].task_id, "task-2");
    }

    #[test]
    fn test_detect_explicit_completion() {
        let tasks = vec![
            create_test_task("task-1", "Write unit tests"),
        ];

        let response = "I've completed the unit tests for the authentication module.";
        let matches = CompletionDetector::detect_completed_tasks(response, &tasks);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].task_id, "task-1");
    }

    #[test]
    fn test_detect_sentence_completion() {
        let tasks = vec![
            create_test_task("task-1", "Database migration"),
        ];

        let response = "The database migration is now complete.";
        let matches = CompletionDetector::detect_completed_tasks(response, &tasks);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].task_id, "task-1");
    }

    #[test]
    fn test_no_false_positives() {
        let tasks = vec![
            create_test_task("task-1", "Implement feature X"),
        ];

        // Should NOT match - talking about completion but not actually completing
        let response = "I will complete this feature once we have the requirements.";
        let matches = CompletionDetector::detect_completed_tasks(response, &tasks);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_empty_tasks() {
        let tasks: Vec<Task> = vec![];

        let response = "✓ Done with everything!";
        let matches = CompletionDetector::detect_completed_tasks(response, &tasks);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_deduplicate_keeps_highest_confidence() {
        let tasks = vec![
            create_test_task("task-1", "Setup database"),
        ];

        // Response with multiple patterns matching the same task
        let response = "✓ Setup database [DONE]";
        let matches = CompletionDetector::detect_completed_tasks(response, &tasks);

        // Should only have one match for task-1
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].task_id, "task-1");
        // Marker pattern has higher confidence (0.95) than checkmark (0.9)
        assert_eq!(matches[0].pattern, "marker");
    }
}
