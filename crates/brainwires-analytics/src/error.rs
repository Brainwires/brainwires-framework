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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_closed_display() {
        let e = AnalyticsError::ChannelClosed;
        assert!(e.to_string().contains("closed"));
    }

    #[test]
    fn io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let analytics_err: AnalyticsError = io_err.into();
        assert!(analytics_err.to_string().contains("I/O"));
    }

    #[test]
    fn serde_error_from() {
        let serde_err: Result<serde_json::Value, _> = serde_json::from_str("{bad}");
        let analytics_err: AnalyticsError = serde_err.unwrap_err().into();
        assert!(analytics_err.to_string().contains("Serialization"));
    }

    #[test]
    fn other_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("custom failure");
        let analytics_err: AnalyticsError = anyhow_err.into();
        assert!(analytics_err.to_string().contains("custom failure"));
    }

    #[test]
    fn analytics_result_ok() {
        let result: AnalyticsResult<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn analytics_result_err() {
        let result: AnalyticsResult<i32> = Err(AnalyticsError::ChannelClosed);
        assert!(result.is_err());
    }
}
