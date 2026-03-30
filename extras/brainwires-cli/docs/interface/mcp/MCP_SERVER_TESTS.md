# MCP Server Test Documentation

This document describes the test coverage for the MCP server functionality.

## Overview

The MCP server implementation includes three levels of testing:

1. **Unit Tests** - Testing individual components in isolation
2. **Integration Tests** - Testing the full MCP server over stdin/stdout
3. **Manual Tests** - Interactive testing with MCP clients

## Unit Tests

### Handler Tests (`src/mcp_server/handler.rs`)

#### Basic Handler Tests
- **`test_handler_creation`** - Verifies handler can be created with valid configuration
- **`test_initialize_request`** - Tests MCP initialize handshake
- **`test_list_tools_request`** - Verifies tool listing returns all registered tools
- **`test_invalid_method`** - Ensures proper error handling for unknown methods

#### Agent Management Tests
- **`test_agent_list_initially_empty`** - Verifies no agents are running initially
- **`test_agent_spawn_missing_task_param`** - Tests error handling for missing parameters
- **`test_get_nonexistent_agent_status`** - Tests querying non-existent agents
- **`test_stop_nonexistent_agent`** - Tests stopping non-existent agents

#### JSON-RPC Protocol Tests
- **`test_next_request_id_increments`** - Verifies request ID counter works
- **`test_json_rpc_request_structure`** - Tests request serialization
- **`test_json_rpc_response_structure`** - Tests response serialization
- **`test_json_rpc_error_structure`** - Tests error response format

**To run:**
```bash
cargo test --lib mcp_server::handler::tests
```

### Agent Tools Tests (`src/mcp_server/agent_tools.rs`)

#### Registry Tests
- **`test_registry_creation`** - Verifies registry creates 4 agent tools
- **`test_default_creation`** - Tests default trait implementation

#### Individual Tool Tests
- **`test_agent_spawn_tool`** - Validates spawn tool schema and description
- **`test_agent_list_tool`** - Validates list tool has empty parameters
- **`test_agent_status_tool`** - Validates status tool requires agent_id
- **`test_agent_stop_tool`** - Validates stop tool requires agent_id

#### Schema Validation Tests
- **`test_all_tools_have_descriptions`** - Ensures all tools are documented
- **`test_all_tools_have_object_schemas`** - Verifies schema consistency
- **`test_tool_names_are_prefixed`** - Checks naming convention (agent_*)
- **`test_no_approval_required`** - Verifies autonomous operation capability
- **`test_schema_serialization`** - Tests JSON schema serialization

**To run:**
```bash
cargo test --lib mcp_server::agent_tools::tests
```

## Integration Tests

### MCP Protocol Tests (`tests/mcp_server_integration.rs`)

#### Basic Protocol Tests
- **`test_mcp_server_initialize`**
  - Spawns MCP server process
  - Sends initialize request
  - Validates response structure and capabilities
  - Verifies clean shutdown

- **`test_mcp_server_list_tools`**
  - Tests full initialize → tools/list flow
  - Validates all agent tools are present
  - Checks tool schemas are correct

- **`test_mcp_server_invalid_method`**
  - Tests error handling for invalid JSON-RPC methods
  - Validates error code (-32601 Method not found)

#### Tool Schema Validation
- **`test_agent_tool_schemas`**
  - Deep validation of tool input schemas
  - Verifies required parameters
  - Checks parameter types and descriptions

**To run:**
```bash
cargo test --test mcp_server_integration
```

**Note:** Integration tests spawn actual processes and may take longer to run.

## Manual Testing

### Test with Echo Client

Test basic JSON-RPC communication:

```bash
# Start MCP server
./target/debug/brainwires chat --mcp-server

# In another terminal, send requests:
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | \
  ./target/debug/brainwires chat --mcp-server

echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | \
  ./target/debug/brainwires chat --mcp-server
```

### Test with Claude Desktop

1. Add to Claude Desktop MCP configuration:
```json
{
  "mcpServers": {
    "brainwires": {
      "command": "/path/to/brainwires",
      "args": ["chat", "--mcp-server"]
    }
  }
}
```

2. Restart Claude Desktop
3. Use agent tools in conversation:
   - "Use agent_spawn to analyze this codebase"
   - "Use agent_list to show running agents"

### Test Agent Spawning

```bash
# Start server
./target/debug/brainwires chat --mcp-server

# Send agent_spawn request
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"agent_spawn","arguments":{"task":"List files in current directory"}}}' | \
  ./target/debug/brainwires chat --mcp-server

# List agents
echo '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"agent_list","arguments":{}}}' | \
  ./target/debug/brainwires chat --mcp-server
```

## Test Coverage Summary

### What is Tested

✅ **Handler Initialization**
- Server creation with different configurations
- Initialize handshake with protocol negotiation
- Server capability advertisement

✅ **Tool Listing**
- All tools are registered and exposed
- Tool schemas are valid JSON Schema
- Required parameters are marked correctly

✅ **Error Handling**
- Invalid methods return proper error codes
- Missing parameters are caught
- Non-existent resources return errors

✅ **JSON-RPC Protocol**
- Request/response structure compliance
- Error format compliance
- ID tracking and incrementing

✅ **Agent Tool Schemas**
- All 4 agent tools present
- Schemas follow MCP specification
- Parameters properly typed

### What Needs Additional Testing

⚠️ **Agent Execution** (Requires running AI provider)
- Actual agent spawning and execution
- Agent communication hub
- File lock management
- Agent completion and result reporting

⚠️ **Tool Execution** (Integration test)
- Calling regular tools through MCP
- Tool result formatting
- Error propagation

⚠️ **Concurrent Agents** (Load testing)
- Multiple agents running simultaneously
- Resource contention handling
- Performance under load

⚠️ **Long-Running Operations**
- Agent timeout handling
- Iteration limits
- Memory management

## Running All Tests

```bash
# Run all unit tests
cargo test --lib mcp_server

# Run integration tests
cargo test --test mcp_server_integration

# Run all tests with output
cargo test mcp_server -- --nocapture

# Run specific test
cargo test mcp_server::handler::tests::test_initialize_request
```

## Debugging Tests

Enable logging for tests:
```bash
RUST_LOG=debug cargo test mcp_server -- --nocapture
```

Run single test in isolation:
```bash
cargo test --lib mcp_server::handler::tests::test_handler_creation -- --exact
```

## Continuous Integration

Tests should be run on:
- Every commit to main
- All pull requests
- Before releases

Recommended CI configuration:
```yaml
test:
  - cargo test --lib mcp_server
  - cargo test --test mcp_server_integration
  - cargo clippy -- -D warnings
  - cargo fmt -- --check
```

## Known Issues

1. **Git tool tests** - Pre-existing test failures in `src/tools/git.rs` prevent full test suite from running. These are unrelated to MCP server functionality.

2. **Provider requirement** - Some integration tests require a configured AI provider (Anthropic API key or Brainwires session).

## Future Test Additions

- [ ] Load testing with multiple concurrent agents
- [ ] Stress testing with complex task hierarchies
- [ ] Memory leak detection for long-running servers
- [ ] Protocol compliance test suite
- [ ] Fuzzing for input validation
- [ ] Performance benchmarks
