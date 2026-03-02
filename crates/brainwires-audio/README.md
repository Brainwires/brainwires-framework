# brainwires-audio

[![Crates.io](https://img.shields.io/crates/v/brainwires-audio.svg)](https://crates.io/crates/brainwires-audio)
[![Documentation](https://img.shields.io/docsrs/brainwires-audio)](https://docs.rs/brainwires-audio)
[![License](https://img.shields.io/crates/l/brainwires-audio.svg)](LICENSE)

Audio I/O, speech-to-text, and text-to-speech for the Brainwires Agent Framework.

## Overview

`brainwires-audio` provides a unified interface for audio capture, playback, speech recognition, and speech synthesis. It abstracts over hardware backends (CPAL), cloud APIs (OpenAI Whisper / TTS), and local inference (whisper-rs) behind common traits, so agents can speak and listen without caring about the underlying implementation.

**Design principles:**

- **Trait-driven** — `AudioCapture`, `AudioPlayback`, `SpeechToText`, and `TextToSpeech` are all trait objects for swappable backends
- **Hardware-agnostic** — CPAL handles cross-platform audio device access (Linux, macOS, Windows)
- **Cloud + local** — OpenAI APIs for zero-setup, local Whisper for offline/private deployments
- **WAV-native** — built-in WAV encode/decode via `hound`, no external codec dependencies
- **Ring-buffered** — `AudioRingBuffer` for lock-free streaming between capture and processing

```text
  ┌──────────────────────────────────────────────────────────┐
  │                     brainwires-audio                      │
  │                                                          │
  │  ┌────────────┐     ┌────────────┐     ┌─────────────┐  │
  │  │  Hardware   │     │  Capture   │     │  Playback   │  │
  │  │  CpalCapture│────▶│  (trait)   │     │  (trait)    │  │
  │  │  CpalPlay   │     │ AudioBuffer│     │ AudioBuffer │  │
  │  └────────────┘     └──────┬─────┘     └──────┬──────┘  │
  │                            │                   │         │
  │                            ▼                   ▼         │
  │  ┌────────────┐     ┌────────────┐     ┌─────────────┐  │
  │  │    WAV     │     │    STT     │     │    TTS      │  │
  │  │ encode/    │     │  (trait)   │     │  (trait)    │  │
  │  │ decode     │     │ Transcript │     │  Voice      │  │
  │  └────────────┘     └──────┬─────┘     └──────┬──────┘  │
  │                            │                   │         │
  │                   ┌────────┴────────┐  ┌──────┴──────┐  │
  │                   │  API Backends   │  │ API Backend │  │
  │                   │  OpenAiStt      │  │ OpenAiTts   │  │
  │                   │  WhisperStt     │  │             │  │
  │                   └─────────────────┘  └─────────────┘  │
  └──────────────────────────────────────────────────────────┘

  Flow: Hardware → Capture/Playback → STT/TTS → API/Local backends
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-audio = "0.1"
```

Capture audio and transcribe it:

```rust
use brainwires_audio::{
    AudioConfig, AudioCapture, SpeechToText,
    SttOptions, Transcript,
};

// Configure audio format
let config = AudioConfig {
    sample_rate: 16000,
    channels: 1,
    ..Default::default()
};

// Capture from default microphone (requires `native` feature)
#[cfg(feature = "native")]
{
    use brainwires_audio::CpalCapture;
    let capture = CpalCapture::new(config.clone())?;
    let buffer = capture.record_seconds(5.0).await?;

    // Transcribe with OpenAI Whisper API
    use brainwires_audio::OpenAiStt;
    let stt = OpenAiStt::new("your-api-key");
    let transcript: Transcript = stt.transcribe(&buffer, SttOptions::default()).await?;
    println!("You said: {}", transcript.text);
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Hardware audio via CPAL, cloud APIs via reqwest, async streaming |
| `local-stt` | No | Local speech-to-text via `whisper-rs` — requires Whisper model weights on disk |

```toml
# Lightweight — no hardware or network deps (WAV encode/decode + traits only)
[dependencies]
brainwires-audio = { version = "0.1", default-features = false }

# With local Whisper STT
[dependencies]
brainwires-audio = { version = "0.1", features = ["local-stt"] }
```

## Architecture

### AudioBuffer

In-memory audio data with format metadata.

| Field | Type | Description |
|-------|------|-------------|
| `samples` | `Vec<f32>` | PCM samples normalized to [-1.0, 1.0] |
| `config` | `AudioConfig` | Sample rate, channel count, sample format |

### AudioConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sample_rate` | `u32` | `16000` | Samples per second |
| `channels` | `u16` | `1` | Mono (1) or stereo (2) |
| `sample_format` | `SampleFormat` | `F32` | Sample data type |

### AudioCapture (trait)

```rust
#[async_trait]
pub trait AudioCapture: Send + Sync {
    async fn record_seconds(&self, duration: f64) -> Result<AudioBuffer>;
    async fn start_stream(&self) -> Result<AudioStream>;
}
```

**Implementations:** `CpalCapture` (native feature)

### AudioPlayback (trait)

```rust
#[async_trait]
pub trait AudioPlayback: Send + Sync {
    async fn play(&self, buffer: &AudioBuffer) -> Result<()>;
}
```

**Implementations:** `CpalPlayback` (native feature)

### SpeechToText (trait)

```rust
#[async_trait]
pub trait SpeechToText: Send + Sync {
    async fn transcribe(&self, audio: &AudioBuffer, options: SttOptions) -> Result<Transcript>;
}
```

**Implementations:** `OpenAiStt` (native), `WhisperStt` (local-stt)

### TextToSpeech (trait)

```rust
#[async_trait]
pub trait TextToSpeech: Send + Sync {
    async fn synthesize(&self, text: &str, options: TtsOptions) -> Result<AudioBuffer>;
}
```

**Implementations:** `OpenAiTts` (native)

### Transcript

| Field | Type | Description |
|-------|------|-------------|
| `text` | `String` | Full transcription text |
| `segments` | `Vec<TranscriptSegment>` | Word-level or phrase-level segments with timestamps |
| `language` | `Option<String>` | Detected language code |

### AudioDevice

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Device display name |
| `direction` | `DeviceDirection` | `Input`, `Output`, or `Both` |
| `is_default` | `bool` | Whether this is the system default device |

### AudioRingBuffer

Lock-free ring buffer for streaming audio between producer (capture) and consumer (processing) threads. Fixed capacity, overwrites oldest samples when full.

## Usage Examples

### Text-to-Speech

```rust
use brainwires_audio::{TextToSpeech, TtsOptions, Voice};

#[cfg(feature = "native")]
{
    use brainwires_audio::{OpenAiTts, CpalPlayback, AudioPlayback};

    let tts = OpenAiTts::new("your-api-key");
    let options = TtsOptions {
        voice: Voice::alloy(),
        speed: 1.0,
        ..Default::default()
    };

    let audio = tts.synthesize("Hello from Brainwires!", options).await?;

    // Play through speakers
    let playback = CpalPlayback::new(audio.config.clone())?;
    playback.play(&audio).await?;
}
```

### WAV File I/O

```rust
use brainwires_audio::{encode_wav, decode_wav, AudioBuffer, AudioConfig};

// Decode a WAV file
let buffer = decode_wav("recording.wav")?;
println!("Duration: {:.1}s", buffer.duration_seconds());

// Encode audio to WAV
let config = AudioConfig { sample_rate: 16000, channels: 1, ..Default::default() };
let buffer = AudioBuffer { samples: vec![0.0; 16000], config };
encode_wav(&buffer, "silence.wav")?;
```

### Listing Audio Devices

```rust
use brainwires_audio::{AudioDevice, DeviceDirection};

#[cfg(feature = "native")]
{
    let devices = AudioDevice::enumerate()?;
    for device in &devices {
        println!(
            "{} ({:?}){}",
            device.name,
            device.direction,
            if device.is_default { " [default]" } else { "" }
        );
    }
}
```

### Local Whisper STT

```rust
use brainwires_audio::{SpeechToText, SttOptions};

#[cfg(feature = "local-stt")]
{
    use brainwires_audio::WhisperStt;

    // Load a local Whisper model
    let stt = WhisperStt::from_file("models/ggml-base.en.bin")?;
    let transcript = stt.transcribe(&buffer, SttOptions {
        language: Some("en".into()),
        ..Default::default()
    }).await?;

    for segment in &transcript.segments {
        println!("[{:.1}s - {:.1}s] {}", segment.start, segment.end, segment.text);
    }
}
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.1", features = ["audio"] }
```

Or depend on `brainwires-audio` directly for standalone audio support without the rest of the framework.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
