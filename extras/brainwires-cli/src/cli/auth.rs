use anyhow::{Result, anyhow};
use clap::Subcommand;
use dialoguer::Password;

use crate::auth::{AuthClient, SessionManager};
use crate::config::{ConfigManager, ConfigUpdates, ModelService, constants};
use crate::providers::ProviderType;
use crate::utils::logger::Logger;
use crate::utils::rich_output::RichOutput;

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Login with API key (Brainwires SaaS) or connect to a direct provider
    Login {
        #[arg(short, long)]
        key: Option<String>,

        /// Override backend URL (auto-detected from API key prefix by default)
        #[arg(short, long)]
        backend: Option<String>,

        /// Connect directly to a provider instead of Brainwires SaaS
        /// Supported: anthropic, openai, google, groq, ollama, bedrock, vertex-ai
        #[arg(short, long)]
        provider: Option<String>,

        /// Base URL for the provider (e.g., Ollama URL, custom OpenAI-compatible endpoint)
        #[arg(long)]
        base_url: Option<String>,

        /// Default model to use with this provider
        #[arg(short, long)]
        model: Option<String>,

        /// AWS region (Bedrock) or GCP region (Vertex AI)
        #[arg(long)]
        region: Option<String>,

        /// GCP project ID (Vertex AI)
        #[arg(long)]
        project_id: Option<String>,
    },

    /// Logout and clear session
    Logout,

    /// Show authentication status
    Status {
        #[arg(short, long)]
        verbose: bool,
    },

    /// Validate current session or provider connection
    Validate,

    /// Refresh provider API keys
    Refresh,
}

pub async fn handle_auth(cmd: AuthCommands) -> Result<()> {
    match cmd {
        AuthCommands::Login {
            key,
            backend,
            provider,
            base_url,
            model,
            region,
            project_id,
        } => {
            if let Some(provider_name) = provider {
                handle_direct_provider_login(
                    provider_name,
                    key,
                    base_url,
                    model,
                    region,
                    project_id,
                )
                .await
            } else {
                handle_brainwires_login(key, backend).await
            }
        }

        AuthCommands::Logout => {
            let config_manager = ConfigManager::new()?;
            let provider_type = config_manager.get().provider_type;

            match provider_type {
                ProviderType::Brainwires => {
                    if !SessionManager::is_authenticated()? {
                        Logger::warn("Not currently logged in");
                        return Ok(());
                    }
                    AuthClient::logout()?;
                }
                ProviderType::Ollama | ProviderType::Bedrock | ProviderType::VertexAI => {
                    // No key to clear — these use credential chains
                }
                _ => {
                    // Delete provider API key from keyring
                    if let Err(e) = config_manager.delete_provider_api_key(provider_type) {
                        tracing::debug!("Could not delete provider key: {}", e);
                    }
                }
            }

            // Reset to Brainwires default
            let mut config_manager = config_manager;
            config_manager.update(ConfigUpdates {
                provider_type: Some(ProviderType::Brainwires),
                model: Some(ProviderType::Brainwires.default_model().to_string()),
                provider_base_url: Some(None),
                ..Default::default()
            });
            config_manager.save()?;

            println!();
            println!(
                "{}",
                RichOutput::boxed(
                    "You have been logged out successfully.\nProvider reset to Brainwires (default).",
                    Some("Logout"),
                    "cyan",
                )
            );
            Ok(())
        }

        AuthCommands::Status { verbose } => {
            let config_manager = ConfigManager::new()?;
            let config = config_manager.get();

            match config.provider_type {
                ProviderType::Brainwires => {
                    show_brainwires_status(verbose)?;
                }
                ProviderType::Ollama => {
                    let base_url = config
                        .provider_base_url
                        .as_deref()
                        .unwrap_or("http://localhost:11434");
                    let status_text = format!(
                        "Provider: Ollama (local)\nModel: {}\nBase URL: {}",
                        config.model, base_url,
                    );
                    println!();
                    println!(
                        "{}",
                        RichOutput::boxed(&status_text, Some("Provider Status"), "green")
                    );
                }
                ProviderType::Bedrock => {
                    let region = config
                        .extra
                        .get("provider_options")
                        .and_then(|o| o.get("region"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("us-east-1");
                    let has_creds = std::env::var("AWS_ACCESS_KEY_ID").is_ok();
                    let status_text = format!(
                        "Provider: Amazon Bedrock\nModel: {}\nRegion: {}\nAWS Credentials: {}",
                        config.model,
                        region,
                        if has_creds {
                            "available (env)"
                        } else {
                            "NOT found in env"
                        },
                    );
                    println!();
                    println!(
                        "{}",
                        RichOutput::boxed(
                            &status_text,
                            Some("Provider Status"),
                            if has_creds { "green" } else { "yellow" },
                        )
                    );
                }
                ProviderType::VertexAI => {
                    let opts = config.extra.get("provider_options");
                    let project_id = opts
                        .and_then(|o| o.get("project_id"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("(not set)");
                    let region = opts
                        .and_then(|o| o.get("region"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("us-central1");
                    let has_creds = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok();
                    let status_text = format!(
                        "Provider: Google Vertex AI\nModel: {}\nProject: {}\nRegion: {}\nGCP Credentials: {}",
                        config.model,
                        project_id,
                        region,
                        if has_creds {
                            "available (env)"
                        } else {
                            "using gcloud auth (if available)"
                        },
                    );
                    println!();
                    println!(
                        "{}",
                        RichOutput::boxed(&status_text, Some("Provider Status"), "green")
                    );
                }
                _ => {
                    let has_key = config_manager.get_provider_api_key()?.is_some();
                    let mut status_text = format!(
                        "Provider: {}\nModel: {}\nAPI Key: {}",
                        config.provider_type.as_str(),
                        config.model,
                        if has_key {
                            "configured"
                        } else {
                            "NOT configured"
                        },
                    );
                    if let Some(ref url) = config.provider_base_url {
                        status_text.push_str(&format!("\nBase URL: {}", url));
                    }
                    println!();
                    println!(
                        "{}",
                        RichOutput::boxed(
                            &status_text,
                            Some("Provider Status"),
                            if has_key { "green" } else { "yellow" },
                        )
                    );
                }
            }

            Ok(())
        }

        AuthCommands::Validate => {
            let config_manager = ConfigManager::new()?;
            let config = config_manager.get();

            match config.provider_type {
                ProviderType::Brainwires => {
                    validate_brainwires_session().await?;
                }
                ProviderType::Ollama => {
                    let base_url = config
                        .provider_base_url
                        .as_deref()
                        .unwrap_or("http://localhost:11434");
                    Logger::info(format!("Checking Ollama at {}...", base_url));
                    match reqwest::get(&format!("{}/api/tags", base_url)).await {
                        Ok(resp) if resp.status().is_success() => {
                            Logger::success("Ollama is reachable and responding");
                        }
                        Ok(resp) => {
                            Logger::error(format!(
                                "Ollama responded with status: {}",
                                resp.status()
                            ));
                        }
                        Err(e) => {
                            Logger::error(format!("Cannot reach Ollama: {}", e));
                        }
                    }
                }
                ProviderType::Bedrock => {
                    if std::env::var("AWS_ACCESS_KEY_ID").is_ok() {
                        Logger::success("Bedrock: AWS credentials found in environment");
                    } else {
                        Logger::error(
                            "Bedrock: AWS_ACCESS_KEY_ID not set. Configure AWS credentials.",
                        );
                    }
                }
                ProviderType::VertexAI => {
                    let has_adc = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok();
                    let has_project = config
                        .extra
                        .get("provider_options")
                        .and_then(|o| o.get("project_id"))
                        .and_then(|v| v.as_str())
                        .is_some()
                        || std::env::var("GOOGLE_CLOUD_PROJECT").is_ok();
                    if has_project {
                        Logger::success("Vertex AI: project ID configured");
                    } else {
                        Logger::error("Vertex AI: no project ID configured");
                    }
                    if has_adc {
                        Logger::success("Vertex AI: GOOGLE_APPLICATION_CREDENTIALS found");
                    } else {
                        Logger::warn(
                            "Vertex AI: GOOGLE_APPLICATION_CREDENTIALS not set (will try gcloud auth)",
                        );
                    }
                }
                _ => {
                    let has_key = config_manager.get_provider_api_key()?.is_some();
                    if has_key {
                        Logger::success(format!(
                            "{} provider is configured with an API key",
                            config.provider_type.as_str()
                        ));
                    } else {
                        Logger::error(format!(
                            "No API key configured for {}. Run: brainwires auth login --provider {}",
                            config.provider_type.as_str(),
                            config.provider_type.as_str()
                        ));
                    }
                }
            }

            Ok(())
        }

        AuthCommands::Refresh => {
            if !SessionManager::is_authenticated()? {
                Logger::error("Not authenticated");
                return Ok(());
            }

            Logger::info("Refreshing provider API keys...");

            let config = ConfigManager::new()?;
            let client = AuthClient::new(config.get().backend_url.clone());

            client.refresh_provider_keys().await?;
            Logger::success("Provider API keys refreshed");

            Ok(())
        }
    }
}

/// Handle Brainwires SaaS login (existing flow).
async fn handle_brainwires_login(key: Option<String>, backend: Option<String>) -> Result<()> {
    let api_key = if let Some(k) = key {
        k
    } else {
        Password::new()
            .with_prompt("Enter your Brainwires API key")
            .interact()?
    };

    let backend_url =
        backend.unwrap_or_else(|| constants::get_backend_from_api_key(&api_key).to_string());

    Logger::info(format!("Authenticating with {}...", backend_url));

    let client = AuthClient::new(backend_url.clone());
    match client.authenticate(&api_key).await {
        Ok(session) => {
            // Ensure config reflects Brainwires provider
            let mut config_manager = ConfigManager::new()?;
            config_manager.update(ConfigUpdates {
                provider_type: Some(ProviderType::Brainwires),
                provider_base_url: Some(None),
                ..Default::default()
            });
            config_manager.save()?;

            println!();
            println!(
                "{}",
                RichOutput::boxed(
                    format!(
                        "Welcome, {}!\n\nUsername: {}\nRole: {}\nAPI Key: {}\nBackend: {}",
                        session.user.display_name,
                        session.user.username,
                        session.user.role,
                        session.key_name,
                        session.backend
                    ),
                    Some("Authentication Success"),
                    "green"
                )
            );
            Ok(())
        }
        Err(e) => {
            Logger::error(format!("Login failed: {}", e));
            Err(e)
        }
    }
}

/// Handle direct provider login (new flow).
async fn handle_direct_provider_login(
    provider_name: String,
    key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    region: Option<String>,
    project_id: Option<String>,
) -> Result<()> {
    let provider_type = ProviderType::from_str_opt(&provider_name)
        .ok_or_else(|| anyhow!(
            "Unknown provider: '{}'. Supported: anthropic, openai, google, groq, ollama, bedrock, vertex-ai",
            provider_name
        ))?;

    if provider_type == ProviderType::Brainwires {
        return Err(anyhow!(
            "Use 'brainwires auth login' (without --provider) for Brainwires SaaS login"
        ));
    }

    let model_name = model.unwrap_or_else(|| provider_type.default_model().to_string());
    let mut config_manager = ConfigManager::new()?;

    // Ollama: no API key, just configure URL
    if provider_type == ProviderType::Ollama {
        let ollama_url = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());

        config_manager.update(ConfigUpdates {
            provider_type: Some(ProviderType::Ollama),
            model: Some(model_name.clone()),
            provider_base_url: Some(Some(ollama_url.clone())),
            ..Default::default()
        });
        config_manager.save()?;

        println!();
        println!(
            "{}",
            RichOutput::boxed(
                format!(
                    "Ollama configured!\n\nModel: {}\nBase URL: {}\n\nNo API key required.",
                    model_name, ollama_url
                ),
                Some("Provider Configured"),
                "green",
            )
        );

        validate_selected_model(ProviderType::Ollama, &model_name, None, Some(&ollama_url)).await;
        return Ok(());
    }

    // Bedrock: uses AWS credential chain, no API key prompt
    if provider_type == ProviderType::Bedrock {
        let aws_region = region
            .clone()
            .or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
            .unwrap_or_else(|| "us-east-1".to_string());

        // Validate AWS creds are available
        if std::env::var("AWS_ACCESS_KEY_ID").is_err() {
            Logger::warn(
                "AWS_ACCESS_KEY_ID not set. Bedrock will fail at runtime without AWS credentials.",
            );
            Logger::warn(
                "Configure via: aws configure, or set AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY env vars.",
            );
        }

        // Store provider options as JSON in the extra config
        let mut provider_options = serde_json::Map::new();
        provider_options.insert("region".to_string(), serde_json::json!(aws_region));

        config_manager.update(ConfigUpdates {
            provider_type: Some(ProviderType::Bedrock),
            model: Some(model_name.clone()),
            provider_base_url: Some(None),
            ..Default::default()
        });
        config_manager.get_mut().extra.insert(
            "provider_options".to_string(),
            serde_json::Value::Object(provider_options),
        );
        config_manager.save()?;

        println!();
        println!(
            "{}",
            RichOutput::boxed(
                format!(
                    "Bedrock configured!\n\nModel: {}\nRegion: {}\n\nUsing AWS credential chain (env vars / ~/.aws/credentials).\nNo API key required.",
                    model_name, aws_region
                ),
                Some("Provider Configured"),
                "green",
            )
        );

        return Ok(());
    }

    // Vertex AI: uses GCP Application Default Credentials, no API key prompt
    if provider_type == ProviderType::VertexAI {
        let gcp_project = project_id.or_else(|| std::env::var("GOOGLE_CLOUD_PROJECT").ok());

        let gcp_project = match gcp_project {
            Some(p) => p,
            None => {
                return Err(anyhow!(
                    "Vertex AI requires a project ID. Provide --project-id or set GOOGLE_CLOUD_PROJECT env var."
                ));
            }
        };

        let gcp_region = region.clone().unwrap_or_else(|| "us-central1".to_string());

        // Validate GCP creds are available
        if std::env::var("GOOGLE_APPLICATION_CREDENTIALS").is_err() {
            // Check if gcloud auth is available as fallback
            Logger::warn(
                "GOOGLE_APPLICATION_CREDENTIALS not set. Vertex AI will use gcloud auth application-default credentials if available.",
            );
        }

        let mut provider_options = serde_json::Map::new();
        provider_options.insert("project_id".to_string(), serde_json::json!(gcp_project));
        provider_options.insert("region".to_string(), serde_json::json!(gcp_region));

        config_manager.update(ConfigUpdates {
            provider_type: Some(ProviderType::VertexAI),
            model: Some(model_name.clone()),
            provider_base_url: Some(None),
            ..Default::default()
        });
        config_manager.get_mut().extra.insert(
            "provider_options".to_string(),
            serde_json::Value::Object(provider_options),
        );
        config_manager.save()?;

        println!();
        println!(
            "{}",
            RichOutput::boxed(
                format!(
                    "Vertex AI configured!\n\nModel: {}\nProject: {}\nRegion: {}\n\nUsing GCP Application Default Credentials.\nNo API key required.",
                    model_name, gcp_project, gcp_region
                ),
                Some("Provider Configured"),
                "green",
            )
        );

        return Ok(());
    }

    // All other providers require an API key
    let api_key = if let Some(k) = key {
        k
    } else {
        Password::new()
            .with_prompt(format!("Enter your {} API key", provider_type.as_str()))
            .interact()?
    };

    // Store in keyring
    config_manager.set_provider_api_key(provider_type, &api_key)?;

    // Update config
    config_manager.update(ConfigUpdates {
        provider_type: Some(provider_type),
        model: Some(model_name.clone()),
        provider_base_url: Some(base_url.clone()),
        ..Default::default()
    });
    config_manager.save()?;

    let mut status = format!(
        "{} configured!\n\nModel: {}\nAPI Key: stored in system keyring",
        provider_type.as_str(),
        model_name
    );
    if let Some(ref url) = base_url {
        status.push_str(&format!("\nBase URL: {}", url));
    }

    println!();
    println!(
        "{}",
        RichOutput::boxed(&status, Some("Provider Configured"), "green",)
    );

    // Validate the selected model (non-blocking — warn but don't fail)
    validate_selected_model(
        provider_type,
        &model_name,
        Some(&api_key),
        base_url.as_deref(),
    )
    .await;

    Ok(())
}

/// Validate that the selected model exists at the provider.
/// Prints a warning if validation fails but never blocks login.
async fn validate_selected_model(
    provider_type: ProviderType,
    model_id: &str,
    api_key: Option<&str>,
    base_url: Option<&str>,
) {
    match ModelService::validate_model(provider_type, model_id, api_key, base_url).await {
        Ok(model) => {
            let caps: Vec<String> = model.capabilities.iter().map(|c| c.to_string()).collect();
            let mut info = format!("Model '{}' validated", model.id);
            if !caps.is_empty() {
                info.push_str(&format!(" ({})", caps.join(", ")));
            }
            if let Some(ctx) = model.context_window {
                info.push_str(&format!(" [context: {}K]", ctx / 1000));
            }
            Logger::success(&info);
        }
        Err(e) => {
            Logger::warn(format!("Could not validate model '{}': {}", model_id, e));
            Logger::warn(
                "The model may still work (preview/beta models may not appear in listings)",
            );
        }
    }
}

/// Show Brainwires SaaS authentication status.
fn show_brainwires_status(verbose: bool) -> Result<()> {
    match SessionManager::get_session()? {
        Some(session) => {
            if session.is_expired() {
                Logger::warn("Session may be expired. Consider re-authenticating.");
            }

            let mut status_text = format!(
                "Provider: Brainwires (SaaS)\nUser: {} (@{})\nRole: {}\nAPI Key: {}\nBackend: {}\nAuthenticated: {}",
                session.user.display_name,
                session.user.username,
                session.user.role,
                session.key_name,
                session.backend,
                session.authenticated_at.format("%Y-%m-%d %H:%M:%S")
            );

            if verbose {
                status_text.push_str("\n\n--- Supabase ---\n");
                status_text.push_str(&format!("URL: {}\n", session.supabase.url));
                status_text.push_str("Anon Key: <hidden for security>");
            }

            println!();
            println!(
                "{}",
                RichOutput::boxed(&status_text, Some("Authentication Status"), "green")
            );
        }
        None => {
            println!();
            println!(
                "{}",
                RichOutput::boxed(
                    "Provider: Brainwires (SaaS)\nStatus: Not authenticated\n\nRun \"brainwires auth login\" to authenticate",
                    Some("Authentication Status"),
                    "yellow"
                )
            );
        }
    }
    Ok(())
}

/// Validate Brainwires SaaS session.
async fn validate_brainwires_session() -> Result<()> {
    if !SessionManager::is_authenticated()? {
        Logger::error("Not authenticated");
        return Ok(());
    }

    Logger::info("Validating session with backend...");

    let config = ConfigManager::new()?;
    let client = AuthClient::new(config.get().backend_url.clone());

    match client.validate_session().await {
        Ok(true) => Logger::success("Session is valid"),
        Ok(false) | Err(_) => {
            Logger::error("Session is invalid or expired. Please login again.");
        }
    }

    Ok(())
}
