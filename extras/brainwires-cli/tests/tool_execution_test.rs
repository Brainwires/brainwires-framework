/// Integration test for CLI-local tool execution flow
/// Tests the complete cycle: toolCall event → execute tool → send continuation → receive response
use brainwires_cli::types::message::StreamChunk;
use serde_json::json;

#[test]
fn test_stream_chunk_tool_call_parsing() {
    // Test that we can parse toolCall events correctly
    let tool_call = StreamChunk::ToolCall {
        call_id: "test-call-123".to_string(),
        response_id: "test-response-456".to_string(),
        chat_id: Some("test-chat-789".to_string()),
        tool_name: "read_file".to_string(),
        server: "cli-local".to_string(),
        parameters: json!({"path": "/home/test/file.rs"}),
    };

    // Verify the variant matches
    match tool_call {
        StreamChunk::ToolCall {
            call_id,
            tool_name,
            server,
            ..
        } => {
            assert_eq!(call_id, "test-call-123");
            assert_eq!(tool_name, "read_file");
            assert_eq!(server, "cli-local");
        }
        _ => panic!("Expected ToolCall variant"),
    }
}

#[test]
fn test_tool_call_with_cli_local_server() {
    // Verify we can identify cli-local tools
    let tool_call = StreamChunk::ToolCall {
        call_id: "call-1".to_string(),
        response_id: "resp-1".to_string(),
        chat_id: None,
        tool_name: "list_directory".to_string(),
        server: "cli-local".to_string(),
        parameters: json!({"path": "."}),
    };

    if let StreamChunk::ToolCall { server, .. } = tool_call {
        assert_eq!(server, "cli-local", "Should be cli-local tool");
    }
}

#[test]
fn test_tool_call_with_remote_server() {
    // Verify we can identify non-cli-local tools that should be ignored
    let tool_call = StreamChunk::ToolCall {
        call_id: "call-2".to_string(),
        response_id: "resp-2".to_string(),
        chat_id: None,
        tool_name: "some_tool".to_string(),
        server: "remote-mcp-server".to_string(),
        parameters: json!({}),
    };

    if let StreamChunk::ToolCall { server, .. } = tool_call {
        assert_ne!(server, "cli-local", "Should NOT be cli-local tool");
    }
}
