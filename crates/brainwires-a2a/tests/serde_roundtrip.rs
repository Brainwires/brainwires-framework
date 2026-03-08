//! Serde round-trip tests for A2A types.

use brainwires_a2a::*;

fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug>(val: &T) {
    let json = serde_json::to_string(val).expect("serialize");
    let deserialized: T = serde_json::from_str(&json).expect("deserialize");
    let json2 = serde_json::to_string(&deserialized).expect("re-serialize");
    assert_eq!(json, json2, "round-trip mismatch");
}

#[test]
fn test_message_roundtrip() {
    let msg = Message {
        message_id: "msg-1".into(),
        role: Role::User,
        parts: vec![Part::Text {
            text: "Hello agent".into(),
            metadata: None,
        }],
        context_id: Some("ctx-1".into()),
        task_id: None,
        reference_task_ids: None,
        metadata: None,
        extensions: None,
        kind: "message".into(),
    };
    roundtrip(&msg);
}

#[test]
fn test_message_with_file_part() {
    let msg = Message {
        message_id: "msg-2".into(),
        role: Role::Agent,
        parts: vec![Part::File {
            file: FileContent::Uri {
                uri: "https://example.com/file.pdf".into(),
                mime_type: Some("application/pdf".into()),
                name: Some("file.pdf".into()),
            },
            metadata: None,
        }],
        context_id: None,
        task_id: Some("task-1".into()),
        reference_task_ids: Some(vec!["ref-1".into()]),
        metadata: None,
        extensions: None,
        kind: "message".into(),
    };
    roundtrip(&msg);
}

#[test]
fn test_task_roundtrip() {
    let task = Task {
        id: "task-1".into(),
        context_id: Some("ctx-1".into()),
        status: TaskStatus {
            state: TaskState::Working,
            message: None,
            timestamp: Some("2025-01-01T00:00:00Z".into()),
        },
        artifacts: Some(vec![Artifact {
            artifact_id: "art-1".into(),
            name: Some("output".into()),
            description: None,
            parts: vec![Part::Text {
                text: "result".into(),
                metadata: None,
            }],
            metadata: None,
            extensions: None,
        }]),
        history: None,
        metadata: None,
        kind: "task".into(),
    };
    roundtrip(&task);
}

#[test]
fn test_task_state_serde() {
    // All states should serialize to kebab-case
    let states = vec![
        (TaskState::Unknown, "\"unknown\""),
        (TaskState::Submitted, "\"submitted\""),
        (TaskState::Working, "\"working\""),
        (TaskState::Completed, "\"completed\""),
        (TaskState::Failed, "\"failed\""),
        (TaskState::Canceled, "\"canceled\""),
        (TaskState::Rejected, "\"rejected\""),
        (TaskState::InputRequired, "\"input-required\""),
        (TaskState::AuthRequired, "\"auth-required\""),
    ];
    for (state, expected) in states {
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, expected, "TaskState::{state:?} serialization mismatch");
        let back: TaskState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
    }
}

#[test]
fn test_agent_card_roundtrip() {
    let card = AgentCard {
        name: "Test Agent".into(),
        description: "A test agent".into(),
        version: "1.0.0".into(),
        supported_interfaces: Some(vec![AgentInterface {
            url: "https://agent.example.com".into(),
            protocol_binding: "JSONRPC".into(),
            tenant: None,
            protocol_version: "0.3".into(),
        }]),
        capabilities: AgentCapabilities {
            streaming: Some(true),
            push_notifications: Some(false),
            extended_agent_card: None,
            extensions: None,
        },
        skills: vec![AgentSkill {
            id: "skill-1".into(),
            name: "Chat".into(),
            description: "Basic chat".into(),
            tags: vec!["chat".into()],
            examples: Some(vec!["Hello".into()]),
            input_modes: None,
            output_modes: None,
            security_requirements: None,
        }],
        default_input_modes: vec!["text/plain".into()],
        default_output_modes: vec!["text/plain".into()],
        provider: Some(AgentProvider {
            url: "https://example.com".into(),
            organization: "Test Org".into(),
        }),
        security_schemes: None,
        security_requirements: None,
        documentation_url: None,
        icon_url: None,
        signatures: None,
    };
    roundtrip(&card);
}

#[test]
fn test_jsonrpc_roundtrip() {
    let req = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "message/send".into(),
        params: Some(serde_json::json!({"message": {"messageId": "1", "role": "user", "parts": []}})),
        id: RequestId::String("req-1".into()),
    };
    roundtrip(&req);

    let resp = JsonRpcResponse::success(
        RequestId::Number(42),
        serde_json::json!({"id": "task-1", "status": {"state": "working"}}),
    );
    roundtrip(&resp);
}

#[test]
fn test_error_roundtrip() {
    let err = A2aError::task_not_found("task-42");
    let json = serde_json::to_string(&err).unwrap();
    let back: A2aError = serde_json::from_str(&json).unwrap();
    assert_eq!(back.code, err.code);
    assert_eq!(back.message, err.message);
}

#[test]
fn test_stream_event_roundtrip() {
    let event = TaskStatusUpdateEvent {
        task_id: "t-1".into(),
        context_id: "c-1".into(),
        status: TaskStatus {
            state: TaskState::Completed,
            message: None,
            timestamp: None,
        },
        metadata: None,
    };
    roundtrip(&event);

    let event = TaskArtifactUpdateEvent {
        task_id: "t-1".into(),
        context_id: "c-1".into(),
        artifact: Artifact {
            artifact_id: "a-1".into(),
            name: None,
            description: None,
            parts: vec![Part::Data {
                data: serde_json::json!({"key": "value"}),
                metadata: None,
            }],
            metadata: None,
            extensions: None,
        },
        append: Some(true),
        last_chunk: Some(false),
        metadata: None,
    };
    roundtrip(&event);
}

#[test]
fn test_push_notification_roundtrip() {
    let config = TaskPushNotificationConfig {
        tenant: None,
        id: Some("cfg-1".into()),
        task_id: "task-1".into(),
        url: "https://webhook.example.com/notify".into(),
        token: Some("secret-token".into()),
        authentication: Some(AuthenticationInfo {
            scheme: "Bearer".into(),
            credentials: Some("abc123".into()),
        }),
    };
    roundtrip(&config);
}

#[test]
fn test_send_message_request_roundtrip() {
    let req = SendMessageRequest {
        tenant: None,
        message: Message::user_text("Hello"),
        configuration: Some(SendMessageConfiguration {
            accepted_output_modes: Some(vec!["text/plain".into()]),
            task_push_notification_config: None,
            history_length: Some(10),
            blocking: Some(true),
        }),
        metadata: None,
    };
    roundtrip(&req);
}

#[test]
fn test_security_scheme_roundtrip() {
    let scheme = SecurityScheme::Http {
        scheme: "Bearer".into(),
        bearer_format: Some("JWT".into()),
        description: None,
    };
    roundtrip(&scheme);

    let scheme = SecurityScheme::ApiKey {
        name: "X-API-Key".into(),
        location: "header".into(),
        description: Some("API key header".into()),
    };
    roundtrip(&scheme);
}

#[test]
fn test_message_helpers() {
    let user_msg = Message::user_text("test");
    assert_eq!(user_msg.role, Role::User);
    assert_eq!(user_msg.kind, "message");
    assert!(!user_msg.message_id.is_empty());

    let agent_msg = Message::agent_text("response");
    assert_eq!(agent_msg.role, Role::Agent);
}
