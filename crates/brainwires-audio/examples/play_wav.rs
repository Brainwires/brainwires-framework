//! Load a WAV file and play it through the default speaker.
//!
//! Usage:
//!   cargo run --example play_wav -- recording.wav
//!   cargo run --example play_wav -- --file recording.wav

use brainwires_audio::{CpalPlayback, AudioPlayback, decode_wav};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let path = wav_path_from_args()?;

    let wav_bytes = std::fs::read(&path)?;
    let buffer = decode_wav(&wav_bytes)?;

    println!(
        "Loaded {} ({:.2}s, {} Hz, {} ch, {:?})",
        path,
        buffer.duration_secs(),
        buffer.config.sample_rate,
        buffer.config.channels,
        buffer.config.sample_format,
    );

    let playback = CpalPlayback::new();

    // Show available output devices
    let devices = playback.list_devices()?;
    println!("Output devices:");
    for dev in &devices {
        let marker = if dev.is_default { " (default)" } else { "" };
        println!("  - {}{marker}", dev.name);
    }

    println!("\nPlaying...");
    playback.play(None, &buffer).await?;
    println!("Done.");

    Ok(())
}

fn wav_path_from_args() -> anyhow::Result<String> {
    let args: Vec<String> = std::env::args().collect();

    // Support both `play_wav recording.wav` and `play_wav --file recording.wav`
    if let Some(i) = args.iter().position(|a| a == "--file") {
        if let Some(path) = args.get(i + 1) {
            return Ok(path.clone());
        }
    }

    // Positional: first non-flag argument after the binary name
    if let Some(path) = args.get(1) {
        if !path.starts_with('-') {
            return Ok(path.clone());
        }
    }

    anyhow::bail!("Usage: play_wav <file.wav>  or  play_wav --file <file.wav>")
}
