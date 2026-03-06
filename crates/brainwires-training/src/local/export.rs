use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Export format for trained models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// GGUF format (for llama.cpp / Ollama inference).
    Gguf,
    /// SafeTensors format (HuggingFace compatible).
    SafeTensors,
    /// Adapter-only weights (LoRA/QLoRA/DoRA).
    AdapterOnly,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gguf => write!(f, "gguf"),
            Self::SafeTensors => write!(f, "safetensors"),
            Self::AdapterOnly => write!(f, "adapter_only"),
        }
    }
}

/// Configuration for model export.
#[derive(Debug, Clone)]
pub struct ExportConfig {
    /// Output format.
    pub format: ExportFormat,
    /// Output file path.
    pub output_path: PathBuf,
    /// Quantization for GGUF export (e.g., "Q4_K_M", "Q5_K_S").
    pub gguf_quantization: Option<String>,
    /// Include model metadata.
    pub include_metadata: bool,
}

impl ExportConfig {
    /// Create a GGUF export configuration with default Q4_K_M quantization.
    pub fn gguf(output_path: impl Into<PathBuf>) -> Self {
        Self {
            format: ExportFormat::Gguf,
            output_path: output_path.into(),
            gguf_quantization: Some("Q4_K_M".to_string()),
            include_metadata: true,
        }
    }

    /// Create a SafeTensors export configuration.
    pub fn safetensors(output_path: impl Into<PathBuf>) -> Self {
        Self {
            format: ExportFormat::SafeTensors,
            output_path: output_path.into(),
            gguf_quantization: None,
            include_metadata: true,
        }
    }

    /// Create an adapter-only export configuration.
    pub fn adapter_only(output_path: impl Into<PathBuf>) -> Self {
        Self {
            format: ExportFormat::AdapterOnly,
            output_path: output_path.into(),
            gguf_quantization: None,
            include_metadata: true,
        }
    }
}

/// Export metadata written alongside the model.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportMetadata {
    /// Export format name (e.g., "gguf", "safetensors", "adapter_only").
    pub format: String,
    /// Path or identifier of the base model used for training.
    pub base_model: String,
    /// Adapter method used (e.g., "LoRA", "QLoRA", "DoRA"), if applicable.
    pub adapter_method: Option<String>,
    /// Number of training epochs completed.
    pub training_epochs: u32,
    /// Final training loss at export time.
    pub final_loss: Option<f64>,
    /// Timestamp when the model was exported.
    pub exported_at: chrono::DateTime<chrono::Utc>,
}

/// Write export metadata to a JSON file next to the model.
pub fn write_export_metadata(
    output_dir: &Path,
    metadata: &ExportMetadata,
) -> std::io::Result<()> {
    let meta_path = output_dir.join("export_metadata.json");
    let json = serde_json::to_string_pretty(metadata)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(&meta_path, json)?;
    info!("Export metadata written to {:?}", meta_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_display() {
        assert_eq!(ExportFormat::Gguf.to_string(), "gguf");
        assert_eq!(ExportFormat::SafeTensors.to_string(), "safetensors");
        assert_eq!(ExportFormat::AdapterOnly.to_string(), "adapter_only");
    }

    #[test]
    fn test_export_config_builders() {
        let gguf = ExportConfig::gguf("/tmp/model.gguf");
        assert_eq!(gguf.format, ExportFormat::Gguf);
        assert!(gguf.gguf_quantization.is_some());

        let st = ExportConfig::safetensors("/tmp/model.safetensors");
        assert_eq!(st.format, ExportFormat::SafeTensors);
        assert!(st.gguf_quantization.is_none());
    }
}
