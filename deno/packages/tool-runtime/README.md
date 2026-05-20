# @brainwires/tool-runtime

Tool execution framework: registry, executor trait, error taxonomy,
sanitization, smart routing, transaction manager, OpenAPI / OAuth / validation /
tool-search / tool-embedding helpers.

Extracted from `@brainwires/tools` in v0.11.0 to mirror Rust's
`brainwires-tool-runtime` crate.

Built-in tool implementations (BashTool, FileOpsTool, GitTool, WebTool,
SearchTool, SemanticSearchTool, CalendarTool, SessionsTool) live in the sibling
package `@brainwires/tool-builtins`.
