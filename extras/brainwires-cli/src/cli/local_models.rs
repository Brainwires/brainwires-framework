//! CLI commands for managing local LLM models
//!
//! Provides commands to list, download, register, and configure local models
//! for CPU-based inference.

use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use console::{Term, style};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

use crate::providers::local_llm::{
    LocalLlmConfig, LocalModelRegistry, get_known_model, known_models,
};

#[derive(Subcommand)]
pub enum LocalModelCommands {
    /// List registered local models
    List {
        /// Show available (downloadable) models instead of installed
        #[arg(long)]
        available: bool,

        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Download a model from Hugging Face
    Download {
        /// Model ID to download (use 'list --available' to see options)
        model_id: String,

        /// Custom path to save the model (default: ~/.local/share/brainwires/models/)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Quantization level (q4_0, q4_1, q5_0, q5_1, q8_0)
        #[arg(short, long, default_value = "q8_0")]
        quantization: String,
    },

    /// Register an existing GGUF model file
    Register {
        /// Path to the GGUF model file
        path: PathBuf,

        /// Custom ID for the model (default: derived from filename)
        #[arg(long)]
        id: Option<String>,

        /// Human-readable name for the model
        #[arg(long)]
        name: Option<String>,

        /// Context window size
        #[arg(long)]
        context_size: Option<u32>,

        /// Set as default model
        #[arg(short, long)]
        default: bool,
    },

    /// Remove a model from the registry
    Remove {
        /// Model ID to remove
        model_id: String,

        /// Also delete the model file
        #[arg(long)]
        delete_file: bool,
    },

    /// Set the default local model
    Default {
        /// Model ID to set as default
        model_id: String,
    },

    /// Show detailed information about a model
    Info {
        /// Model ID to show info for
        model_id: String,
    },

    /// Scan the models directory for unregistered GGUF files
    Scan,

    /// Test a local model with a simple prompt
    Test {
        /// Model ID to test
        model_id: String,

        /// Test prompt (default: "Hello, please respond with 'ok'")
        #[arg(short, long)]
        prompt: Option<String>,
    },
}

/// Handle local model CLI commands
pub async fn handle_local_models(command: Option<LocalModelCommands>) -> Result<()> {
    match command {
        Some(LocalModelCommands::List { available, verbose }) => {
            if available {
                list_available_models(verbose)
            } else {
                list_installed_models(verbose)
            }
        }
        Some(LocalModelCommands::Download {
            model_id,
            path,
            quantization,
        }) => download_model(&model_id, path, &quantization).await,
        Some(LocalModelCommands::Register {
            path,
            id,
            name,
            context_size,
            default,
        }) => register_model(path, id, name, context_size, default),
        Some(LocalModelCommands::Remove {
            model_id,
            delete_file,
        }) => remove_model(&model_id, delete_file),
        Some(LocalModelCommands::Default { model_id }) => set_default(&model_id),
        Some(LocalModelCommands::Info { model_id }) => show_info(&model_id),
        Some(LocalModelCommands::Scan) => scan_models(),
        Some(LocalModelCommands::Test { model_id, prompt }) => test_model(&model_id, prompt).await,
        None => list_installed_models(false),
    }
}

fn list_installed_models(verbose: bool) -> Result<()> {
    let registry = LocalModelRegistry::load()?;
    let _term = Term::stdout();

    if registry.models.is_empty() {
        println!("{}", style("No local models installed.").yellow());
        println!();
        println!(
            "Use {} to see downloadable models",
            style("brainwires local-models list --available").cyan()
        );
        println!(
            "Use {} to download a model",
            style("brainwires local-models download <model_id>").cyan()
        );
        return Ok(());
    }

    println!("{}", style("Installed Local Models").bold());
    println!("{}", style("─".repeat(60)).dim());

    let default_id = registry.default_model.as_deref();

    for model in registry.list() {
        let is_default = default_id == Some(model.id.as_str());
        let default_marker = if is_default {
            style(" (default)").green()
        } else {
            style("").dim()
        };

        let ram = model
            .estimated_ram_mb
            .map(|r| format!("~{}MB", r))
            .unwrap_or_else(|| "?".to_string());

        println!(
            "  {} {}{}",
            style(&model.id).cyan().bold(),
            style(&model.name).white(),
            default_marker
        );

        if verbose {
            println!("    Path: {}", style(model.model_path.display()).dim());
            println!(
                "    Context: {} tokens, RAM: {}, Tools: {}",
                model.context_size,
                ram,
                if model.supports_tools { "yes" } else { "no" }
            );
            println!("    Type: {:?}", model.model_type);
            println!();
        }
    }

    if !verbose {
        println!();
        println!("{}", style("Use --verbose for more details").dim());
    }

    Ok(())
}

fn list_available_models(verbose: bool) -> Result<()> {
    let models = known_models();

    println!("{}", style("Available Models for Download").bold());
    println!("{}", style("─".repeat(60)).dim());
    println!();

    for model in models {
        println!(
            "  {} - {}",
            style(model.id).cyan().bold(),
            style(model.name).white()
        );
        println!("    {}", style(model.description).dim());

        if verbose {
            println!("    Repo: {}", style(model.huggingface_repo).dim());
            println!(
                "    Context: {} tokens, RAM: ~{}MB",
                model.context_size, model.estimated_ram_mb
            );
            println!(
                "    Tools: {}",
                if model.supports_tools { "yes" } else { "no" }
            );
        }
        println!();
    }

    println!(
        "Download with: {}",
        style("brainwires local-models download <model_id>").cyan()
    );

    Ok(())
}

async fn download_model(
    model_id: &str,
    custom_path: Option<PathBuf>,
    _quantization: &str,
) -> Result<()> {
    let known = get_known_model(model_id).ok_or_else(|| {
        anyhow!(
            "Unknown model: {}. Use 'list --available' to see options.",
            model_id
        )
    })?;

    let registry = LocalModelRegistry::load()?;
    let models_dir = custom_path.unwrap_or_else(|| registry.models_dir.clone());

    // Ensure models directory exists
    std::fs::create_dir_all(&models_dir).context("Failed to create models directory")?;

    let model_path = models_dir.join(known.filename);

    if model_path.exists() {
        println!(
            "{} Model already exists at {}",
            style("!").yellow(),
            model_path.display()
        );
        return Ok(());
    }

    println!(
        "{} Downloading {} from {}...",
        style("→").cyan(),
        style(known.name).bold(),
        known.huggingface_repo
    );

    // Create progress bar
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("#>-"),
    );

    // Download using huggingface-hub API
    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        known.huggingface_repo, known.filename
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to start download")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download failed with status: {}. Model may not be available.",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    pb.set_length(total_size);

    // Stream to file
    let mut file = std::fs::File::create(&model_path).context("Failed to create model file")?;

    let mut stream = response.bytes_stream();
    use futures::StreamExt;
    use std::io::Write;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Error reading download stream")?;
        file.write_all(&chunk).context("Error writing model file")?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("Download complete");

    println!(
        "{} Downloaded to {}",
        style("✓").green(),
        model_path.display()
    );

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

    // Set as default if it's the first model
    if registry.default_model.is_none() {
        registry.set_default(known.id);
    }

    registry.save()?;

    println!("{} Registered as '{}'", style("✓").green(), known.id);

    Ok(())
}

fn register_model(
    path: PathBuf,
    id: Option<String>,
    name: Option<String>,
    context_size: Option<u32>,
    set_default: bool,
) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("Model file not found: {}", path.display()));
    }

    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let model_id = id.unwrap_or_else(|| filename.to_string());
    let model_name = name.unwrap_or_else(|| filename.to_string());

    let config = LocalLlmConfig {
        id: model_id.clone(),
        name: model_name,
        model_path: path.canonicalize()?,
        context_size: context_size.unwrap_or(4096),
        ..Default::default()
    };

    let mut registry = LocalModelRegistry::load()?;
    registry.register(config);

    if set_default || registry.default_model.is_none() {
        registry.set_default(&model_id);
    }

    registry.save()?;

    println!("{} Registered model '{}'", style("✓").green(), model_id);

    Ok(())
}

fn remove_model(model_id: &str, delete_file: bool) -> Result<()> {
    let mut registry = LocalModelRegistry::load()?;

    let config = registry
        .remove(model_id)
        .ok_or_else(|| anyhow!("Model '{}' not found", model_id))?;

    if delete_file && config.model_path.exists() {
        std::fs::remove_file(&config.model_path).context("Failed to delete model file")?;
        println!(
            "{} Deleted model file: {}",
            style("✓").green(),
            config.model_path.display()
        );
    }

    registry.save()?;

    println!("{} Removed model '{}'", style("✓").green(), model_id);

    Ok(())
}

fn set_default(model_id: &str) -> Result<()> {
    let mut registry = LocalModelRegistry::load()?;

    if !registry.set_default(model_id) {
        return Err(anyhow!("Model '{}' not found", model_id));
    }

    registry.save()?;

    println!(
        "{} Set '{}' as default local model",
        style("✓").green(),
        model_id
    );

    Ok(())
}

fn show_info(model_id: &str) -> Result<()> {
    let registry = LocalModelRegistry::load()?;

    let config = registry
        .get(model_id)
        .ok_or_else(|| anyhow!("Model '{}' not found", model_id))?;

    println!("{}", style("Model Information").bold());
    println!("{}", style("─".repeat(40)).dim());
    println!("ID:           {}", style(&config.id).cyan());
    println!("Name:         {}", config.name);
    println!("Path:         {}", config.model_path.display());
    println!("Type:         {:?}", config.model_type);
    println!("Context Size: {} tokens", config.context_size);
    println!("Batch Size:   {}", config.batch_size);
    println!("Max Tokens:   {}", config.max_tokens);
    println!("GPU Layers:   {}", config.gpu_layers);
    println!(
        "Memory Map:   {}",
        if config.use_mmap { "yes" } else { "no" }
    );
    println!(
        "Memory Lock:  {}",
        if config.use_mlock { "yes" } else { "no" }
    );
    println!(
        "Tools:        {}",
        if config.supports_tools { "yes" } else { "no" }
    );

    if let Some(ram) = config.estimated_ram_mb {
        println!("Est. RAM:     ~{}MB", ram);
    }

    let is_default = registry.default_model.as_deref() == Some(model_id);
    println!("Default:      {}", if is_default { "yes" } else { "no" });

    Ok(())
}

fn scan_models() -> Result<()> {
    let mut registry = LocalModelRegistry::load()?;

    println!(
        "{} Scanning {} for GGUF files...",
        style("→").cyan(),
        registry.models_dir.display()
    );

    let discovered = registry.scan_models_dir()?;

    if discovered.is_empty() {
        println!("{} No new models found", style("!").yellow());
    } else {
        for id in &discovered {
            println!("{} Found: {}", style("✓").green(), id);
        }
        registry.save()?;
        println!(
            "{} Registered {} new model(s)",
            style("✓").green(),
            discovered.len()
        );
    }

    Ok(())
}

async fn test_model(model_id: &str, custom_prompt: Option<String>) -> Result<()> {
    use crate::providers::ProviderFactory;
    use crate::types::provider::ChatOptions;

    let prompt = custom_prompt.unwrap_or_else(|| {
        "Hello! Please respond with just 'ok' to confirm you're working.".to_string()
    });

    println!("{} Testing model '{}'...", style("→").cyan(), model_id);

    let factory = ProviderFactory::new();
    let provider = factory.create_local(model_id)?;

    println!("  Prompt: {}", style(&prompt).dim());
    println!();

    let messages = vec![crate::types::message::Message::user(&prompt)];
    let options = ChatOptions {
        temperature: Some(0.1),
        max_tokens: Some(100),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let response = provider.chat(&messages, None, &options).await?;
    let elapsed = start.elapsed();

    println!(
        "  Response: {}",
        style(response.message.text().unwrap_or("(no text)")).green()
    );
    println!();
    println!(
        "  Time: {:?}, Tokens: {} in / {} out",
        elapsed, response.usage.prompt_tokens, response.usage.completion_tokens
    );

    println!(
        "\n{} Model test completed successfully!",
        style("✓").green()
    );

    Ok(())
}
