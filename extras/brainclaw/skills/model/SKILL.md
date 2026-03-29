---
name: model
description: Switch or show the AI model for the current session. Usage: /model [model-name]
allowed-tools: []
metadata:
  category: session
  execution: inline
---

# Model Selection

The user wants to view or change the AI model used for this session.

## Usage

- `/model` → show the current model and list available options
- `/model claude-opus-4-6` → request switching to a specific model
- `/model list` → list known available models

## Instructions

1. If no model name is provided (or the argument is `list`):
   - Tell the user the current model is configured in `brainclaw.toml` under `[provider] default_model`.
   - List common models by provider:
     - **Anthropic**: `claude-opus-4-6`, `claude-sonnet-4-6`, `claude-haiku-4-5`
     - **OpenAI**: `gpt-4o`, `gpt-4o-mini`, `o3`, `o4-mini`
     - **Google**: `gemini-2.5-pro`, `gemini-2.5-flash`
     - **Groq**: `llama-3.3-70b-versatile`, `mixtral-8x7b-32768`
   - Tell the user they can change the model by editing `[provider] default_model` in their config file and restarting BrainClaw.

2. If a model name is provided:
   - Acknowledge the request.
   - Explain that per-session model switching is not yet available in this version.
   - Tell the user to set `default_model = "<model-name>"` in `[provider]` in `brainclaw.toml` and restart to use that model.
   - If the model name looks valid (matches a known provider), tell the user which provider to set as `default_provider` as well.

## Notes

- Be helpful and specific — don't just say "not supported". Give the user the exact config change they need.
- If the user provides an unknown model name, say so and suggest the closest known alternative.
