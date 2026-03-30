---
name: model
description: Switch or show the AI model for the current session. Usage: /model [model-name|default|list]
allowed-tools: []
metadata:
  category: session
  execution: inline
---

# Model Selection

The user wants to view or change the AI model used for this session.

## Usage

- `/model` → show the current model override (or "provider default" if none set)
- `/model list` → same as above, plus list available models
- `/model claude-opus-4-6` → switch to that model for this session immediately
- `/model default` or `/model reset` → revert to the provider's configured default model

## How It Works

Per-session model switching is fully supported. When you run `/model <name>`:
1. BrainClaw records the override for your session.
2. Your current conversation history is cleared (the new model starts fresh).
3. Every subsequent message in this channel uses `<name>` instead of the provider default.
4. Switching back with `/model default` similarly clears the session and reverts.

## Available Models (by provider)

**Anthropic**
- `claude-opus-4-6` — most capable, best for complex tasks
- `claude-sonnet-4-6` — fast and smart, best balance
- `claude-haiku-4-5` — fastest, cheapest

**OpenAI**
- `gpt-4o` — flagship multimodal model
- `gpt-4o-mini` — fast and cheap
- `o3` — extended reasoning
- `o4-mini` — fast reasoning

**Google**
- `gemini-2.5-pro` — Google's most capable
- `gemini-2.5-flash` — fast and cheap

**Groq**
- `llama-3.3-70b-versatile` — fast inference
- `mixtral-8x7b-32768` — large context

**Ollama (local)**
- `llama3.3`, `mistral`, `qwen2.5`, `phi4`, etc. — whatever you have pulled locally

## Instructions

1. The `/model` command is handled directly by BrainClaw before this skill runs.
   - Show or switch the model as requested.
   - This skill provides supplementary information about available models.

2. If the user asks which models are available, list the options above.

3. If the user asks whether per-session switching is supported: yes, it is — no restart needed.

## Notes

- The override persists for the life of the session; `/model default` reverts it.
- Switching models clears conversation history (new model starts fresh).
- The override is per-channel unless cross-channel identity is enabled.
- If the provider rejects an unknown model name, an error will appear on the next message.
