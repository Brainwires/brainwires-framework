---
name: export
description: Export the current conversation history to a formatted transcript. Usage: /export [format]
allowed-tools: []
metadata:
  category: session
  execution: inline
---

# Export Conversation

The user wants to export or save the current conversation history.

## Usage

- `/export` → export as markdown (default)
- `/export markdown` → export as formatted markdown
- `/export json` → export as JSON
- `/export text` → export as plain text

## Instructions

1. Parse the optional format argument (markdown, json, text). Default to markdown.
2. Generate a transcript of the conversation so far in the requested format.
3. For **markdown**: format each turn as `**User:**` / `**Assistant:**` with a horizontal rule between turns.
4. For **json**: output a JSON array of `{"role": "user"|"assistant", "content": "..."}` objects.
5. For **text**: plain text with `User:` / `Assistant:` labels and blank lines between turns.
6. Present the export as a code block in the appropriate language (markdown, json, or text).
7. Tell the user they can copy the block or save it to a file.

## Notes

- Do NOT fabricate conversation history. Only include exchanges that actually happened in this session.
- If the conversation has just started (this is the first message), say so and output an empty transcript.
- Keep the transcript accurate — do not summarize or paraphrase earlier messages.
