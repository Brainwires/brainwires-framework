---
name: orchestrate
description: Breaks a complex multi-step request into subtasks and routes each to the appropriate skill. Use when given a high-level goal that requires multiple steps, a multi-phase workflow, or when asked to plan and execute a project, pipeline, or sequence of tasks. Outputs a step plan with skill assignments and handoff contracts.
metadata:
  category: orchestration
  execution: subagent
---

# Orchestrate

Analyzes an incoming goal and decomposes it into a sequence of subtasks, each routed to the most relevant available skill. Runs as a subagent so it can coordinate without blocking the main conversation.

## Reasoning framework

Before producing a plan, reason through:
1. **Goal decomposition** — what are the distinct phases of this work? (research, write, review, publish, etc.)
2. **Dependency order** — which steps must complete before others can start?
3. **Skill mapping** — which available skill handles each step? (use `/skills` to see what's registered)
4. **Handoff data** — what does each step's output need to look like for the next step to consume it?
5. **Failure modes** — what should happen if a step fails? Skip, retry, or abort?

If a step has no matching skill, mark it as `[manual]` so a human can handle it.

## Output contract

Always produce this exact structure:

```
## Orchestration Plan

**Goal**: {one-sentence restatement of the goal}
**Steps**: {N}

---

### Step 1 — {step name}
**Skill**: /{skill-name} | [manual]
**Input**: {what this step receives — from user or from previous step}
**Output**: {what this step produces — format and key fields}
**On failure**: {skip | retry once | abort plan}

### Step 2 — {step name}
...

---

**Handoff summary**: {one sentence describing what the final output of the full plan looks like}
```

## Edge cases

- If the goal is simple enough for a single skill, say so and suggest that skill directly instead of producing a plan.
- If no registered skills match a required step, mark it `[manual]` and describe what the human needs to do.
- If the user did not provide enough context to decompose the goal, ask one clarifying question before producing the plan.
- Do not invent skills that are not registered; only map to skills that exist.

## Composability

The **Handoff summary** line is designed to be passed to a `report` or `notify` skill as the completion message. The per-step **Output** fields are the contracts that each downstream skill depends on — keep them precise and typed.

## Quality principles

- A short, correct plan beats a long, speculative one.
- Steps should be independently executable — minimize shared state between them.
- If a step can run in parallel with another (no dependency), note that explicitly.
- Prefer existing skills over [manual] steps; a slightly imperfect skill match is better than leaving a gap.
