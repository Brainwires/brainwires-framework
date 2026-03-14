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
- **WAV + FLAC** — built-in WAV encode/decode via `hound`, lossless FLAC encode/decode via `flacenc`/`claxon` (all pure Rust)
- **Ring-buffered** — `AudioRingBuffer` for lock-free streaming between capture and processing

```text
  ┌──────────────────────────────────────────────────────────┐
  │                     brainwires-audio                      │
  │                                                          │
  │  ┌────────────┐     ┌────────────┐     ┌─────────────┐  │
  │  │  Hardware   │     │  Capture   │     │  Playback   │  │
  │  │  CpalCapture│────>│  (trait)   │     │  (trait)    │  │
  │  │  CpalPlay   │     │ AudioBuffer│     │ AudioBuffer │  │
  │  └────────────┘     └──────┬─────┘     └──────┬──────┘  │
  │                            │                   │         │
  │                            v                   v         │
  │  ┌────────────┐     ┌────────────┐     ┌─────────────┐  │
  │  │  WAV/FLAC  │     │    STT     │     │    TTS      │  │
  │  │  encode/   │     │  (trait)   │     │  (trait)    │  │
  │  │  decode    │     │ Transcript │     │  Voice      │  │
  │  └────────────┘     └──────┬─────┘     └──────┬──────┘  │
  │                            │                   │         │
  │                   ┌────────┴────────┐  ┌──────┴──────┐  │
  │                   │  API Backends   │  │ API Backend │  │
  │                   │  OpenAiStt      │  │ OpenAiTts   │  │
  │                   │  WhisperStt     │  │             │  │
  │                   └─────────────────┘  └─────────────┘  │
  └──────────────────────────────────────────────────────────┘

  Flow: Hardware -> Capture/Playback -> STT/TTS -> API/Local backends
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
brainwires-audio = "0.4"
```

Capture audio, save as WAV, and transcribe:

```rust
use brainwires_audio::{
    AudioConfig, AudioCapture, SpeechToText,
    CpalCapture, OpenAiStt,
    SttOptions, encode_wav,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let capture = CpalCapture::new();
    let config = AudioConfig::speech(); // 16 kHz mono i16

    // Record 5 seconds from default mic
    let buffer = capture.record(None, &config, 5.0).await?;

    // Save to WAV
    let wav = encode_wav(&buffer)?;
    std::fs::write("recording.wav", &wav)?;

    // Transcribe with OpenAI Whisper
    let stt = OpenAiStt::new("your-api-key");
    let transcript = stt.transcribe(&buffer, &SttOptions::default()).await?;
    println!("You said: {}", transcript.text);

    Ok(())
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `native` | Yes | Hardware audio via CPAL, cloud APIs via reqwest, async streaming. Includes `flac`. |
| `flac` | Yes (via `native`) | FLAC lossless encode (`flacenc`) and decode (`claxon`) — pure Rust, no system deps |
| `local-stt` | No | Local speech-to-text via `whisper-rs` — requires Whisper GGML model weights on disk |

```toml
# Lightweight — no hardware or network deps (WAV encode/decode + traits only)
[dependencies]
brainwires-audio = { version = "0.4", default-features = false }

# Default + local Whisper STT
[dependencies]
brainwires-audio = { version = "0.4", features = ["local-stt"] }

# FLAC only, no hardware
[dependencies]
brainwires-audio = { version = "0.4", default-features = false, features = ["flac"] }
```

## Architecture

### AudioBuffer

In-memory audio data with format metadata.

| Field | Type | Description |
|-------|------|-------------|
| `data` | `Vec<u8>` | Raw PCM samples (little-endian bytes) |
| `config` | `AudioConfig` | Sample rate, channel count, sample format |

**Methods:** `from_pcm(data, config)`, `duration_secs()`, `num_frames()`, `is_empty()`

### AudioConfig

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `sample_rate` | `u32` | `16000` | Samples per second |
| `channels` | `u16` | `1` | Mono (1) or stereo (2) |
| `sample_format` | `SampleFormat` | `I16` | `I16` or `F32` |

**Presets:** `AudioConfig::speech()` (16 kHz mono i16), `AudioConfig::cd_quality()` (44.1 kHz stereo i16), `AudioConfig::high_quality()` (48 kHz stereo f32)

### AudioCapture (trait)

```rust
#[async_trait]
pub trait AudioCapture: Send + Sync {
    fn list_devices(&self) -> AudioResult<Vec<AudioDevice>>;
    fn default_device(&self) -> AudioResult<Option<AudioDevice>>;
    fn start_capture(&self, device: Option<&AudioDevice>, config: &AudioConfig)
        -> AudioResult<BoxStream<'static, AudioResult<AudioBuffer>>>;
    async fn record(&self, device: Option<&AudioDevice>, config: &AudioConfig, duration_secs: f64)
        -> AudioResult<AudioBuffer>;
}
```

**Implementations:** `CpalCapture` (native feature)

### AudioPlayback (trait)

```rust
#[async_trait]
pub trait AudioPlayback: Send + Sync {
    fn list_devices(&self) -> AudioResult<Vec<AudioDevice>>;
    fn default_device(&self) -> AudioResult<Option<AudioDevice>>;
    async fn play(&self, device: Option<&AudioDevice>, buffer: &AudioBuffer) -> AudioResult<()>;
    async fn play_stream(&self, device: Option<&AudioDevice>, config: &AudioConfig,
        stream: BoxStream<'static, AudioResult<AudioBuffer>>) -> AudioResult<()>;
}
```

**Implementations:** `CpalPlayback` (native feature)

### SpeechToText (trait)

```rust
#[async_trait]
pub trait SpeechToText: Send + Sync {
    fn name(&self) -> &str;
    async fn transcribe(&self, audio: &AudioBuffer, options: &SttOptions) -> AudioResult<Transcript>;
    fn transcribe_stream(&self, audio_stream: BoxStream<'static, AudioResult<AudioBuffer>>,
        options: &SttOptions) -> BoxStream<'static, AudioResult<Transcript>>;
}
```

**Implementations:** `OpenAiStt` (native), `WhisperStt` (local-stt)

### TextToSpeech (trait)

```rust
#[async_trait]
pub trait TextToSpeech: Send + Sync {
    fn name(&self) -> &str;
    async fn list_voices(&self) -> AudioResult<Vec<Voice>>;
    async fn synthesize(&self, text: &str, options: &TtsOptions) -> AudioResult<AudioBuffer>;
    fn synthesize_stream(&self, text: &str, options: &TtsOptions)
        -> BoxStream<'static, AudioResult<AudioBuffer>>;
}
```

**Implementations:** `OpenAiTts` (native)

### Transcript

| Field | Type | Description |
|-------|------|-------------|
| `text` | `String` | Full transcription text |
| `segments` | `Vec<TranscriptSegment>` | Timed segments with `start`/`end` in seconds |
| `language` | `Option<String>` | Detected language code |
| `duration_secs` | `Option<f64>` | Audio duration |

### AudioDevice

| Field | Type | Description |
|-------|------|-------------|
| `id` | `String` | Platform-specific device identifier |
| `name` | `String` | Human-readable display name |
| `direction` | `DeviceDirection` | `Input` or `Output` |
| `is_default` | `bool` | Whether this is the system default device |

### AudioRingBuffer

Lock-free ring buffer for streaming audio between producer (capture) and consumer (processing) threads. Fixed capacity, overwrites oldest samples when full.

**Methods:** `new(config, duration_secs)`, `push(bytes)`, `read_all()`, `duration_secs()`, `clear()`, `is_full()`

## Usage Examples

### Record and Save as WAV or FLAC

```rust
use brainwires_audio::{AudioConfig, AudioCapture, CpalCapture, encode_wav};

let capture = CpalCapture::new();
let config = AudioConfig::speech();
let buffer = capture.record(None, &config, 5.0).await?;

// Save as WAV
let wav = encode_wav(&buffer)?;
std::fs::write("recording.wav", &wav)?;

// Save as FLAC (smaller, lossless)
#[cfg(feature = "flac")]
{
    let flac = brainwires_audio::encode_flac(&buffer)?;
    std::fs::write("recording.flac", &flac)?;
}
```

### Load and Play Audio (WAV or FLAC)

```rust
use brainwires_audio::{CpalPlayback, AudioPlayback, decode_wav};

// WAV
let buffer = decode_wav(&std::fs::read("recording.wav")?)?;

// FLAC
#[cfg(feature = "flac")]
let buffer = brainwires_audio::decode_flac(&std::fs::read("recording.flac")?)?;

let playback = CpalPlayback::new();
playback.play(None, &buffer).await?;
```

### Text-to-Speech

```rust
use brainwires_audio::{TextToSpeech, TtsOptions, CpalPlayback, AudioPlayback, OpenAiTts};

let tts = OpenAiTts::new("your-api-key");
let audio = tts.synthesize("Hello from Brainwires!", &TtsOptions::default()).await?;

let playback = CpalPlayback::new();
playback.play(None, &audio).await?;
```

### Listing Audio Devices

```rust
use brainwires_audio::{CpalCapture, CpalPlayback, AudioCapture, AudioPlayback};

let capture = CpalCapture::new();
for dev in capture.list_devices()? {
    let tag = if dev.is_default { " (default)" } else { "" };
    println!("Input:  {}{tag}", dev.name);
}

let playback = CpalPlayback::new();
for dev in playback.list_devices()? {
    let tag = if dev.is_default { " (default)" } else { "" };
    println!("Output: {}{tag}", dev.name);
}
```

### Local Whisper STT

```rust
use brainwires_audio::{SpeechToText, SttOptions};

#[cfg(feature = "local-stt")]
{
    use brainwires_audio::WhisperStt;

    let stt = WhisperStt::new("models/ggml-base.en.bin");
    let transcript = stt.transcribe(&buffer, &SttOptions {
        language: Some("en".into()),
        timestamps: true,
        ..Default::default()
    }).await?;

    for seg in &transcript.segments {
        println!("[{:.1}s - {:.1}s] {}", seg.start, seg.end, seg.text);
    }
}
```

## Examples

Run the included examples:

```bash
# Record 5 seconds to WAV (default)
cargo run --example capture_audio

# Record 10 seconds to FLAC
cargo run --example capture_audio -- --duration 10 --format flac

# Play a WAV or FLAC file
cargo run --example play_audio -- recording.wav
cargo run --example play_audio -- recording.flac
```

## Integration with Brainwires

Use via the `brainwires` facade crate:

```toml
[dependencies]
brainwires = { version = "0.4", features = ["audio"] }
```

Or depend on `brainwires-audio` directly for standalone audio support without the rest of the framework.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.
