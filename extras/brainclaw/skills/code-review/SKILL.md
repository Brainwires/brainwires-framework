---
name: code-review
description: Analyzes code for bugs, security vulnerabilities, performance problems, and style issues. Use when reviewing a PR, reading a diff, auditing a file, or asked to check, critique, inspect, or audit code. Outputs a structured markdown report with severity ratings.
allowed-tools: Read Grep
metadata:
  category: development
  execution: inline
---

# Code Review

## Input contract

Accepts any of the following:
- **File path** — read and review the file directly
- **Pasted code** — review inline after the command
- **Diff / patch** — analyze changes only
- **No argument** — ask the user to provide code or a file path

## Output contract

Always produce this exact structure so downstream skills (e.g. fix-issues, pr-comment) can parse it:

```
## Code Review

**Language / Framework**: {detected language and framework}

### Issues Found
- [HIGH] {description} — line {N} if available
- [MED]  {description}
- [LOW]  {description}

### Looks Good
- {what is done well}

### Suggestions
- {non-bug improvements}

### Summary
{One or two sentences: overall quality assessment and recommended next step.}
```

Omit any section that has nothing to report. If no issues are found, say so explicitly and explain what makes the code good.

## Review checklist

- **Bugs**: off-by-one errors, null/None dereferences, logic errors, missed edge cases
- **Security**: injection (SQL, shell, XSS), insecure defaults, hardcoded secrets, missing input validation, unsafe deserialization
- **Performance**: unnecessary allocations, O(n²) loops, blocking calls in async contexts, missing indexes
- **Readability**: naming clarity, comment quality, excessive function length, high cyclomatic complexity
- **Error handling**: unhandled errors, swallowed panics/exceptions, missing validation at boundaries

## Edge cases

- If the file does not exist, report it and ask the user to confirm the path.
- If the pasted content is not code (e.g. config, prose), still analyze it and note what it is.
- If the diff is very large (500+ lines), prioritize HIGH-severity findings and note that a full review was not feasible.
- Do not request tools not listed in `allowed-tools`; if a needed file is outside reach, note the limitation.

## Composability

The **Summary** line is designed to be passed to a `fix-issues` or `pr-comment` skill as a handoff. Keep it action-oriented: "Two high-severity issues found; recommend fixing before merge."
