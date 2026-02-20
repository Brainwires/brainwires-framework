//! Brainwires MDAP - MAKER voting framework
//!
//! Multi-Dimensional Adaptive Planning system implementing the MAKER paper's
//! approach to reliable agent execution through:
//!
//! - **Voting**: First-to-ahead-by-k consensus algorithm for error correction
//! - **Microagents**: Minimal context single-step agents (m=1 decomposition)
//! - **Decomposition**: Task decomposition strategies (binary recursive, sequential)
//! - **Red Flags**: Output validation and format checking
//! - **Scaling**: Cost/probability estimation and optimization
//! - **Metrics**: Execution metrics collection and reporting
//! - **Composer**: Result composition from subtask outputs
//! - **Tool Intent**: Structured tool calling intent for stateless execution

// Re-export core types
pub use brainwires_core;

pub mod error;
pub mod voting;
pub mod microagent;
pub mod red_flags;
pub mod scaling;
pub mod metrics;
pub mod composer;
pub mod tool_intent;
pub mod decomposition;

// Re-exports
pub use error::{MdapError, MdapResult};
pub use voting::{FirstToAheadByKVoter, VoteResult, SampledResponse, ResponseMetadata};
pub use microagent::{
    Microagent, MicroagentConfig, MicroagentConfigBuilder,
    MicroagentProvider, MicroagentResponse,
    Subtask, SubtaskOutput,
};
pub use red_flags::{RedFlagConfig, StandardRedFlagValidator, OutputFormat};
pub use scaling::{MdapEstimate, ModelCosts, estimate_mdap};
pub use metrics::MdapMetrics;
pub use composer::{Composer, StandardComposer, CompositionBuilder};
pub use tool_intent::{ToolIntent, ToolSchema, ToolCategory, SubtaskOutputWithIntent};
pub use decomposition::{
    TaskDecomposer, DecomposeContext, DecompositionResult, DecompositionStrategy,
    CompositionFunction, SequentialDecomposer, AtomicDecomposer,
    BinaryRecursiveDecomposer, SimpleRecursiveDecomposer,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::voting::{FirstToAheadByKVoter, VoteResult, SampledResponse};
    pub use super::microagent::{
        Microagent, MicroagentProvider, MicroagentResponse,
        Subtask, SubtaskOutput,
    };
    pub use super::red_flags::{RedFlagConfig, OutputFormat};
    pub use super::decomposition::{TaskDecomposer, DecomposeContext, DecompositionResult};
    pub use super::tool_intent::{ToolIntent, ToolSchema, ToolCategory};
    pub use super::error::{MdapError, MdapResult};
}
