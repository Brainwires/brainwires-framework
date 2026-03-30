# MCP Migration Strategy: Custom Implementation → rmcp Crate

## Executive Summary

This document outlines the strategy for migrating brainwires-cli from its custom Model Context Protocol (MCP) implementation (~1,000 lines) to the official `rmcp` crate (v0.8).

**Status**: rmcp is already listed in Cargo.toml but completely unused.

**Goal**: Replace custom MCP code with standards-compliant library implementation.

**Benefits**:
- Standards compliance with official MCP specification
- Automatic protocol updates and bug fixes
- Reduced maintenance burden
- Better community support and ecosystem integration
- Type safety improvements

---

## Current Implementation Analysis

### Custom Code Structure

```
src/mcp/
├── types.rs          (423 lines) - All JSON-RPC & MCP protocol types
├── client.rs         (367 lines) - Complete MCP client implementation
├── transport.rs      (150 lines) - Stdio transport layer
├── config.rs         (191 lines) - Server configuration (keep)
├── tool_adapter.rs   (213 lines) - Tool bridging (adapt)
└── mod.rs            - Module exports
```

### Custom Implementation Details

**types.rs** defines:
- JSON-RPC 2.0: `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcNotification`
- MCP Protocol: `ServerCapabilities`, `ClientCapabilities`, `InitializeParams`, `InitializeResult`
- MCP Entities: `McpTool`, `McpResource`, `McpPrompt`
- Operations: `CallToolParams`, `CallToolResult`, `ReadResourceParams`, etc.
- Content types: `ToolResultContent`, `ResourceContent`, `PromptContent`

**client.rs** implements:
- `McpClient` struct managing multiple server connections
- Request ID generation (atomic counter)
- Connection lifecycle (connect, disconnect, is_connected)
- Protocol handshake (initialize)
- Tool operations (list_tools, call_tool)
- Resource operations (list_resources, read_resource)
- Prompt operations (list_prompts, get_prompt)
- Low-level JSON-RPC request/response handling

**transport.rs** implements:
- `StdioTransport` for spawning and communicating with child processes
- Async read/write over stdin/stdout using Tokio
- Message framing and parsing

---

## rmcp Crate API Overview

### Key Modules

1. **`rmcp::model`** - MCP protocol data types
2. **`rmcp::service`** - Peer-to-peer communication abstraction
3. **`rmcp::transport`** - Transport mechanisms (stdio, HTTP, etc.)
4. **`rmcp::handler`** - Client/server handler traits

### Core Types

```rust
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::TokioChildProcess,
    handler::ClientHandler,
};
```

### Client Workflow

```rust
// 1. Create service with transport
let service = TokioChildProcess::new(command).service();

// 2. Initialize
service.peer_info().await?;

// 3. Discover tools
let tools = service.list_tools().await?;

// 4. Call tools
let result = service.call_tool(params).await?;
```

---

## Migration Strategy

### Phase 1: Type Replacement (types.rs)

**Action**: Replace custom types with rmcp equivalents

**Mapping**:

| Custom Type | rmcp Equivalent | Notes |
|-------------|----------------|-------|
| `JsonRpcRequest` | `rmcp::protocol::jsonrpc::Request` | Built-in JSON-RPC handling |
| `JsonRpcResponse` | `rmcp::protocol::jsonrpc::Response` | |
| `JsonRpcError` | `rmcp::protocol::jsonrpc::Error` | |
| `McpTool` | `rmcp::model::Tool` | Check field compatibility |
| `McpResource` | `rmcp::model::Resource` | |
| `McpPrompt` | `rmcp::model::Prompt` | |
| `CallToolParams` | `rmcp::model::CallToolRequestParam` | |
| `CallToolResult` | `rmcp::model::CallToolResult` | |
| `ServerCapabilities` | `rmcp::model::ServerCapabilities` | |
| `ClientCapabilities` | `rmcp::model::ClientCapabilities` | |

**Approach**:
1. Create type aliases for backward compatibility
2. Incrementally replace usage across codebase
3. Remove custom types once migration complete

```rust
// types.rs (transitional)
pub use rmcp::model::{
    Tool as McpTool,
    Resource as McpResource,
    Prompt as McpPrompt,
    CallToolRequestParam as CallToolParams,
    CallToolResult,
    ServerCapabilities,
    ClientCapabilities,
};
```

### Phase 2: Transport Migration (transport.rs)

**Action**: Replace `StdioTransport` with `rmcp::transport::TokioChildProcess`

**Changes**:
- Remove custom stdio implementation
- Use `rmcp::transport::io::StdioTransport` or `TokioChildProcess`
- Adapter pattern if interface differs

**Code Change**:
```rust
// OLD
use super::transport::{StdioTransport, Transport};
let transport = StdioTransport::new(&command, &args).await?;

// NEW
use rmcp::transport::TokioChildProcess;
let transport = TokioChildProcess::new(command_with_args);
```

### Phase 3: Client Refactoring (client.rs)

**Action**: Refactor `McpClient` to use `rmcp::service::Service`

**Strategy**: Wrapper approach
- Keep `McpClient` public API unchanged
- Internally use `rmcp::service::Service` trait
- Manage connections as `Arc<dyn Service>`

**Implementation**:
```rust
use rmcp::service::{Service, ServiceExt};
use std::sync::Arc;

pub struct McpClient {
    connections: Arc<RwLock<HashMap<String, Arc<dyn Service>>>>,
}

impl McpClient {
    pub async fn connect(&self, config: &McpServerConfig) -> Result<()> {
        // Create transport
        let transport = TokioChildProcess::new(
            format!("{} {}", config.command, config.args.join(" "))
        );

        // Create service
        let service = transport.service();

        // Initialize
        service.peer_info().await?;

        // Store
        self.connections.write().await.insert(config.name.clone(), service);
        Ok(())
    }

    pub async fn list_tools(&self, server_name: &str) -> Result<Vec<Tool>> {
        let service = self.get_service(server_name)?;
        let tools = service.list_tools().await?;
        Ok(tools)
    }

    pub async fn call_tool(&self, server_name: &str, tool_name: &str, args: Option<Value>)
        -> Result<CallToolResult>
    {
        let service = self.get_service(server_name)?;
        let params = CallToolRequestParam {
            name: tool_name.to_string(),
            arguments: args,
        };
        let result = service.call_tool(params).await?;
        Ok(result)
    }
}
```

**Challenges**:
- rmcp uses higher-level abstractions (Service trait) vs. low-level JSON-RPC
- May need to implement `ClientHandler` trait
- Connection management differs

**Solution**:
- Wrap rmcp Service with existing McpClient interface
- Maintain backward compatibility
- Gradual internal refactoring

### Phase 4: Tool Adapter Updates (tool_adapter.rs)

**Action**: Update tool adapter to work with rmcp types

**Changes**:
- Update imports to use rmcp types
- Adjust type conversions if needed
- Test tool execution flow

**Focus Areas**:
- `McpToolAdapter::adapt_tool()` - Converting rmcp::model::Tool to internal ToolInfo
- Tool execution parameter mapping
- Result handling

### Phase 5: CLI Command Updates (cli/mcp.rs)

**Action**: Update MCP CLI commands to use new client

**Changes**:
- Update type imports
- Adjust error handling
- Update output formatting if needed

**Commands to update**:
- `mcp list`, `mcp add`, `mcp remove`
- `mcp connect`, `mcp disconnect`
- `mcp tools`, `mcp resources`, `mcp prompts`

### Phase 6: Tool Executor Updates (tools/mcp_tool.rs)

**Action**: Update MCP tool executor

**Changes**:
- Update type references
- Ensure tool execution works with rmcp types
- Update error handling

---

## Migration Checklist

### Prerequisites
- [x] rmcp crate added to Cargo.toml (already present)
- [ ] Create migration strategy document
- [ ] Back up current working implementation
- [ ] Create migration branch

### Implementation Steps

1. **Types Migration**
   - [ ] Create type aliases in types.rs
   - [ ] Add rmcp imports
   - [ ] Test compilation
   - [ ] Update all type references

2. **Transport Migration**
   - [ ] Replace StdioTransport with TokioChildProcess
   - [ ] Update Transport enum
   - [ ] Test process spawning
   - [ ] Test communication

3. **Client Migration**
   - [ ] Refactor McpClient to wrap Service
   - [ ] Implement initialize using service.peer_info()
   - [ ] Migrate list_tools, call_tool
   - [ ] Migrate resource operations
   - [ ] Migrate prompt operations
   - [ ] Update connection management

4. **Adapter Updates**
   - [ ] Update tool_adapter.rs imports
   - [ ] Adjust type conversions
   - [ ] Test tool adaptation

5. **CLI Updates**
   - [ ] Update cli/mcp.rs imports
   - [ ] Test all MCP commands
   - [ ] Update help text if needed

6. **Tool Executor Updates**
   - [ ] Update tools/mcp_tool.rs
   - [ ] Test tool execution flow

7. **Testing**
   - [ ] Unit tests pass
   - [ ] Integration tests with Thalora server
   - [ ] Test all MCP operations
   - [ ] Performance testing

8. **Cleanup**
   - [ ] Remove custom JSON-RPC types
   - [ ] Remove custom transport code
   - [ ] Update documentation
   - [ ] Remove obsolete code

### Testing Strategy

**Unit Tests**:
- Test type conversions
- Test client API compatibility
- Test error handling

**Integration Tests**:
- Connect to Thalora MCP server
- List tools from server
- Execute tools
- Read resources
- Test disconnection

**Regression Tests**:
- Verify existing functionality unchanged
- Test with multiple concurrent servers
- Test error scenarios

---

## Risk Assessment

### High Risk
- **Breaking API changes**: rmcp may have different error handling
  - *Mitigation*: Wrapper pattern maintains existing API

- **Service abstraction mismatch**: rmcp uses Service trait, we use direct JSON-RPC
  - *Mitigation*: Study rmcp examples, potentially implement ClientHandler

### Medium Risk
- **Type incompatibilities**: Field name differences, serialization issues
  - *Mitigation*: Careful type mapping, extensive testing

- **Transport behavior differences**: Process management, I/O handling
  - *Mitigation*: Thorough integration testing

### Low Risk
- **Configuration changes**: Unlikely to affect McpServerConfig
- **CLI impact**: Wrapper pattern shields CLI from changes

---

## Rollback Plan

If migration encounters critical issues:

1. **Git branch strategy**: Keep custom implementation in separate branch
2. **Feature flag**: Could add `use-custom-mcp` feature flag temporarily
3. **Revert commits**: Clear commit history for easy rollback

---

## Timeline Estimate

| Phase | Effort | Duration |
|-------|--------|----------|
| 1. Type Replacement | 4 hours | Day 1 |
| 2. Transport Migration | 4 hours | Day 1-2 |
| 3. Client Refactoring | 8 hours | Day 2-3 |
| 4. Adapter Updates | 2 hours | Day 3 |
| 5. CLI Updates | 2 hours | Day 3 |
| 6. Tool Executor | 2 hours | Day 4 |
| 7. Testing & Debug | 8 hours | Day 4-5 |
| 8. Documentation | 2 hours | Day 5 |

**Total**: ~32 hours (~1 week for 1 developer)

---

## Success Criteria

- [ ] All unit tests pass
- [ ] Integration tests with Thalora server successful
- [ ] No regression in existing MCP functionality
- [ ] Code size reduced by ~50% (removing custom implementation)
- [ ] Type safety improved (using official types)
- [ ] Documentation updated
- [ ] Zero breaking changes to public API

---

## References

- [rmcp crate documentation](https://docs.rs/rmcp/0.8.5)
- [rmcp GitHub repository](https://github.com/modelcontextprotocol/rust-sdk)
- [MCP Specification](https://modelcontextprotocol.io)
- [Current MCP implementation](../src/mcp/)
- [Thalora MCP setup docs](./THALORA_MCP_SETUP.md)

---

## Next Steps

1. Review and approve this strategy
2. Create migration branch: `feat/mcp-rmcp-migration`
3. Begin Phase 1: Type Replacement
4. Iterative development with testing at each phase

---

**Document Version**: 1.0
**Created**: 2025-01-17
**Author**: Brainwires Team
**Status**: Draft - Awaiting Approval
