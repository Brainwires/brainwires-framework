# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for the brainwires-cli project.

## What is an ADR?

An ADR is a document that captures an important architectural decision made along with its context and consequences. They help future developers understand why certain decisions were made.

## Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [0001](0001-use-lancedb-for-vector-storage.md) | Use LanceDB for Vector Storage | Accepted | 2025 |
| [0002](0002-multi-agent-architecture.md) | Multi-Agent Architecture | Accepted | 2025 |
| [0003](0003-mdap-voting-system.md) | MDAP Voting System for High-Reliability Tasks | Accepted | 2025 |
| [0004](0004-mcp-server-mode.md) | MCP Server Mode for Agent Management | Accepted | 2025 |
| [0005](0005-token-auditing-system.md) | Token Usage Auditing System | Accepted | 2025 |

## Template for New ADRs

```markdown
# ADR NNNN: Title

## Status

[Proposed | Accepted | Deprecated | Superseded by ADR-XXXX]

## Context

What is the issue that we're seeing that is motivating this decision or change?

## Options Considered

### Option 1
**Pros:**
- ...

**Cons:**
- ...

### Option 2
...

## Decision

What is the change that we're proposing and/or doing?

## Consequences

### Positive
- ...

### Negative
- ...

### Mitigations
- ...

## References

- Links to relevant code, docs, or external resources
```

## Creating a New ADR

1. Copy the template above
2. Name the file `NNNN-short-title.md` where NNNN is the next number
3. Fill in all sections
4. Update this README's index
5. Commit with a descriptive message
