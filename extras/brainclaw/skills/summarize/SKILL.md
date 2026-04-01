---
name: summarize
description: Condenses any text into a structured summary with key points and a one-sentence takeaway. Use when asked to summarize, TLDR, condense, distill, or recap text, articles, transcripts, meeting notes, or documents. Outputs an overview paragraph, bullet-point key points, and a single takeaway line.
metadata:
  category: productivity
  execution: inline
---

# Summarize

## Input contract

Accepts any of the following:
- **Pasted text** — summarize inline after the command
- **File path** — read and summarize the file
- **Topic description** — ask the user to paste the content if not provided

## Output contract

Always produce this exact structure so downstream skills (e.g. write-report, send-digest) can consume it:

```
**Summary**
{2–4 sentence overview of the content.}

**Key Points**
- {point 1}
- {point 2}
- {point 3 — add more as needed, remove section if fewer than 2 points}

**Takeaway**
{Single sentence: the one thing most important to remember.}
```

## Instructions

1. If text was pasted after `/summarize`, summarize it immediately.
2. If a file path was given, read the file and summarize it.
3. If neither, ask the user to paste or describe the content.

Prioritize signal over volume. For long documents, surface the highest-value points — do not pad with filler. Strip marketing language and restate facts plainly.

## Edge cases

- For very long content (5,000+ words), note that a representative sample was used if you cannot process it all.
- For structured content (meeting notes, lists), preserve the structure in key points rather than flattening it.
- For technical content, keep jargon only when removing it would lose precision; define it on first use.
- If the content is already a summary or bullet list, summarize it as-is and note the source was already condensed.

## Composability

The **Takeaway** line is designed to be passed to an `email-draft` or `send-digest` skill as the subject/lead. Keep it a complete sentence that stands alone without context.
