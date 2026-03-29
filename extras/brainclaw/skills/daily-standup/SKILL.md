---
name: daily-standup
description: Guides you through a daily standup check-in and formats the output for sharing with your team.
allowed-tools:
  - Read
metadata:
  category: productivity
  execution: inline
---

# Daily Standup

Guide the user through a structured daily standup. Ask the following questions one at a time, wait for each answer, then produce a clean formatted standup report at the end.

## Questions to ask

1. **Yesterday**: What did you accomplish yesterday? (or since last standup)
2. **Today**: What will you work on today?
3. **Blockers**: Do you have any blockers or things slowing you down?

## Output format

Once you have all three answers, produce a standup summary in this format:

```
📋 Daily Standup — {today's date}

✅ Yesterday
{their answer}

🔨 Today
{their answer}

🚧 Blockers
{their answer, or "None" if they have no blockers}
```

Start by greeting the user and asking the first question. Keep a friendly, efficient tone.
