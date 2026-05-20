# @brainwires/tool-builtins

Built-in tool implementations: BashTool, FileOpsTool, GitTool, WebTool,
SearchTool, SemanticSearchTool, CalendarTool, SessionsTool.

Extracted from `@brainwires/tools` in v0.11.0 to mirror Rust's
`brainwires-tool-builtins` crate. The execution framework (`ToolRegistry`,
`ToolExecutor`, sanitization, smart router, transaction manager) lives in
`@brainwires/tool-runtime`.

Native-only tools (`code_exec`/`interpreters`, `sandbox_executor`, `browser`,
`email`, `system`) are intentionally Rust-only — see `SKIPPED.md`.
