//! First-run provider picker.
//!
//! Shown the first time a user runs an interactive CLI command with no
//! config file on disk and no provider detected in the environment. Persists
//! the selected provider to `~/.brainwires/config.json` and returns the
//! chosen `ProviderType` so the caller can proceed with the session.
//!
//! Non-TTY invocations skip the picker and return an error instructing the
//! user how to configure a provider from the CLI or environment. We never
//! prompt when stdin is piped — that would hang CI jobs.

use anyhow::{Result, anyhow};
use console::style;
use dialoguer::{Select, theme::ColorfulTheme};
use std::io::IsTerminal;

use crate::auth::SessionManager;
use crate::config::{ConfigManager, ConfigUpdates};
use crate::providers::ProviderType;
use crate::types::provider_ext::{CHAT_PROVIDERS, credential_hint, detect_provider_from_env, summary};

/// Check whether we should offer a first-run picker.
///
/// Returns `true` only when:
/// - `ConfigManager::is_first_run()` (no config file exists yet), AND
/// - `detect_provider_from_env()` found no credentials in the environment, AND
/// - stdin AND stdout are both TTYs (interactive user, not CI).
pub fn should_prompt(config: &ConfigManager) -> bool {
    if !config.is_first_run() {
        return false;
    }
    if detect_provider_from_env().is_some() {
        return false;
    }
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// Print a non-TTY-friendly "please configure a provider" error.
///
/// Used when `should_prompt` returns false on a first run because stdin
/// isn't a TTY — e.g. in CI or when output is piped. Lists the available
/// providers and the three ways to configure one.
pub fn print_unconfigured_help() {
    eprintln!();
    eprintln!(
        "{} No provider is configured.",
        style("error:").red().bold()
    );
    eprintln!();
    eprintln!("Choose one of the following to get started:");
    eprintln!();
    eprintln!("  {} Set an API key in your environment:", style("·").cyan());
    eprintln!("      export ANTHROPIC_API_KEY=…   # Claude");
    eprintln!("      export OPENAI_API_KEY=…      # GPT");
    eprintln!("      export GEMINI_API_KEY=…      # Gemini");
    eprintln!("      export GROQ_API_KEY=…        # Groq");
    eprintln!("      # …or run Ollama locally (OLLAMA_HOST)");
    eprintln!();
    eprintln!("  {} Log in explicitly:", style("·").cyan());
    eprintln!("      brainwires auth login                     # Brainwires SaaS");
    eprintln!("      brainwires auth login --provider anthropic");
    eprintln!();
    eprintln!("  {} Pass a provider per-invocation:", style("·").cyan());
    eprintln!("      brainwires chat --provider anthropic");
    eprintln!();
}

/// Show the first-run picker interactively.
///
/// Persists the choice to config (provider + default model for that provider).
/// Does NOT prompt for an API key — that's the job of `brainwires auth login`.
/// Returns the selected provider so the caller can decide whether to keep
/// going (e.g. env-var fallback works) or redirect the user to the auth flow.
pub async fn prompt_and_save(config: &mut ConfigManager) -> Result<ProviderType> {
    println!();
    println!("{}", style("━".repeat(70)).cyan());
    println!();
    println!(
        "  {} {}",
        style("✱").bold(),
        style("Welcome to Brainwires CLI").bold().cyan()
    );
    println!();
    println!(
        "  {}",
        style("Pick an AI provider to use. You can change this any time with").dim()
    );
    println!(
        "  {}",
        style("/provider in chat, or `brainwires config set provider <name>`.").dim()
    );
    println!();
    println!("{}", style("━".repeat(70)).cyan());
    println!();

    let items: Vec<String> = CHAT_PROVIDERS
        .iter()
        .map(|p| format!("{:<14}  {}", p.as_str(), style(summary(*p)).dim()))
        .collect();

    // Default selection: Brainwires if the user already has a saved SaaS
    // session (they previously ran `auth login`), otherwise Anthropic as
    // the most common coding target.
    let prefer_brainwires = SessionManager::is_authenticated().unwrap_or(false);
    let preferred = if prefer_brainwires {
        ProviderType::Brainwires
    } else {
        ProviderType::Anthropic
    };
    let default_idx = CHAT_PROVIDERS
        .iter()
        .position(|p| *p == preferred)
        .unwrap_or(0);

    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select provider")
        .items(&items)
        .default(default_idx)
        .interact()?;

    let chosen = CHAT_PROVIDERS[choice];

    config.update(ConfigUpdates {
        provider_type: Some(chosen),
        model: Some(chosen.default_model().to_string()),
        ..Default::default()
    });
    config.save()?;

    println!();
    println!(
        "  {} {} saved as default provider.",
        style("✓").green().bold(),
        style(chosen.as_str()).cyan().bold()
    );
    println!("  {}", style("Default model: ").dim());
    println!(
        "    {}",
        style(chosen.default_model()).cyan()
    );

    // If the provider needs an API key that isn't yet in keyring or env,
    // print a helpful next-step hint (non-blocking — they can also just
    // rely on env vars being set later).
    let needs_login = matches!(
        chosen,
        ProviderType::Brainwires
            | ProviderType::Anthropic
            | ProviderType::OpenAI
            | ProviderType::OpenAiResponses
            | ProviderType::Google
            | ProviderType::Groq
            | ProviderType::Together
            | ProviderType::Fireworks
            | ProviderType::Anyscale
            | ProviderType::MiniMax
    );

    if needs_login {
        let has_env = crate::types::provider_ext::env_var_name(chosen)
            .and_then(|v| std::env::var(v).ok())
            .filter(|v| !v.is_empty())
            .is_some();
        let has_key = config
            .get_provider_api_key_for(chosen)
            .ok()
            .flatten()
            .is_some();

        if !has_env && !has_key {
            println!();
            println!(
                "  {} No API key found. Next step:",
                style("›").yellow().bold()
            );
            println!("    {}", credential_hint(chosen));
        }
    }

    println!();

    Ok(chosen)
}

/// Convenience wrapper: if it's a first run, prompt; otherwise no-op.
///
/// Returns the selected provider (or the existing config value if not
/// first run). Returns an error on a non-TTY first run — the caller
/// should handle that by exiting gracefully.
pub async fn maybe_prompt(config: &mut ConfigManager) -> Result<Option<ProviderType>> {
    if !config.is_first_run() {
        return Ok(None);
    }

    // Env-var users skip the picker and land directly on their provider.
    if let Some((detected, var)) = detect_provider_from_env() {
        tracing::info!(
            "First run: detected {} from {} — skipping picker",
            detected.as_str(),
            var
        );
        config.update(ConfigUpdates {
            provider_type: Some(detected),
            model: Some(detected.default_model().to_string()),
            ..Default::default()
        });
        config.save()?;
        return Ok(Some(detected));
    }

    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        print_unconfigured_help();
        return Err(anyhow!(
            "no provider configured and no TTY available for interactive picker"
        ));
    }

    let chosen = prompt_and_save(config).await?;
    Ok(Some(chosen))
}
