use brainwires_model_tools::{
    BashTool, FileOpsTool, GitTool, SearchTool, ToolRegistry, ValidationTool, WebTool,
};

pub fn build_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register_tools(BashTool::get_tools());
    registry.register_tools(FileOpsTool::get_tools());
    registry.register_tools(GitTool::get_tools());
    registry.register_tools(SearchTool::get_tools());
    registry.register_tools(ValidationTool::get_tools());
    registry.register_tools(WebTool::get_tools());
    registry
}
