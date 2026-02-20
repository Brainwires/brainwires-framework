//! MDAP (Massively Decomposed Agentic Processes) Error Types
//!
//! Provides domain-specific error types for the MDAP framework implementation,
//! based on the MAKER paper's error handling requirements.

use std::collections::HashMap;
use thiserror::Error;

/// Main error type for the MDAP system
#[derive(Error, Debug)]
pub enum MdapError {
    #[error("Voting error: {0}")]
    Voting(#[from] VotingError),

    #[error("Red-flag validation error: {0}")]
    RedFlag(#[from] RedFlagError),

    #[error("Decomposition error: {0}")]
    Decomposition(#[from] DecompositionError),

    #[error("Microagent error: {0}")]
    Microagent(#[from] MicroagentError),

    #[error("Composition error: {0}")]
    Composition(#[from] CompositionError),

    #[error("Scaling error: {0}")]
    Scaling(#[from] ScalingError),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Configuration error: {0}")]
    Config(#[from] MdapConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Semaphore acquire error: {0}")]
    Semaphore(String),

    #[error("Task join error: {0}")]
    TaskJoin(String),

    #[error("{0}")]
    Other(String),

    // Tool-related errors for microagent tool execution
    #[error("Tool recursion limit reached: depth {depth} >= max {max_depth}")]
    ToolRecursionLimit { depth: u32, max_depth: u32 },

    #[error("Tool execution failed: {tool} - {reason}")]
    ToolExecutionFailed { tool: String, reason: String },

    #[error("Tool not allowed for microagent: {tool} (category: {category})")]
    ToolNotAllowed { tool: String, category: String },

    #[error("Tool intent parsing failed: {0}")]
    ToolIntentParseFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Errors related to the first-to-ahead-by-k voting system (Algorithm 2)
#[derive(Error, Debug)]
pub enum VotingError {
    #[error("Maximum samples exceeded: {samples} samples taken, no consensus reached")]
    MaxSamplesExceeded {
        samples: u32,
        votes: HashMap<String, u32>,
    },

    #[error("All samples were red-flagged: {red_flagged}/{total} samples invalid")]
    AllSamplesRedFlagged { red_flagged: u32, total: u32 },

    #[error("Voting cancelled")]
    Cancelled,

    #[error("No valid responses received after {attempts} attempts")]
    NoValidResponses { attempts: u32 },

    #[error("Sampler returned error: {0}")]
    SamplerError(String),

    #[error("Vote comparison failed: unable to hash response")]
    HashError,

    #[error("Invalid k value: k must be >= 1, got {0}")]
    InvalidK(u32),

    #[error("Parallel execution error: {0}")]
    ParallelError(String),
}

/// Errors related to red-flag validation (Algorithm 3)
#[derive(Error, Debug)]
pub enum RedFlagError {
    #[error("Response too long: {tokens} tokens exceeds limit of {limit}")]
    ResponseTooLong { tokens: u32, limit: u32 },

    #[error("Invalid format: expected {expected}, got {got}")]
    InvalidFormat { expected: String, got: String },

    #[error("Self-correction detected: '{pattern}' indicates model confusion")]
    SelfCorrectionDetected { pattern: String },

    #[error("Confused reasoning detected: '{pattern}'")]
    ConfusedReasoning { pattern: String },

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Empty response")]
    EmptyResponse,

    #[error("Invalid JSON structure: {0}")]
    InvalidJson(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Validation pattern error: {0}")]
    PatternError(String),
}

/// Errors related to task decomposition
#[derive(Error, Debug)]
pub enum DecompositionError {
    #[error("Maximum decomposition depth exceeded: {depth} > {max_depth}")]
    MaxDepthExceeded { depth: u32, max_depth: u32 },

    #[error("Task cannot be decomposed further: {0}")]
    CannotDecompose(String),

    #[error("Circular dependency detected in subtasks: {0}")]
    CircularDependency(String),

    #[error("Invalid subtask dependency: '{subtask}' depends on non-existent '{dependency}'")]
    InvalidDependency { subtask: String, dependency: String },

    #[error("Decomposition voting failed: {0}")]
    VotingFailed(String),

    #[error("Empty decomposition result for task: {0}")]
    EmptyResult(String),

    #[error("Invalid decomposition strategy: {0}")]
    InvalidStrategy(String),

    #[error("Discriminator error: {0}")]
    DiscriminatorError(String),
}

/// Errors related to microagent execution
#[derive(Error, Debug)]
pub enum MicroagentError {
    #[error("Subtask execution failed: {subtask_id} - {reason}")]
    ExecutionFailed { subtask_id: String, reason: String },

    #[error("Subtask timeout after {timeout_ms}ms: {subtask_id}")]
    Timeout { subtask_id: String, timeout_ms: u64 },

    #[error("Invalid input state for subtask '{subtask_id}': {reason}")]
    InvalidInput { subtask_id: String, reason: String },

    #[error("Output parsing failed for subtask '{subtask_id}': {reason}")]
    OutputParseFailed { subtask_id: String, reason: String },

    #[error("Provider communication error: {0}")]
    ProviderError(String),

    #[error("Context too large for microagent: {size} tokens > {limit} limit")]
    ContextTooLarge { size: u32, limit: u32 },

    #[error("Missing dependency result: subtask '{subtask_id}' requires '{dependency}'")]
    MissingDependency {
        subtask_id: String,
        dependency: String,
    },
}

/// Errors related to result composition
#[derive(Error, Debug)]
pub enum CompositionError {
    #[error("Missing subtask result: {0}")]
    MissingResult(String),

    #[error("Incompatible result types: cannot compose {type_a} with {type_b}")]
    IncompatibleTypes { type_a: String, type_b: String },

    #[error("Composition function '{function}' not found")]
    FunctionNotFound { function: String },

    #[error("Composition execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Invalid composition order: {0}")]
    InvalidOrder(String),

    #[error("Result validation failed: {0}")]
    ValidationFailed(String),
}

/// Errors related to scaling law calculations
#[derive(Error, Debug)]
pub enum ScalingError {
    #[error("Invalid success probability: {0} must be in range (0.5, 1.0)")]
    InvalidSuccessProbability(f64),

    #[error("Invalid target probability: {0} must be in range (0.0, 1.0)")]
    InvalidTargetProbability(f64),

    #[error("Invalid step count: must be > 0, got {0}")]
    InvalidStepCount(u64),

    #[error("Voting cannot converge: per-step success rate {p} <= 0.5")]
    VotingCannotConverge { p: f64 },

    #[error("Cost estimation failed: {0}")]
    CostEstimationFailed(String),

    #[error("Numerical overflow in calculation: {0}")]
    NumericalOverflow(String),
}

/// Errors related to MDAP configuration
#[derive(Error, Debug)]
pub enum MdapConfigError {
    #[error("Invalid k value: must be >= 1, got {0}")]
    InvalidK(u32),

    #[error("Invalid target success rate: must be in (0.0, 1.0), got {0}")]
    InvalidTargetSuccessRate(f64),

    #[error("Invalid parallel samples: must be 1-4, got {0}")]
    InvalidParallelSamples(u32),

    #[error("Invalid max samples per subtask: must be > 0, got {0}")]
    InvalidMaxSamples(u32),

    #[error("Invalid max response tokens: must be > 0, got {0}")]
    InvalidMaxTokens(u32),

    #[error("Invalid decomposition max depth: must be > 0, got {0}")]
    InvalidMaxDepth(u32),

    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    #[error("Configuration parse error: {0}")]
    ParseError(String),
}

// Conversion from anyhow::Error to MdapError
impl From<anyhow::Error> for MdapError {
    fn from(err: anyhow::Error) -> Self {
        MdapError::Other(format!("{:#}", err))
    }
}

// Conversion from tokio semaphore acquire error
impl From<tokio::sync::AcquireError> for MdapError {
    fn from(err: tokio::sync::AcquireError) -> Self {
        MdapError::Semaphore(err.to_string())
    }
}

// Conversion from tokio join error
impl From<tokio::task::JoinError> for MdapError {
    fn from(err: tokio::task::JoinError) -> Self {
        MdapError::TaskJoin(err.to_string())
    }
}

// Helper methods for MdapError
impl MdapError {
    /// Create a new error from a string message
    pub fn other(msg: impl Into<String>) -> Self {
        MdapError::Other(msg.into())
    }

    /// Create a provider error
    pub fn provider(msg: impl Into<String>) -> Self {
        MdapError::Provider(msg.into())
    }

    /// Convert to a user-facing error string
    pub fn to_user_string(&self) -> String {
        format!("{}", self)
    }

    /// Check if this is a user/configuration error vs system/runtime error
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            MdapError::Config(_)
                | MdapError::Scaling(ScalingError::InvalidSuccessProbability(_))
                | MdapError::Scaling(ScalingError::InvalidTargetProbability(_))
        )
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            MdapError::Provider(_)
                | MdapError::Semaphore(_)
                | MdapError::Voting(VotingError::SamplerError(_))
                | MdapError::Microagent(MicroagentError::ProviderError(_))
                | MdapError::ToolExecutionFailed { .. }
        )
    }

    /// Check if this is a tool-related error
    pub fn is_tool_error(&self) -> bool {
        matches!(
            self,
            MdapError::ToolRecursionLimit { .. }
                | MdapError::ToolExecutionFailed { .. }
                | MdapError::ToolNotAllowed { .. }
                | MdapError::ToolIntentParseFailed(_)
        )
    }

    /// Check if this error indicates the voting process should be restarted
    pub fn should_restart_voting(&self) -> bool {
        matches!(
            self,
            MdapError::RedFlag(RedFlagError::ResponseTooLong { .. })
                | MdapError::RedFlag(RedFlagError::SelfCorrectionDetected { .. })
                | MdapError::RedFlag(RedFlagError::ConfusedReasoning { .. })
        )
    }

    /// Check if this error is a red-flag (should discard and resample)
    pub fn is_red_flag(&self) -> bool {
        matches!(self, MdapError::RedFlag(_))
    }
}

/// Result type alias for MDAP operations
pub type MdapResult<T> = Result<T, MdapError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voting_error_display() {
        let mut votes = HashMap::new();
        votes.insert("option_a".to_string(), 3);
        votes.insert("option_b".to_string(), 2);

        let err = VotingError::MaxSamplesExceeded { samples: 50, votes };
        assert!(err
            .to_string()
            .contains("Maximum samples exceeded: 50 samples taken"));
    }

    #[test]
    fn test_red_flag_error_display() {
        let err = RedFlagError::ResponseTooLong {
            tokens: 800,
            limit: 750,
        };
        assert_eq!(
            err.to_string(),
            "Response too long: 800 tokens exceeds limit of 750"
        );
    }

    #[test]
    fn test_self_correction_error() {
        let err = RedFlagError::SelfCorrectionDetected {
            pattern: "Wait,".to_string(),
        };
        assert!(err.to_string().contains("Wait,"));
        assert!(err.to_string().contains("model confusion"));
    }

    #[test]
    fn test_decomposition_error() {
        let err = DecompositionError::MaxDepthExceeded {
            depth: 15,
            max_depth: 10,
        };
        assert_eq!(
            err.to_string(),
            "Maximum decomposition depth exceeded: 15 > 10"
        );
    }

    #[test]
    fn test_microagent_error() {
        let err = MicroagentError::Timeout {
            subtask_id: "task_001".to_string(),
            timeout_ms: 5000,
        };
        assert!(err.to_string().contains("task_001"));
        assert!(err.to_string().contains("5000ms"));
    }

    #[test]
    fn test_scaling_error() {
        let err = ScalingError::VotingCannotConverge { p: 0.45 };
        assert!(err.to_string().contains("0.45"));
        assert!(err.to_string().contains("<= 0.5"));
    }

    #[test]
    fn test_config_error() {
        let err = MdapConfigError::InvalidParallelSamples(8);
        assert_eq!(err.to_string(), "Invalid parallel samples: must be 1-4, got 8");
    }

    #[test]
    fn test_mdap_error_from_voting() {
        let voting_err = VotingError::Cancelled;
        let mdap_err: MdapError = voting_err.into();
        assert!(matches!(mdap_err, MdapError::Voting(_)));
    }

    #[test]
    fn test_mdap_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("test error");
        let mdap_err: MdapError = anyhow_err.into();
        assert!(matches!(mdap_err, MdapError::Other(_)));
    }

    #[test]
    fn test_is_user_error() {
        let user_err = MdapError::Config(MdapConfigError::InvalidK(0));
        assert!(user_err.is_user_error());

        let system_err = MdapError::Provider("connection failed".to_string());
        assert!(!system_err.is_user_error());
    }

    #[test]
    fn test_is_retryable() {
        let retryable = MdapError::Provider("timeout".to_string());
        assert!(retryable.is_retryable());

        let not_retryable = MdapError::Config(MdapConfigError::InvalidK(0));
        assert!(!not_retryable.is_retryable());
    }

    #[test]
    fn test_is_red_flag() {
        let red_flag = MdapError::RedFlag(RedFlagError::EmptyResponse);
        assert!(red_flag.is_red_flag());

        let not_red_flag = MdapError::Voting(VotingError::Cancelled);
        assert!(!not_red_flag.is_red_flag());
    }

    #[test]
    fn test_should_restart_voting() {
        let should_restart = MdapError::RedFlag(RedFlagError::SelfCorrectionDetected {
            pattern: "Actually,".to_string(),
        });
        assert!(should_restart.should_restart_voting());

        let should_not_restart = MdapError::Voting(VotingError::MaxSamplesExceeded {
            samples: 50,
            votes: HashMap::new(),
        });
        assert!(!should_not_restart.should_restart_voting());
    }

    #[test]
    fn test_error_chain() {
        let red_flag_err = RedFlagError::InvalidFormat {
            expected: "JSON".to_string(),
            got: "plain text".to_string(),
        };
        let mdap_err: MdapError = red_flag_err.into();
        assert!(matches!(mdap_err, MdapError::RedFlag(_)));
        assert!(mdap_err.to_string().contains("Invalid format"));
    }

    #[test]
    fn test_composition_error() {
        let err = CompositionError::IncompatibleTypes {
            type_a: "String".to_string(),
            type_b: "Number".to_string(),
        };
        assert!(err.to_string().contains("String"));
        assert!(err.to_string().contains("Number"));
    }

    #[test]
    fn test_circular_dependency() {
        let err = DecompositionError::CircularDependency("A -> B -> C -> A".to_string());
        assert!(err.to_string().contains("Circular dependency"));
    }

    #[test]
    fn test_invalid_dependency() {
        let err = DecompositionError::InvalidDependency {
            subtask: "task_b".to_string(),
            dependency: "task_unknown".to_string(),
        };
        assert!(err.to_string().contains("task_b"));
        assert!(err.to_string().contains("task_unknown"));
    }
}
