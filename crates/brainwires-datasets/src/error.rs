use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatasetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Format conversion error: {message}")]
    FormatConversion { message: String },

    #[error("Tokenizer error: {message}")]
    Tokenizer { message: String },

    #[error("Index out of bounds: {index} (len: {len})")]
    IndexOutOfBounds { index: usize, len: usize },

    #[error("Empty dataset")]
    EmptyDataset,

    #[error("{0}")]
    Other(String),
}

pub type DatasetResult<T> = Result<T, DatasetError>;
