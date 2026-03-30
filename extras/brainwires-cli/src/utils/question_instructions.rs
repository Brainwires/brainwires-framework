//! Question Instructions
//!
//! Instructions for AI models on how to format clarifying questions
//! during planning stages for the TUI Q&A system.
//!
//! Includes AT-CoT (Ambiguity Type-Chain of Thought) methodology from
//! arXiv:2504.12113 for improved disambiguation.

/// Instructions for formatting clarifying questions in AI responses
///
/// These instructions are injected into the conversation context during
/// planning stages to enable the TUI Q&A system with AT-CoT methodology.
pub const QUESTION_INSTRUCTIONS: &str = r#"## Clarifying Questions with AT-CoT

When you need clarification from the user during planning or before taking significant actions, use the AT-CoT (Ambiguity Type-Chain of Thought) methodology to ask structured questions.

### AT-CoT Two-Step Process

**STEP 1: Predict Ambiguity Types**

Before generating questions, analyze the user's query for three types of ambiguity:

1. **SEMANTIC** - Terms with multiple meanings
   - User action: User clarifies the MEANING of unclear terms
   - Example: "cache" could mean LRU cache, LFU cache, TTL cache, Redis cache
   - When to use: Technical terms, domain-specific vocabulary, words with multiple interpretations
   - Question focus: "What do you mean by X?" / "Which type of X?"

2. **GENERALIZE** - Request is too specific, may need broader scope
   - User action: User BROADENS overly specific request
   - Example: "Add validation to login form" → User wants ALL forms, not just login
   - When to use: User mentions specific instance but pattern suggests broader need
   - Question focus: "Should this apply to X only, or to Y as well?"

3. **SPECIFY** - Request is too broad, needs concrete constraints
   - User action: User NARROWS overly broad request
   - Example: "Optimize the code" → User specifies "database query performance"
   - When to use: Vague goals, multiple valid approaches, missing constraints
   - Question focus: "What specific aspect?" / "Which priority?" / "What constraints?"

**STEP 2: Generate Clarifications**

Use predicted ambiguity types to generate targeted questions:
- For SEMANTIC: Ask about term meanings and definitions
- For GENERALIZE: Ask if scope should be broader
- For SPECIFY: Ask for concrete constraints and priorities

### JSON Format (Enhanced with AT-CoT)

<clarifying_questions>
{
  "ambiguity_analysis": {
    "predicted_types": ["semantic", "specify"],
    "reasoning": "Query mentions 'cache' (semantic ambiguity - could be LRU, LFU, TTL, Redis) and 'optimize' (needs specification of optimization goals)"
  },
  "questions": [
    {
      "id": "q1",
      "question": "Which cache type should we implement?",
      "header": "Cache Type",
      "ambiguity_type": "semantic",
      "multi_select": false,
      "options": [
        {"id": "a", "label": "LRU cache", "description": "Least Recently Used eviction"},
        {"id": "b", "label": "LFU cache", "description": "Least Frequently Used eviction"},
        {"id": "c", "label": "TTL cache", "description": "Time-To-Live expiration"}
      ]
    },
    {
      "id": "q2",
      "question": "What optimization goal is most important?",
      "header": "Goal",
      "ambiguity_type": "specify",
      "multi_select": false,
      "options": [
        {"id": "a", "label": "Lookup speed", "description": "Minimize cache access time"},
        {"id": "b", "label": "Memory usage", "description": "Minimize memory footprint"},
        {"id": "c", "label": "Hit rate", "description": "Maximize cache effectiveness"}
      ]
    }
  ]
}
</clarifying_questions>

### AT-CoT Examples

**Example 1: Semantic Ambiguity**
User: "Implement a cache for user data"

Ambiguity Analysis:
- predicted_types: ["semantic"]
- reasoning: "The term 'cache' is ambiguous - could refer to in-memory LRU cache, Redis distributed cache, or browser cache"

Question:
- "Which cache implementation should we use?" [SEMANTIC]
- Options: LRU in-memory, Redis distributed, Browser cache

**Example 2: Semantic + Specify Ambiguity**
User: "Optimize the authentication flow"

Ambiguity Analysis:
- predicted_types: ["semantic", "specify"]
- reasoning: "'Optimize' is vague (could mean speed, security, UX) and 'authentication flow' could refer to multiple auth methods"

Questions:
- "What aspect of authentication should we optimize?" [SEMANTIC]
  Options: Speed, Security, User experience
- "Which authentication method are you using?" [SPECIFY]
  Options: JWT tokens, OAuth2, Session cookies

**Example 3: Generalize Ambiguity**
User: "Add error handling to the login endpoint"

Ambiguity Analysis:
- predicted_types: ["generalize"]
- reasoning: "User mentions specific endpoint but error handling is likely needed across all endpoints"

Question:
- "Should error handling be added to just the login endpoint, or all API endpoints?" [GENERALIZE]
  Options: Login only, All endpoints, All authentication endpoints

**Example 4: Specify Ambiguity**
User: "Make the database queries faster"

Ambiguity Analysis:
- predicted_types: ["specify"]
- reasoning: "Goal is too broad - needs concrete constraints on which queries, what metrics, what tradeoffs"

Questions:
- "Which database queries need optimization?" [SPECIFY]
  Options: User profile queries, Search queries, All queries
- "What performance target do you have in mind?" [SPECIFY]
  Options: Under 100ms, Under 50ms, Under 10ms

### Rules:
1. **ALWAYS** include `ambiguity_analysis` with `predicted_types` and `reasoning`
2. **OPTIONALLY** add `ambiguity_type` field to each question (recommended but not required)
3. Maximum 4 questions per response
4. Each question must have 2-4 options
5. Keep option labels brief (1-5 words)
6. Headers must be 12 characters or less
7. Use `multi_select: true` only when multiple options can be selected together
8. Write the question block at the very END of your response
9. Only ask when genuinely needed - don't ask unnecessary questions
10. Users can always type a custom "Other" response

### Backward Compatibility

The `ambiguity_analysis` and `ambiguity_type` fields are **optional**. If you're unsure about ambiguity types or the query is straightforward, you can omit these fields and use the simple format:

<clarifying_questions>
{
  "questions": [
    {
      "id": "q1",
      "question": "Which option?",
      "header": "Option",
      "multi_select": false,
      "options": [...]
    }
  ]
}
</clarifying_questions>

### When to ask questions:
- Before implementing features with multiple valid approaches
- When user requirements are ambiguous (use AT-CoT to classify ambiguity type)
- Before making significant architectural decisions
- When you need to understand preferences or priorities

### When NOT to ask questions:
- For simple, straightforward tasks with clear requirements
- When the answer is obvious from context
- For minor implementation details you can decide yourself
- When the user has already provided sufficient detail
"#;

/// Get the question instructions for injection into conversation context
pub fn get_question_instructions() -> &'static str {
    QUESTION_INSTRUCTIONS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instructions_contain_at_cot_keywords() {
        let instructions = get_question_instructions();

        // Check for AT-CoT ambiguity types
        assert!(instructions.contains("SEMANTIC"));
        assert!(instructions.contains("GENERALIZE"));
        assert!(instructions.contains("SPECIFY"));

        // Check for AT-CoT methodology
        assert!(instructions.contains("ambiguity_analysis"));
        assert!(instructions.contains("predicted_types"));
        assert!(instructions.contains("ambiguity_type"));

        // Check for examples
        assert!(instructions.contains("Example 1"));
        assert!(instructions.contains("Example 2"));

        // Check for backward compatibility note
        assert!(instructions.contains("optional"));
    }

    #[test]
    fn test_instructions_include_two_step_process() {
        let instructions = get_question_instructions();
        assert!(instructions.contains("STEP 1"));
        assert!(instructions.contains("STEP 2"));
        assert!(instructions.contains("Predict Ambiguity Types"));
        assert!(instructions.contains("Generate Clarifications"));
    }
}
