//! Local Models Setup Dialog
//!
//! Provides a startup dialog for first-time users to optionally download
//! local LLM models for improved performance and cost savings.

use anyhow::Result;
use console::style;
use dialoguer::{Confirm, MultiSelect, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;

use crate::providers::local_llm::{
    LocalLlmConfig, LocalModelRegistry, get_known_model, known_models,
};

/// Models recommended for download
const RECOMMENDED_MODELS: &[&str] = &["lfm2-350m", "lfm2-1.2b"];

/// Check if local models setup should be shown
/// Returns true if no models are installed and we're in an interactive terminal
pub fn should_show_setup() -> bool {
    // Only show in interactive terminals
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return false;
    }

    // Check if any models are installed
    match LocalModelRegistry::load() {
        Ok(registry) => registry.models.is_empty(),
        Err(_) => true, // Show setup if registry can't be loaded
    }
}

/// Show the local models setup dialog
/// Returns Ok(true) if setup was completed, Ok(false) if skipped
pub async fn show_setup_dialog() -> Result<bool> {
    // Clear some space and show header
    println!();
    println!("{}", style("━".repeat(70)).cyan());
    println!();
    println!(
        "  {} {}",
        style("🧠").bold(),
        style("Local LLM Setup").bold().cyan()
    );
    println!();
    println!("{}", style("━".repeat(70)).cyan());
    println!();

    // Explain the benefits
    println!(
        "{}",
        style("  Brainwires can use local AI models to improve your experience:").white()
    );
    println!();
    println!(
        "  {} {} - Route queries to the right tools faster",
        style("•").cyan(),
        style("Smart Query Routing").bold()
    );
    println!(
        "  {} {} - Score task complexity for optimal processing",
        style("•").cyan(),
        style("Complexity Analysis").bold()
    );
    println!(
        "  {} {} - Validate responses before showing them",
        style("•").cyan(),
        style("Response Validation").bold()
    );
    println!(
        "  {} {} - Summarize conversation context efficiently",
        style("•").cyan(),
        style("Context Summarization").bold()
    );
    println!(
        "  {} {} - Decide when to search conversation history",
        style("•").cyan(),
        style("Retrieval Gating").bold()
    );
    println!(
        "  {} {} - Re-rank results by relevance",
        style("•").cyan(),
        style("Relevance Scoring").bold()
    );
    println!();

    println!("{}", style("  Benefits:").white().bold());
    println!(
        "  {} Reduce API costs by up to {}",
        style("💰").bold(),
        style("80%").green().bold()
    );
    println!(
        "  {} Faster routing and validation ({})",
        style("⚡").bold(),
        style("~50ms local vs 1-2s API").dim()
    );
    println!(
        "  {} All processing stays {} - your data never leaves your machine",
        style("🔒").bold(),
        style("local").green()
    );
    println!();

    // Show available models
    println!("{}", style("  Recommended Models:").white().bold());
    println!();

    let models = known_models();
    let recommended: Vec<_> = models
        .iter()
        .filter(|m| RECOMMENDED_MODELS.contains(&m.id))
        .collect();

    for model in &recommended {
        let size_indicator = if model.id == "lfm2-350m" {
            style("FAST").green()
        } else {
            style("QUALITY").yellow()
        };

        println!(
            "  {} {} [{}]",
            style("→").cyan(),
            style(model.name).bold(),
            size_indicator
        );
        println!("      {}", style(model.description).dim());
        println!(
            "      RAM: ~{}MB | Context: {} tokens",
            model.estimated_ram_mb, model.context_size
        );
        println!();
    }

    // Calculate total download size (approximate)
    let total_ram: u32 = recommended.iter().map(|m| m.estimated_ram_mb).sum();
    println!(
        "  {} Total RAM usage: ~{}MB (models are loaded on demand)",
        style("ℹ").blue(),
        total_ram
    );
    println!();

    // Ask if user wants to download
    let theme = ColorfulTheme::default();
    let proceed = Confirm::with_theme(&theme)
        .with_prompt("Would you like to download these models now?")
        .default(true)
        .interact()?;

    if !proceed {
        println!();
        println!(
            "  {} No problem! You can download models later with:",
            style("ℹ").blue()
        );
        println!(
            "      {}",
            style("brainwires local-models download lfm2-350m").cyan()
        );
        println!(
            "      {}",
            style("brainwires local-models download lfm2-1.2b").cyan()
        );
        println!();
        println!("  {} Or see all available models with:", style("ℹ").blue());
        println!(
            "      {}",
            style("brainwires local-models list --available").cyan()
        );
        println!();

        // Mark setup as shown so we don't ask again
        mark_setup_shown()?;

        return Ok(false);
    }

    // Let user select which models to download
    println!();
    let model_options: Vec<String> = recommended
        .iter()
        .map(|m| {
            let size_indicator = if m.id == "lfm2-350m" {
                "Fast"
            } else {
                "Quality"
            };
            format!(
                "{} ({}) - ~{}MB RAM",
                m.name, size_indicator, m.estimated_ram_mb
            )
        })
        .collect();

    let selections = MultiSelect::with_theme(&theme)
        .with_prompt("Select models to download (space to toggle, enter to confirm)")
        .items(&model_options)
        .defaults(&[true, true]) // Both selected by default
        .interact()?;

    if selections.is_empty() {
        println!();
        println!(
            "  {} No models selected. You can download later with:",
            style("ℹ").blue()
        );
        println!(
            "      {}",
            style("brainwires local-models download <model_id>").cyan()
        );
        println!();
        mark_setup_shown()?;
        return Ok(false);
    }

    // Download selected models
    println!();
    println!("{}", style("  Downloading models...").white().bold());
    println!();

    let selected_models: Vec<_> = selections.iter().map(|&i| recommended[i]).collect();

    // Download models sequentially with progress bars
    let mut all_success = true;
    for model in &selected_models {
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(&format!(
                    "  {{spinner:.green}} {} [{{bar:30.cyan/blue}}] {{bytes}}/{{total_bytes}} ({{eta}})",
                    style(model.id).cyan()
                ))?
                .progress_chars("█▓░"),
        );

        let model_id = model.id.to_string();
        match download_model_with_progress(model_id, pb).await {
            Ok(()) => {}
            Err(e) => {
                eprintln!("  {} Download error: {}", style("✗").red(), e);
                all_success = false;
            }
        }
    }

    println!();

    if all_success {
        println!(
            "  {} All models downloaded successfully!",
            style("✓").green().bold()
        );
        println!();
        println!(
            "  {} Local inference is now enabled. Brainwires will automatically",
            style("ℹ").blue()
        );
        println!("      use local models for routing, validation, and summarization.");
        println!();
        println!(
            "  {} Manage models with: {}",
            style("→").cyan(),
            style("brainwires local-models --help").cyan()
        );
    } else {
        println!(
            "  {} Some downloads failed. You can retry with:",
            style("!").yellow()
        );
        println!(
            "      {}",
            style("brainwires local-models download <model_id>").cyan()
        );
    }

    println!();
    mark_setup_shown()?;

    Ok(all_success)
}

/// Download a model with progress bar
async fn download_model_with_progress(model_id: String, pb: ProgressBar) -> Result<()> {
    let known =
        get_known_model(&model_id).ok_or_else(|| anyhow::anyhow!("Unknown model: {}", model_id))?;

    let registry = LocalModelRegistry::load()?;
    let model_path = registry.models_dir.join(known.filename);

    // Ensure models directory exists
    std::fs::create_dir_all(&registry.models_dir)?;

    if model_path.exists() {
        pb.finish_with_message("Already installed");
        return Ok(());
    }

    // Download from Hugging Face
    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        known.huggingface_repo, known.filename
    );

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        pb.finish_with_message("Download failed");
        return Err(anyhow::anyhow!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    pb.set_length(total_size);

    // Stream to file
    let mut file = std::fs::File::create(&model_path)?;
    let mut stream = response.bytes_stream();
    use futures::StreamExt;
    use std::io::Write;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("Done");

    // Auto-register the model
    let config = LocalLlmConfig {
        id: known.id.to_string(),
        name: known.name.to_string(),
        model_path: model_path.clone(),
        context_size: known.context_size,
        model_type: known.model_type,
        supports_tools: known.supports_tools,
        estimated_ram_mb: Some(known.estimated_ram_mb),
        ..Default::default()
    };

    let mut registry = LocalModelRegistry::load()?;
    registry.register(config);

    // Set lfm2-350m as default (faster for most tasks)
    if registry.default_model.is_none() || model_id == "lfm2-350m" {
        registry.set_default(&model_id);
    }

    registry.save()?;

    Ok(())
}

/// Mark that setup dialog has been shown (so we don't ask again)
fn mark_setup_shown() -> Result<()> {
    let config_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("brainwires");

    std::fs::create_dir_all(&config_dir)?;

    let marker_path = config_dir.join(".local_models_setup_shown");
    std::fs::write(&marker_path, "1")?;

    Ok(())
}

/// Check if setup has already been shown
pub fn setup_already_shown() -> bool {
    let config_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("brainwires");

    config_dir.join(".local_models_setup_shown").exists()
}

/// Combined check: should we show setup dialog?
pub fn should_prompt_for_setup() -> bool {
    // Don't show if already shown before
    if setup_already_shown() {
        return false;
    }

    // Show if no models installed and interactive terminal
    should_show_setup()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommended_models_exist() {
        let models = known_models();
        for id in RECOMMENDED_MODELS {
            assert!(
                models.iter().any(|m| m.id == *id),
                "Recommended model {} not found in known_models",
                id
            );
        }
    }
}
