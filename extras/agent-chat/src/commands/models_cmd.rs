use anyhow::Result;

use crate::cli::ModelsArgs;

struct ModelInfo {
    provider: &'static str,
    models: &'static [&'static str],
}

const KNOWN_MODELS: &[ModelInfo] = &[
    ModelInfo {
        provider: "anthropic",
        models: &[
            "claude-opus-4-20250514",
            "claude-sonnet-4-20250514",
            "claude-haiku-4-20250414",
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
        ],
    },
    ModelInfo {
        provider: "openai",
        models: &[
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-4-turbo",
            "o3-mini",
            "o1",
        ],
    },
    ModelInfo {
        provider: "google",
        models: &[
            "gemini-2.5-pro",
            "gemini-2.5-flash",
            "gemini-2.0-flash",
        ],
    },
    ModelInfo {
        provider: "groq",
        models: &[
            "llama-3.3-70b-versatile",
            "llama-3.1-8b-instant",
            "mixtral-8x7b-32768",
        ],
    },
    ModelInfo {
        provider: "ollama",
        models: &["llama3.1", "codellama", "mistral", "phi3"],
    },
    ModelInfo {
        provider: "together",
        models: &[
            "meta-llama/Llama-3.3-70B-Instruct-Turbo",
            "mistralai/Mixtral-8x7B-Instruct-v0.1",
        ],
    },
    ModelInfo {
        provider: "fireworks",
        models: &[
            "accounts/fireworks/models/llama-v3p3-70b-instruct",
        ],
    },
];

pub async fn run(args: ModelsArgs) -> Result<()> {
    let filter = args.provider.as_deref().map(|s| s.to_lowercase());

    for info in KNOWN_MODELS {
        if let Some(ref f) = filter && info.provider != f.as_str() {
            continue;
        }
        println!("{}:", info.provider);
        for model in info.models {
            println!("  {model}");
        }
        println!();
    }

    if let Some(ref f) = filter && !KNOWN_MODELS.iter().any(|m| m.provider == f.as_str()) {
        eprintln!("Unknown provider: {f}");
    }

    Ok(())
}
