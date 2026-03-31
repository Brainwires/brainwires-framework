---
name: translate
description: Translates text to a specified language. Usage: /translate [language] [text], e.g. /translate Spanish Hello world
metadata:
  category: language
  execution: inline
---

# Translate

The user wants to translate text.

## Usage patterns

- `/translate Spanish Hello world` → translate "Hello world" to Spanish
- `/translate French` followed by pasted text → translate the text to French
- `/translate` with no args → ask what they want translated and to what language

## Instructions

1. Parse the target language from the first argument after `/translate`.
2. The remaining text (if any) is what to translate.
3. If text is missing, ask the user to provide it.
4. If no language is specified, ask which language they want.

## Output format

Produce the translation with:

**Translation ({target language})**
{translated text}

For longer texts, add a brief note about any nuances, idioms, or cultural considerations that affected the translation. Keep these notes concise — one sentence max per note.

If the input language is ambiguous, identify it briefly (e.g. "Translated from Spanish").
