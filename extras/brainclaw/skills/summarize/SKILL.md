---
name: summarize
description: Summarizes text, articles, documents, or anything the user pastes. Produces a concise bullet-point summary with key takeaways.
metadata:
  category: productivity
  execution: inline
---

# Summarize

The user wants a summary of text or content. They will either:
- Paste text directly after the command
- Describe what they want summarized

## Instructions

1. If text was pasted after `/summarize`, summarize it immediately.
2. If no text was provided, ask the user to paste or describe the content.

## Output format

Produce a summary with:

**📝 Summary**
2–4 sentence overview of the content.

**Key Points**
- Bullet point 1
- Bullet point 2
- (etc.)

**💡 Takeaway**
One sentence — the single most important thing to remember.

Keep the summary concise. For long documents, prioritize the most important information. Do not pad with filler.
