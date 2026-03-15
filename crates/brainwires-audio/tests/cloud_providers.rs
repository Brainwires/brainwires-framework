//! Integration tests for cloud provider construction and trait conformance.
//!
//! These tests verify that cloud API provider types can be constructed and
//! satisfy the `TextToSpeech` / `SpeechToText` trait bounds. Tests that would
//! actually call cloud APIs are marked `#[ignore]`.
//!
//! Only compiled when the `native` feature is enabled (on by default).

#![cfg(feature = "native")]

use brainwires_audio::{
    AudioBuffer, AudioConfig, OpenAiStt, OpenAiTts, SpeechToText, SttOptions, TextToSpeech,
    TtsOptions,
};

// ── Trait object construction ────────────────────────────────────────

#[test]
fn openai_tts_is_text_to_speech() {
    let tts = OpenAiTts::new("test-key");
    let _: Box<dyn TextToSpeech> = Box::new(tts);
}

#[test]
fn openai_stt_is_speech_to_text() {
    let stt = OpenAiStt::new("test-key");
    let _: Box<dyn SpeechToText> = Box::new(stt);
}

#[test]
fn openai_tts_name() {
    let tts = OpenAiTts::new("test-key");
    assert!(!tts.name().is_empty());
}

#[test]
fn openai_stt_name() {
    let stt = OpenAiStt::new("test-key");
    assert!(!stt.name().is_empty());
}

// ── ElevenLabs ───────────────────────────────────────────────────────

#[test]
fn elevenlabs_tts_is_text_to_speech() {
    use brainwires_audio::ElevenLabsTts;
    let tts = ElevenLabsTts::new("test-key");
    let _: Box<dyn TextToSpeech> = Box::new(tts);
}

#[test]
fn elevenlabs_stt_is_speech_to_text() {
    use brainwires_audio::ElevenLabsStt;
    let stt = ElevenLabsStt::new("test-key");
    let _: Box<dyn SpeechToText> = Box::new(stt);
}

// ── Deepgram ─────────────────────────────────────────────────────────

#[test]
fn deepgram_tts_is_text_to_speech() {
    use brainwires_audio::DeepgramTts;
    let tts = DeepgramTts::new("test-key");
    let _: Box<dyn TextToSpeech> = Box::new(tts);
}

#[test]
fn deepgram_stt_is_speech_to_text() {
    use brainwires_audio::DeepgramStt;
    let stt = DeepgramStt::new("test-key");
    let _: Box<dyn SpeechToText> = Box::new(stt);
}

// ── Google ────────────────────────────────────────────────────────────

#[test]
fn google_tts_is_text_to_speech() {
    use brainwires_audio::GoogleTts;
    let tts = GoogleTts::new("test-key");
    let _: Box<dyn TextToSpeech> = Box::new(tts);
}

// ── Azure ────────────────────────────────────────────────────────────

#[test]
fn azure_tts_is_text_to_speech() {
    use brainwires_audio::AzureTts;
    let tts = AzureTts::new("test-key", "eastus");
    let _: Box<dyn TextToSpeech> = Box::new(tts);
}

#[test]
fn azure_stt_is_speech_to_text() {
    use brainwires_audio::AzureStt;
    let stt = AzureStt::new("test-key", "eastus");
    let _: Box<dyn SpeechToText> = Box::new(stt);
}

// ── Fish ─────────────────────────────────────────────────────────────

#[test]
fn fish_tts_is_text_to_speech() {
    use brainwires_audio::FishTts;
    let tts = FishTts::new("test-key");
    let _: Box<dyn TextToSpeech> = Box::new(tts);
}

#[test]
fn fish_stt_is_speech_to_text() {
    use brainwires_audio::FishStt;
    let stt = FishStt::new("test-key");
    let _: Box<dyn SpeechToText> = Box::new(stt);
}

// ── Cartesia ─────────────────────────────────────────────────────────

#[test]
fn cartesia_tts_is_text_to_speech() {
    use brainwires_audio::CartesiaTts;
    let tts = CartesiaTts::new("test-key");
    let _: Box<dyn TextToSpeech> = Box::new(tts);
}

// ── Murf ─────────────────────────────────────────────────────────────

#[test]
fn murf_tts_is_text_to_speech() {
    use brainwires_audio::MurfTts;
    let tts = MurfTts::new("test-key");
    let _: Box<dyn TextToSpeech> = Box::new(tts);
}

// ── Ignored: actual API calls ────────────────────────────────────────

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY environment variable"]
async fn openai_tts_synthesize_live() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let tts = OpenAiTts::new(api_key);
    let opts = TtsOptions::default();
    let result = tts.synthesize("Hello, integration test.", &opts).await;
    assert!(result.is_ok());
    let buffer = result.unwrap();
    assert!(!buffer.is_empty());
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY environment variable"]
async fn openai_stt_transcribe_live() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let stt = OpenAiStt::new(&api_key);

    // Generate a short WAV with silence for testing
    let cfg = AudioConfig::speech();
    let data = vec![0u8; 32_000]; // 1 second of silence
    let buffer = AudioBuffer::from_pcm(data, cfg);

    let opts = SttOptions::default();
    let result = stt.transcribe(&buffer, &opts).await;
    // Even silence should not error; the transcript may be empty.
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY environment variable"]
async fn openai_tts_list_voices_live() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let tts = OpenAiTts::new(api_key);
    let voices = tts.list_voices().await;
    assert!(voices.is_ok());
    assert!(!voices.unwrap().is_empty());
}
