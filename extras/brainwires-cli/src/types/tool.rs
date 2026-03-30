//! Tool types for tool execution framework.
//!
//! Re-exports from the brainwires-core framework crate, with CLI-specific extensions.

pub use brainwires::core::tool::*;

/// CLI-specific extension trait for ToolContext
pub trait ToolContextExt {
    /// Create a ToolContext from an AgentContext
    fn from_agent_context(ctx: &super::agent::AgentContext) -> ToolContext;
}

impl ToolContextExt for ToolContext {
    fn from_agent_context(ctx: &super::agent::AgentContext) -> ToolContext {
        ToolContext {
            working_directory: ctx.working_directory.clone(),
            user_id: ctx.user_id.clone(),
            metadata: ctx.metadata.clone(),
            capabilities: serde_json::to_value(&ctx.capabilities).ok(),
            ..Default::default()
        }
    }
}
