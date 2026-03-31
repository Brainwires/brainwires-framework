//! Demonstrate wake word detection from the default microphone.
//!
//! Requires a `.rpw` model file (create one with `rustpotter-cli`).
//!
//! Run with:
//! ```bash
//! cargo run -p brainwires-hardware --example wake_word_demo \
//!     --features wake-word-rustpotter -- --model hey_assistant.rpw
//! ```
//!
//! Press Ctrl-C to stop.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use brainwires_hardware::audio::{
    capture::AudioCapture,
    hardware::cpal_capture::CpalCapture,
    types::AudioConfig,
    vad::pcm_to_i16_mono,
    wake_word::{RustpotterDetector, WakeWordDetector},
};
use futures::StreamExt;

#[derive(Debug)]
struct Args {
    model: PathBuf,
    threshold: f32,
}

fn parse_args() -> Args {
    let args: Vec<String> = std::env::args().collect();
    let mut model = PathBuf::from("wake_word.rpw");
    let mut threshold = 0.5f32;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--model" | "-m" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    model = PathBuf::from(v);
                }
            }
            "--threshold" | "-t" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    threshold = v.parse().unwrap_or(0.5);
                }
            }
            _ => {}
        }
        i += 1;
    }

    Args { model, threshold }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = parse_args();

    println!("Loading wake word model: {}", args.model.display());
    let mut detector = RustpotterDetector::from_model_file(&args.model, args.threshold)?;
    println!(
        "Wake word detector ready — frame size: {} samples ({}ms at 16kHz)",
        detector.frame_size(),
        detector.frame_size() * 1000 / 16000
    );

    let capture = CpalCapture;
    let config = AudioConfig::speech(); // 16kHz mono i16

    let mut stream = capture.start_capture(None, &config)?;

    let running = Arc::new(AtomicBool::new(true));
    let r = Arc::clone(&running);
    ctrlc::set_handler(move || r.store(false, Ordering::Relaxed)).unwrap_or_default();

    println!("Listening for wake word... (Ctrl-C to stop)");

    let frame_size = detector.frame_size();
    let mut sample_buf: Vec<i16> = Vec::new();

    while running.load(Ordering::Relaxed) {
        match tokio::time::timeout(std::time::Duration::from_millis(100), stream.next()).await {
            Ok(Some(Ok(audio_buf))) => {
                let mono = pcm_to_i16_mono(&audio_buf);
                sample_buf.extend_from_slice(&mono);

                while sample_buf.len() >= frame_size {
                    let frame: Vec<i16> = sample_buf.drain(..frame_size).collect();
                    if let Some(det) = detector.process_frame(&frame) {
                        println!(
                            "[{:.1}s] Wake word detected: \"{}\" (score: {:.3})",
                            det.timestamp_ms as f64 / 1000.0,
                            det.keyword,
                            det.score,
                        );
                    }
                }
            }
            Ok(Some(Err(e))) => eprintln!("Capture error: {e}"),
            Ok(None) => break,
            Err(_) => {} // timeout — check running flag
        }
    }

    println!("Done.");
    Ok(())
}
