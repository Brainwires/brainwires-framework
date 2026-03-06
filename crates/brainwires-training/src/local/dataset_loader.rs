//! Dataset loading for local training.
//!
//! Parses JSONL training files into tokenized batches for the Burn training loop.
//! Supports instruction-tuning formats: `{"prompt": ..., "completion": ...}` and
//! `{"messages": [...]}` (chat format).

use std::io::BufRead;
use std::path::Path;

use tracing::info;

use crate::error::TrainingError;

/// A single training example (prompt + completion text).
#[derive(Debug, Clone)]
pub struct TrainingExample {
    /// Input text (prompt/instruction).
    pub prompt: String,
    /// Target text (completion/response).
    pub completion: String,
}

/// Parsed dataset ready for batching.
#[derive(Debug)]
pub struct TrainingDataset {
    /// All training examples.
    pub examples: Vec<TrainingExample>,
}

impl TrainingDataset {
    /// Load a JSONL dataset from disk.
    ///
    /// Supports two formats:
    /// 1. `{"prompt": "...", "completion": "..."}`
    /// 2. `{"messages": [{"role": "user", "content": "..."}, {"role": "assistant", "content": "..."}]}`
    pub fn load_jsonl(path: &Path) -> Result<Self, TrainingError> {
        let file = std::fs::File::open(path).map_err(|e| {
            TrainingError::Config(format!("Failed to open dataset: {}: {}", path.display(), e))
        })?;
        let reader = std::io::BufReader::new(file);
        let mut examples = Vec::new();

        for (line_num, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| {
                TrainingError::Config(format!("Failed to read line {}: {}", line_num + 1, e))
            })?;
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            let value: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
                TrainingError::Config(format!(
                    "Invalid JSON on line {}: {}",
                    line_num + 1,
                    e
                ))
            })?;

            let example = if value.get("messages").is_some() {
                parse_chat_format(&value, line_num + 1)?
            } else if value.get("prompt").is_some() {
                parse_prompt_completion(&value, line_num + 1)?
            } else {
                return Err(TrainingError::Config(format!(
                    "Line {}: expected 'prompt'+'completion' or 'messages' field",
                    line_num + 1,
                )));
            };

            examples.push(example);
        }

        if examples.is_empty() {
            return Err(TrainingError::Config(
                "Dataset is empty (no valid examples found)".to_string(),
            ));
        }

        info!("Loaded {} training examples from {:?}", examples.len(), path);
        Ok(Self { examples })
    }

    /// Number of examples in the dataset.
    pub fn len(&self) -> usize {
        self.examples.len()
    }

    /// Whether the dataset is empty.
    pub fn is_empty(&self) -> bool {
        self.examples.is_empty()
    }

    /// Calculate steps per epoch given a batch size.
    pub fn steps_per_epoch(&self, batch_size: usize) -> u64 {
        (self.examples.len() / batch_size.max(1)).max(1) as u64
    }

    /// Get a batch of examples by index range.
    pub fn get_batch(&self, start: usize, batch_size: usize) -> &[TrainingExample] {
        let end = (start + batch_size).min(self.examples.len());
        &self.examples[start..end]
    }
}

/// Parse `{"prompt": "...", "completion": "..."}` format.
fn parse_prompt_completion(
    value: &serde_json::Value,
    line_num: usize,
) -> Result<TrainingExample, TrainingError> {
    let prompt = value
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            TrainingError::Config(format!("Line {}: 'prompt' must be a string", line_num))
        })?
        .to_string();

    let completion = value
        .get("completion")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            TrainingError::Config(format!("Line {}: 'completion' must be a string", line_num))
        })?
        .to_string();

    Ok(TrainingExample { prompt, completion })
}

/// Parse `{"messages": [{"role": "...", "content": "..."}]}` chat format.
fn parse_chat_format(
    value: &serde_json::Value,
    line_num: usize,
) -> Result<TrainingExample, TrainingError> {
    let messages = value
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            TrainingError::Config(format!("Line {}: 'messages' must be an array", line_num))
        })?;

    let mut prompt_parts = Vec::new();
    let mut completion = String::new();

    for msg in messages {
        let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("");
        let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("");

        match role {
            "system" | "user" => prompt_parts.push(content.to_string()),
            "assistant" => completion = content.to_string(),
            _ => {}
        }
    }

    if prompt_parts.is_empty() {
        return Err(TrainingError::Config(format!(
            "Line {}: no user/system messages found",
            line_num
        )));
    }
    if completion.is_empty() {
        return Err(TrainingError::Config(format!(
            "Line {}: no assistant message found",
            line_num
        )));
    }

    Ok(TrainingExample {
        prompt: prompt_parts.join("\n"),
        completion,
    })
}

/// Simple character-level tokenizer for training.
///
/// In production, this would be a BPE/SentencePiece tokenizer loaded from the model.
/// This basic implementation enables the training loop to work end-to-end.
pub struct SimpleTokenizer {
    max_seq_len: usize,
}

impl SimpleTokenizer {
    /// Create a tokenizer with the given maximum sequence length.
    pub fn new(max_seq_len: usize) -> Self {
        Self { max_seq_len }
    }

    /// Tokenize text into u32 token IDs using byte values.
    ///
    /// Each byte maps to a token ID (0-255). This is a placeholder;
    /// real training would use the model's actual tokenizer.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        text.bytes()
            .take(self.max_seq_len)
            .map(|b| b as u32)
            .collect()
    }

    /// Tokenize a training example into (input_ids, target_ids).
    ///
    /// Concatenates prompt + completion, with targets shifted by one position
    /// (standard causal LM training). Prompt tokens are masked in targets (set to u32::MAX).
    pub fn encode_example(&self, example: &TrainingExample) -> (Vec<u32>, Vec<u32>) {
        let prompt_tokens = self.encode(&example.prompt);
        let completion_tokens = self.encode(&example.completion);
        let prompt_len = prompt_tokens.len();

        let mut input_ids = prompt_tokens;
        input_ids.extend_from_slice(&completion_tokens);
        input_ids.truncate(self.max_seq_len);

        // Targets: shifted input_ids, with prompt portion masked
        let mut target_ids = vec![u32::MAX; input_ids.len()];
        target_ids[prompt_len..input_ids.len()]
            .copy_from_slice(&input_ids[prompt_len..]);

        (input_ids, target_ids)
    }

    /// Vocabulary size for this tokenizer (byte-level = 256 + 1 padding token).
    pub fn vocab_size(&self) -> usize {
        257
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_prompt_completion() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("train.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, r#"{{"prompt": "Hello", "completion": "World"}}"#).unwrap();
        writeln!(f, r#"{{"prompt": "Foo", "completion": "Bar"}}"#).unwrap();

        let dataset = TrainingDataset::load_jsonl(&path).unwrap();
        assert_eq!(dataset.len(), 2);
        assert_eq!(dataset.examples[0].prompt, "Hello");
        assert_eq!(dataset.examples[0].completion, "World");
    }

    #[test]
    fn test_load_chat_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("train.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"messages": [{{"role": "user", "content": "Hi"}}, {{"role": "assistant", "content": "Hello!"}}]}}"#
        )
        .unwrap();

        let dataset = TrainingDataset::load_jsonl(&path).unwrap();
        assert_eq!(dataset.len(), 1);
        assert_eq!(dataset.examples[0].prompt, "Hi");
        assert_eq!(dataset.examples[0].completion, "Hello!");
    }

    #[test]
    fn test_empty_dataset_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.jsonl");
        std::fs::File::create(&path).unwrap();

        let result = TrainingDataset::load_jsonl(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_steps_per_epoch() {
        let dataset = TrainingDataset {
            examples: vec![
                TrainingExample {
                    prompt: "a".into(),
                    completion: "b".into(),
                };
                100
            ],
        };
        assert_eq!(dataset.steps_per_epoch(4), 25);
        assert_eq!(dataset.steps_per_epoch(10), 10);
    }

    #[test]
    fn test_simple_tokenizer() {
        let tok = SimpleTokenizer::new(512);
        let tokens = tok.encode("Hello");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], b'H' as u32);
    }

    #[test]
    fn test_encode_example() {
        let tok = SimpleTokenizer::new(512);
        let example = TrainingExample {
            prompt: "Hi".to_string(),
            completion: "Ok".to_string(),
        };
        let (input, target) = tok.encode_example(&example);
        assert_eq!(input.len(), 4); // "Hi" + "Ok"
        // First 2 tokens (prompt) should be masked
        assert_eq!(target[0], u32::MAX);
        assert_eq!(target[1], u32::MAX);
        // Completion tokens should have actual values
        assert_eq!(target[2], b'O' as u32);
        assert_eq!(target[3], b'k' as u32);
    }
}
