# voice-assistant

Personal voice assistant built on the [Brainwires Framework](https://github.com/Brainwires/brainwires-framework).

## Overview

`voice-assistant` is a standalone binary that listens on a microphone, transcribes speech with an STT provider, sends it to an LLM, and plays back the response via TTS — all wired through `brainwires-hardware`'s audio pipeline.

## Features

- Continuous microphone capture via CPAL
- Speech-to-text via OpenAI Whisper (configurable)
- LLM response via any `brainwires-providers` backend
- Text-to-speech playback via OpenAI TTS (configurable)
- Optional wake-word detection (Rustpotter or Picovoice Porcupine)
- TOML config file at `~/.config/voice-assistant/config.toml`

## Usage

```sh
cargo build --release -p voice-assistant
./target/release/voice-assistant --config ~/.config/voice-assistant/config.toml
```

## Feature flags

| Flag | Description |
|---|---|
| `wake-word` | Enable wake-word detection (engine auto-selected) |
| `wake-word-rustpotter` | Use Rustpotter for wake-word detection |
| `wake-word-porcupine` | Use Picovoice Porcupine for wake-word detection |

## License

MIT OR Apache-2.0
