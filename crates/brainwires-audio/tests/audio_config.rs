//! Integration tests for AudioConfig construction, presets, and calculations.

use brainwires_audio::{AudioConfig, SampleFormat};

// ── Preset constructors ──────────────────────────────────────────────

#[test]
fn speech_preset_is_16khz_mono_i16() {
    let cfg = AudioConfig::speech();
    assert_eq!(cfg.sample_rate, 16_000);
    assert_eq!(cfg.channels, 1);
    assert_eq!(cfg.sample_format, SampleFormat::I16);
}

#[test]
fn cd_quality_preset_is_44100_stereo_i16() {
    let cfg = AudioConfig::cd_quality();
    assert_eq!(cfg.sample_rate, 44_100);
    assert_eq!(cfg.channels, 2);
    assert_eq!(cfg.sample_format, SampleFormat::I16);
}

#[test]
fn high_quality_preset_is_48khz_stereo_f32() {
    let cfg = AudioConfig::high_quality();
    assert_eq!(cfg.sample_rate, 48_000);
    assert_eq!(cfg.channels, 2);
    assert_eq!(cfg.sample_format, SampleFormat::F32);
}

#[test]
fn default_config_matches_speech() {
    let def = AudioConfig::default();
    let speech = AudioConfig::speech();
    assert_eq!(def.sample_rate, speech.sample_rate);
    assert_eq!(def.channels, speech.channels);
    assert_eq!(def.sample_format, speech.sample_format);
}

// ── Bytes per sample / frame ─────────────────────────────────────────

#[test]
fn bytes_per_sample_i16_is_2() {
    let cfg = AudioConfig {
        sample_rate: 8000,
        channels: 1,
        sample_format: SampleFormat::I16,
    };
    assert_eq!(cfg.bytes_per_sample(), 2);
}

#[test]
fn bytes_per_sample_f32_is_4() {
    let cfg = AudioConfig {
        sample_rate: 8000,
        channels: 1,
        sample_format: SampleFormat::F32,
    };
    assert_eq!(cfg.bytes_per_sample(), 4);
}

#[test]
fn bytes_per_frame_mono_i16() {
    let cfg = AudioConfig {
        sample_rate: 16_000,
        channels: 1,
        sample_format: SampleFormat::I16,
    };
    assert_eq!(cfg.bytes_per_frame(), 2);
}

#[test]
fn bytes_per_frame_stereo_i16() {
    let cfg = AudioConfig {
        sample_rate: 44_100,
        channels: 2,
        sample_format: SampleFormat::I16,
    };
    assert_eq!(cfg.bytes_per_frame(), 4);
}

#[test]
fn bytes_per_frame_stereo_f32() {
    let cfg = AudioConfig {
        sample_rate: 48_000,
        channels: 2,
        sample_format: SampleFormat::F32,
    };
    assert_eq!(cfg.bytes_per_frame(), 8);
}

#[test]
fn bytes_per_frame_many_channels() {
    let cfg = AudioConfig {
        sample_rate: 48_000,
        channels: 6, // 5.1 surround
        sample_format: SampleFormat::F32,
    };
    assert_eq!(cfg.bytes_per_frame(), 24); // 4 * 6
}

// ── Serialization round-trip ─────────────────────────────────────────

#[test]
fn audio_config_serializes_and_deserializes() {
    let cfg = AudioConfig::high_quality();
    let json = serde_json::to_string(&cfg).unwrap();
    let restored: AudioConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.sample_rate, cfg.sample_rate);
    assert_eq!(restored.channels, cfg.channels);
    assert_eq!(restored.sample_format, cfg.sample_format);
}

#[test]
fn sample_format_serializes_as_string() {
    let json = serde_json::to_string(&SampleFormat::I16).unwrap();
    assert!(json.contains("I16"));
    let json_f32 = serde_json::to_string(&SampleFormat::F32).unwrap();
    assert!(json_f32.contains("F32"));
}
