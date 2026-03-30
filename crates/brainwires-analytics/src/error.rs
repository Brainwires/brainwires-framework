use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[cfg(feature = "sqlite")]
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Analytics sink channel closed")]
    ChannelClosed,

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type AnalyticsResult<T> = Result<T, AnalyticsError>;
