---
name: code-review
description: Reviews code pasted by the user for bugs, security issues, performance, and style. Provides actionable feedback.
allowed-tools:
  - Read
  - Grep
metadata:
  category: development
  execution: inline
---

# Code Review

The user wants a code review. They will paste code or describe what to review.

## Instructions

1. If code was pasted after `/code-review`, review it immediately.
2. If a file path was given, read the file and review it.
3. If neither, ask the user to paste the code or provide a file path.

## Review checklist

Examine the code for:

- **Bugs**: Off-by-one errors, null dereferences, logic errors, edge cases
- **Security**: Injection vulnerabilities, insecure defaults, secret exposure, input validation
- **Performance**: Unnecessary allocations, O(n²) loops, blocking calls in async contexts
- **Readability**: Naming clarity, comment quality, function length, complexity
- **Error handling**: Unhandled errors, swallowed panics, missing validation

## Output format

```
## Code Review

**Language / Framework**: {detected}

### 🐛 Issues Found
- [SEVERITY: High/Medium/Low] Description of issue, line reference if available

### ✅ Looks Good
- Things done well

### 💡 Suggestions
- Improvements that aren't bugs but would help

### Summary
Overall assessment in 1-2 sentences.
```

If no issues are found, say so clearly and highlight what makes the code good.
