// Tools module - built-in tool implementations
//
// Re-exports from the brainwires-tools framework crate, plus CLI-specific tools.
#![allow(hidden_glob_reexports)]

pub use brainwires::tools::*;

// CLI-specific tool modules
mod agent_pool;
mod context_recall;
mod executor;
mod mcp_tool;
mod monitor;
mod plan;
mod session_task;
pub mod smart_router;
mod task_manager;

// Re-export wrappers preserving module paths used elsewhere in CLI
pub mod error;
pub mod validation_tools;

pub use agent_pool::*;
pub use context_recall::*;
pub use mcp_tool::*;
pub use monitor::MonitorTool;
pub use plan::*;
pub use session_task::*;
pub use smart_router::{analyze_messages, get_smart_tools, get_smart_tools_with_mcp};
pub use task_manager::*;
pub use validation_tools::*;

// Explicitly re-export the CLI's concrete ToolExecutor struct so it shadows
// the brainwires_tools::ToolExecutor trait that enters via the glob above.
pub use executor::ToolExecutor;
