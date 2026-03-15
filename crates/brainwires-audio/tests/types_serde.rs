//! Integration tests for serialization/deserialization of audio types.

use brainwires_audio::{
    AudioDevice, DeviceDirection, OutputFormat, SttOptions, Transcript, TranscriptSegment,
    TtsOptions, Voice,
};

// ── Voice ────────────────────────────────────────────────────────────

#[test]
fn voice_new_sets_id() {
    let v = Voice::new("alloy");
    assert_eq!(v.id, "alloy");
    assert!(v.name.is_none());
    assert!(v.language.is_none());
}

#[test]
fn voice_new_accepts_string() {
    let v = Voice::new(String::from("echo"));
    assert_eq!(v.id, "echo");
}

#[test]
fn voice_serialization_roundtrip() {
    let v = Voice::new("shimmer");
    let json = serde_json::to_string(&v).unwrap();
    let restored: Voice = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, "shimmer");
    assert!(restored.name.is_none());
    assert!(restored.language.is_none());
}

#[test]
fn voice_with_all_fields() {
    let json = r#"{"id":"nova","name":"Nova","language":"en-US"}"#;
    let v: Voice = serde_json::from_str(json).unwrap();
    assert_eq!(v.id, "nova");
    assert_eq!(v.name.as_deref(), Some("Nova"));
    assert_eq!(v.language.as_deref(), Some("en-US"));
}

// ── OutputFormat ─────────────────────────────────────────────────────

#[test]
fn output_format_all_variants_serialize() {
    for fmt in [
        OutputFormat::Wav,
        OutputFormat::Mp3,
        OutputFormat::Pcm,
        OutputFormat::Opus,
        OutputFormat::Flac,
    ] {
        let json = serde_json::to_string(&fmt).unwrap();
        let restored: OutputFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(format!("{:?}", fmt), format!("{:?}", restored));
    }
}

// ── TtsOptions ───────────────────────────────────────────────────────

#[test]
fn tts_options_default_values() {
    let opts = TtsOptions::default();
    assert_eq!(opts.voice.id, "alloy");
    assert!(opts.speed.is_none());
    assert!(matches!(opts.output_format, OutputFormat::Wav));
}

#[test]
fn tts_options_serialization_roundtrip() {
    let opts = TtsOptions {
        voice: Voice::new("echo"),
        speed: Some(1.5),
        output_format: OutputFormat::Mp3,
    };
    let json = serde_json::to_string(&opts).unwrap();
    let restored: TtsOptions = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.voice.id, "echo");
    assert_eq!(restored.speed, Some(1.5));
    assert!(matches!(restored.output_format, OutputFormat::Mp3));
}

// ── SttOptions ───────────────────────────────────────────────────────

#[test]
fn stt_options_default_is_all_none() {
    let opts = SttOptions::default();
    assert!(opts.language.is_none());
    assert!(!opts.timestamps);
    assert!(opts.prompt.is_none());
}

#[test]
fn stt_options_serialization_roundtrip() {
    let opts = SttOptions {
        language: Some("en".to_string()),
        timestamps: true,
        prompt: Some("technical discussion".to_string()),
    };
    let json = serde_json::to_string(&opts).unwrap();
    let restored: SttOptions = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.language.as_deref(), Some("en"));
    assert!(restored.timestamps);
    assert_eq!(restored.prompt.as_deref(), Some("technical discussion"));
}

// ── Transcript ───────────────────────────────────────────────────────

#[test]
fn transcript_serialization_roundtrip() {
    let t = Transcript {
        text: "Hello world".to_string(),
        language: Some("en".to_string()),
        duration_secs: Some(1.5),
        segments: vec![TranscriptSegment {
            text: "Hello".to_string(),
            start: 0.0,
            end: 0.5,
        }],
    };
    let json = serde_json::to_string(&t).unwrap();
    let restored: Transcript = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.text, "Hello world");
    assert_eq!(restored.language.as_deref(), Some("en"));
    assert_eq!(restored.duration_secs, Some(1.5));
    assert_eq!(restored.segments.len(), 1);
    assert_eq!(restored.segments[0].text, "Hello");
    assert!((restored.segments[0].start - 0.0).abs() < f64::EPSILON);
    assert!((restored.segments[0].end - 0.5).abs() < f64::EPSILON);
}

#[test]
fn transcript_with_empty_segments() {
    let t = Transcript {
        text: "Hi".to_string(),
        language: None,
        duration_secs: None,
        segments: vec![],
    };
    let json = serde_json::to_string(&t).unwrap();
    let restored: Transcript = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.text, "Hi");
    assert!(restored.segments.is_empty());
}

// ── AudioDevice / DeviceDirection ────────────────────────────────────

#[test]
fn audio_device_serialization_roundtrip() {
    let dev = AudioDevice {
        id: "hw:0".to_string(),
        name: "Built-in Mic".to_string(),
        is_default: true,
        direction: DeviceDirection::Input,
    };
    let json = serde_json::to_string(&dev).unwrap();
    let restored: AudioDevice = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.id, "hw:0");
    assert_eq!(restored.name, "Built-in Mic");
    assert!(restored.is_default);
    assert_eq!(restored.direction, DeviceDirection::Input);
}

#[test]
fn device_direction_equality() {
    assert_eq!(DeviceDirection::Input, DeviceDirection::Input);
    assert_eq!(DeviceDirection::Output, DeviceDirection::Output);
    assert_ne!(DeviceDirection::Input, DeviceDirection::Output);
}

#[test]
fn device_direction_serialization() {
    let json_in = serde_json::to_string(&DeviceDirection::Input).unwrap();
    let json_out = serde_json::to_string(&DeviceDirection::Output).unwrap();
    let restored_in: DeviceDirection = serde_json::from_str(&json_in).unwrap();
    let restored_out: DeviceDirection = serde_json::from_str(&json_out).unwrap();
    assert_eq!(restored_in, DeviceDirection::Input);
    assert_eq!(restored_out, DeviceDirection::Output);
}
