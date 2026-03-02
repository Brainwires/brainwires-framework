use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrainingError {
    #[error("Dataset error: {0}")]
    Dataset(#[from] brainwires_datasets::DatasetError),

    #[error("API error: {message} (status: {status_code})")]
    Api {
        message: String,
        status_code: u16,
    },

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Job failed: {0}")]
    JobFailed(String),

    #[error("Upload error: {0}")]
    Upload(String),

    #[error("Training backend error: {0}")]
    Backend(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    #[cfg(feature = "cloud")]
    Http(#[from] reqwest::Error),

    #[error("{0}")]
    Other(String),
}

pub type TrainingResult<T> = Result<T, TrainingError>;
