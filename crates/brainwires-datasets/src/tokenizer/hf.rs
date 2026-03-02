use crate::error::{DatasetError, DatasetResult};
use super::Tokenizer;

/// HuggingFace tokenizers wrapper.
pub struct HfTokenizer {
    tokenizer: tokenizers::Tokenizer,
}

impl HfTokenizer {
    /// Load a tokenizer from a local JSON file.
    pub fn from_file(path: &str) -> DatasetResult<Self> {
        let tokenizer = tokenizers::Tokenizer::from_file(path)
            .map_err(|e| DatasetError::Tokenizer {
                message: format!("Failed to load tokenizer from '{}': {}", path, e),
            })?;
        Ok(Self { tokenizer })
    }

    /// Load a tokenizer from raw JSON bytes.
    pub fn from_bytes(bytes: &[u8]) -> DatasetResult<Self> {
        let tokenizer = tokenizers::Tokenizer::from_bytes(bytes)
            .map_err(|e| DatasetError::Tokenizer {
                message: format!("Failed to load tokenizer from bytes: {}", e),
            })?;
        Ok(Self { tokenizer })
    }
}

impl Tokenizer for HfTokenizer {
    fn encode(&self, text: &str) -> DatasetResult<Vec<u32>> {
        let encoding = self.tokenizer.encode(text, false)
            .map_err(|e| DatasetError::Tokenizer {
                message: format!("Encoding error: {}", e),
            })?;
        Ok(encoding.get_ids().to_vec())
    }

    fn decode(&self, ids: &[u32]) -> DatasetResult<String> {
        self.tokenizer.decode(ids, true)
            .map_err(|e| DatasetError::Tokenizer {
                message: format!("Decoding error: {}", e),
            })
    }

    fn vocab_size(&self) -> usize {
        self.tokenizer.get_vocab_size(true)
    }
}
