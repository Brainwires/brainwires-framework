# audio-demo-ffi

UniFFI bindings for [brainwires-audio](../../crates/brainwires-audio/README.md) — exposes TTS, STT, and hardware audio APIs to C#, Kotlin, Swift, and Python via Mozilla's [UniFFI](https://mozilla.github.io/uniffi-rs/) binding generator.

This crate compiles to a `cdylib` shared library (`libaudio_demo_ffi.so` / `.dll` / `.dylib`) that foreign language bindings can call through auto-generated FFI wrappers.

## Supported Providers

| Provider | TTS | STT | Notes |
|----------|-----|-----|-------|
| OpenAI (TTS-1 / Whisper) | Yes | Yes | Standard `/v1/audio/*` endpoints |
| OpenAI Responses API | Yes | Yes | GPT-4o Audio via `/v1/responses` with audio modality |
| ElevenLabs | Yes | Yes | Multilingual v2 |
| Deepgram | Yes | Yes | Aura TTS / Nova-2 STT |
| Google Cloud TTS | Yes | No | Multi-language neural voices |
| Azure Cognitive Services | Yes | Yes | Requires region parameter |
| Fish Audio | Yes | Yes | |
| Cartesia | Yes | No | Sonic English/Multilingual |
| Murf AI | Yes | No | |

## Building

```bash
# Debug build
cargo build -p audio-demo-ffi

# Release build (produces smaller binary)
cargo build --release -p audio-demo-ffi
```

The output is:
- Linux: `target/release/libaudio_demo_ffi.so`
- macOS: `target/release/libaudio_demo_ffi.dylib`
- Windows: `target/release/audio_demo_ffi.dll`

## Generating Language Bindings

### C# (for Avalonia / .NET)

```bash
cargo install uniffi-bindgen-cs --git https://github.com/aspect-build/uniffi-bindgen-cs
uniffi-bindgen-cs target/release/libaudio_demo_ffi.so --out-dir bindings/csharp/
```

### Kotlin

```bash
cargo run -p audio-demo-ffi -- generate --language kotlin --out-dir bindings/kotlin/
```

### Swift

```bash
cargo run -p audio-demo-ffi -- generate --language swift --out-dir bindings/swift/
```

### Python

```bash
cargo run -p audio-demo-ffi -- generate --language python --out-dir bindings/python/
```

## FFI API

All functions are synchronous from the caller's perspective — async Rust operations are bridged through an internal Tokio runtime.

### Provider Lifecycle

```
create_provider(name, api_key, region?) -> handle
drop_provider(handle)
list_providers() -> [ProviderInfo]
```

### Text-to-Speech

```
tts_list_voices(handle) -> [Voice]
tts_synthesize(handle, text, options) -> AudioBuffer
```

### Speech-to-Text

```
stt_transcribe(handle, audio, options) -> Transcript
```

### Hardware Audio

```
audio_list_input_devices() -> [AudioDevice]
audio_list_output_devices() -> [AudioDevice]
audio_record(device_id?, duration_secs) -> AudioBuffer
audio_play(device_id?, buffer)
```

## Architecture

```text
Foreign Language (C#, Kotlin, Swift, Python)
    │
    ▼  (UniFFI-generated bindings)
┌──────────────────────────────┐
│       audio-demo-ffi         │
│  ┌─────────┐ ┌────────────┐  │
│  │types_ffi│ │   bridge   │  │
│  │ (mirror │ │ (Tokio RT, │  │
│  │  types) │ │  registry) │  │
│  └─────────┘ └────────────┘  │
└──────────────┬───────────────┘
               │
    ┌──────────▼──────────┐
    │  brainwires-audio   │
    │  (async Rust API)   │
    └──────────┬──────────┘
               │
    ┌──────────▼──────────┐
    │ brainwires-providers│
    │  (HTTP API clients) │
    └─────────────────────┘
```

## License

MIT OR Apache-2.0
