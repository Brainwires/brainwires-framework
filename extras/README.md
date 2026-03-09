# extras

Standalone utilities and supplementary crates that build on top of the Brainwires Framework but are not part of the core library.

| Directory | Description |
|-----------|-------------|
| [`brainwires-proxy/`](brainwires-proxy/) | Protocol-agnostic proxy framework with pluggable middleware, multiple transports (HTTP, WebSocket, TCP, Unix, SSE), and traffic inspection. |
| [`brainwires-brain-server/`](brainwires-brain-server/) | Standalone server for the Open Brain knowledge system. |
| [`brainwires-rag-server/`](brainwires-rag-server/) | Standalone RAG indexing and semantic search server. |
| [`reload-daemon/`](reload-daemon/) | Minimal MCP server that enables AI coding clients to kill and restart themselves with transformed arguments. |
| [`agent-chat/`](agent-chat/) | Simplified open-source AI chat client with plain and TUI modes, tool execution, and all cloud providers. |
| [`audio-demo-ffi/`](audio-demo-ffi/) | UniFFI bindings (cdylib) exposing brainwires-audio TTS/STT to C#, Kotlin, Swift, and Python. |
| [`audio-demo/`](audio-demo/) | Cross-platform Avalonia (.NET) GUI for demoing TTS and STT across all 9 audio providers. |
