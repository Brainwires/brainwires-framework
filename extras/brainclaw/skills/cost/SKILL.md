---
name: cost
command: /cost
description: Show token usage and estimated cost for this session
version: "1.0.0"
author: BrainClaw
tags: [session, tokens, cost, usage]
---

The user has run the `/cost` command to see their current session's token usage.

Check the admin API metrics endpoint (`GET /admin/metrics`) if available, or estimate based on the conversation history. Then report to the user:

1. **Session token usage**: How many tokens have been used in this conversation (prompt tokens in, completion tokens out).
2. **Estimated cost**: Calculate an approximate cost based on the current provider's pricing. For common providers:
   - Claude claude-sonnet-4-6: ~$3/M input tokens, ~$15/M output tokens
   - Claude claude-opus-4-6: ~$15/M input tokens, ~$75/M output tokens
   - GPT-4o: ~$2.50/M input tokens, ~$10/M output tokens
   - GPT-4o-mini: ~$0.15/M input tokens, ~$0.60/M output tokens
   - Gemini 1.5 Pro: ~$1.25/M input tokens, ~$5/M output tokens
3. **Running total**: Mention that the `/admin/metrics` endpoint provides cumulative token counts across all sessions.

If you do not have exact token counts available, estimate based on the number of messages and average message length (approximately 4 characters per token). Be clear that any estimate is approximate.

Format the response in a concise table or list. Example:

```
Session token usage (estimated):
  Prompt tokens:     ~1,200
  Completion tokens: ~450
  Total tokens:      ~1,650

Estimated cost: ~$0.011 (at current provider rates)

Tip: Exact cumulative totals are available at GET /admin/metrics
```
