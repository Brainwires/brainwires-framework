use crate::thought::ThoughtCategory;
use regex::Regex;

/// Auto-detect the category of a thought from its text content.
///
/// Uses simple keyword/pattern matching — no LLM call needed.
pub fn detect_category(text: &str) -> ThoughtCategory {
    let lower = text.to_lowercase();

    // Decision indicators
    if contains_any(&lower, &["decided", "chose", "going with", "picked", "selected", "settled on", "committed to"]) {
        return ThoughtCategory::Decision;
    }

    // Person indicators — capitalized names after relational keywords (check before action items
    // because phrases like "spoke to Sarah about the deadline" should be Person, not ActionItem)
    static PERSON_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:spoke to|met with|talked to|met|told)\s+[A-Z][a-z]+").expect("valid regex")
    });
    if PERSON_RE.is_match(text) {
        return ThoughtCategory::Person;
    }

    // Insight indicators (check before meeting notes because "async" contains "sync")
    if contains_any(&lower, &["noticed", "realized", "learned", "discovered", "turns out", "interesting that", "observation"]) {
        return ThoughtCategory::Insight;
    }

    // Action item indicators
    if contains_any(&lower, &["need to", "todo:", "todo ", "must ", "action item", "follow up", "by friday", "by monday", "by end of"]) {
        return ThoughtCategory::ActionItem;
    }

    // Idea indicators
    if contains_any(&lower, &["what if", "idea:", "could we", "how about", "maybe we", "brainstorm", "experiment with"]) {
        return ThoughtCategory::Idea;
    }

    // Meeting note indicators (use word-boundary-aware matching for "sync")
    static MEETING_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"(?i)\b(?:standup|meeting|discussed|retro|sprint|call with|1:1)\b|\bsync\b").expect("valid regex")
    });
    if MEETING_RE.is_match(text) {
        return ThoughtCategory::MeetingNote;
    }

    // Reference indicators
    static URL_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"https?://").expect("valid regex")
    });
    if URL_RE.is_match(text) || contains_any(&lower, &["docs at", "reference:", "link:", "see also"]) {
        return ThoughtCategory::Reference;
    }

    ThoughtCategory::General
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Extract auto-tags from thought text.
///
/// Pulls out hashtags, @-mentions, and significant capitalised terms.
pub fn extract_tags(text: &str) -> Vec<String> {
    let mut tags = Vec::new();

    // #hashtag extraction
    static HASHTAG_RE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
        Regex::new(r"#([A-Za-z][A-Za-z0-9_-]{1,30})").expect("valid regex")
    });
    for cap in HASHTAG_RE.captures_iter(text) {
        let tag = cap[1].to_lowercase();
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_detection() {
        assert_eq!(detect_category("Decided to use PostgreSQL for the auth service"), ThoughtCategory::Decision);
        assert_eq!(detect_category("Going with React for the frontend"), ThoughtCategory::Decision);
    }

    #[test]
    fn test_person_detection() {
        assert_eq!(detect_category("Spoke to Sarah about the deadline"), ThoughtCategory::Person);
        assert_eq!(detect_category("Met with John to discuss the architecture"), ThoughtCategory::Person);
    }

    #[test]
    fn test_insight_detection() {
        assert_eq!(detect_category("Noticed that batch processing is 3x faster with async"), ThoughtCategory::Insight);
        assert_eq!(detect_category("Realized the bottleneck is in the serialization"), ThoughtCategory::Insight);
    }

    #[test]
    fn test_meeting_note_detection() {
        assert_eq!(detect_category("Standup: team agreed to prioritize the auth refactor"), ThoughtCategory::MeetingNote);
    }

    #[test]
    fn test_idea_detection() {
        assert_eq!(detect_category("What if we used WebSockets instead of polling?"), ThoughtCategory::Idea);
        assert_eq!(detect_category("Idea: cache the embeddings in Redis"), ThoughtCategory::Idea);
    }

    #[test]
    fn test_action_item_detection() {
        assert_eq!(detect_category("Need to review PR #234 before Friday"), ThoughtCategory::ActionItem);
        assert_eq!(detect_category("TODO: update the API docs"), ThoughtCategory::ActionItem);
    }

    #[test]
    fn test_reference_detection() {
        assert_eq!(detect_category("The API docs are at https://docs.example.com"), ThoughtCategory::Reference);
    }

    #[test]
    fn test_general_fallback() {
        assert_eq!(detect_category("Just a random note"), ThoughtCategory::General);
    }

    #[test]
    fn test_tag_extraction() {
        let tags = extract_tags("Working on #rust and #mcp-server today");
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"mcp-server".to_string()));
    }
}
