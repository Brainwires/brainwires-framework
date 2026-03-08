//! Typed request parameter structs for all A2A methods.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::agent_card::AgentCard;
use crate::push_notification::TaskPushNotificationConfig;
use crate::task::{Task, TaskState};
use crate::types::Message;

/// Configuration for a send-message request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SendMessageConfiguration {
    /// Accepted output media types.
    #[serde(
        rename = "acceptedOutputModes",
        skip_serializing_if = "Option::is_none"
    )]
    pub accepted_output_modes: Option<Vec<String>>,
    /// Push notification configuration.
    #[serde(
        rename = "taskPushNotificationConfig",
        skip_serializing_if = "Option::is_none"
    )]
    pub task_push_notification_config: Option<TaskPushNotificationConfig>,
    /// Max number of history messages to return.
    #[serde(rename = "historyLength", skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
    /// If true, wait for terminal/interrupted state before returning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking: Option<bool>,
}

/// Request parameters for `message/send` and `message/stream`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// The message to send.
    pub message: Message,
    /// Request configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<SendMessageConfiguration>,
    /// Custom metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Request parameters for `tasks/get`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Task identifier.
    pub id: String,
    /// Max number of history messages to return.
    #[serde(rename = "historyLength", skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
}

/// Request parameters for `tasks/list`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ListTasksRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Filter by context ID.
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
    /// Filter by task state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskState>,
    /// Maximum number of tasks to return.
    #[serde(rename = "pageSize", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    /// Pagination token.
    #[serde(rename = "pageToken", skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
    /// Max history messages per task.
    #[serde(rename = "historyLength", skip_serializing_if = "Option::is_none")]
    pub history_length: Option<i32>,
    /// Filter tasks with status updated after this ISO 8601 timestamp.
    #[serde(
        rename = "statusTimestampAfter",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_timestamp_after: Option<String>,
    /// Whether to include artifacts.
    #[serde(rename = "includeArtifacts", skip_serializing_if = "Option::is_none")]
    pub include_artifacts: Option<bool>,
}

/// Response for `tasks/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTasksResponse {
    /// Matching tasks.
    pub tasks: Vec<Task>,
    /// Pagination token for next page.
    #[serde(rename = "nextPageToken")]
    pub next_page_token: String,
    /// Page size used.
    #[serde(rename = "pageSize")]
    pub page_size: i32,
    /// Total number of matching tasks.
    #[serde(rename = "totalSize")]
    pub total_size: i32,
}

/// Request parameters for `tasks/cancel`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Task identifier.
    pub id: String,
    /// Custom metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Request parameters for `tasks/resubscribe`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeToTaskRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Task identifier.
    pub id: String,
}

/// Request for `tasks/pushNotificationConfig/get`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTaskPushNotificationConfigRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Parent task identifier.
    #[serde(rename = "taskId")]
    pub task_id: String,
    /// Configuration identifier.
    pub id: String,
}

/// Request for `tasks/pushNotificationConfig/delete`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteTaskPushNotificationConfigRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Parent task identifier.
    #[serde(rename = "taskId")]
    pub task_id: String,
    /// Configuration identifier.
    pub id: String,
}

/// Request for `tasks/pushNotificationConfig/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTaskPushNotificationConfigsRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
    /// Parent task identifier.
    #[serde(rename = "taskId")]
    pub task_id: String,
    /// Maximum configs to return.
    #[serde(rename = "pageSize", skip_serializing_if = "Option::is_none")]
    pub page_size: Option<i32>,
    /// Pagination token.
    #[serde(rename = "pageToken", skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,
}

/// Response for listing push notification configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListTaskPushNotificationConfigsResponse {
    /// The configs.
    pub configs: Vec<TaskPushNotificationConfig>,
    /// Pagination token for next page.
    #[serde(rename = "nextPageToken", skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Request for `agent/authenticatedExtendedCard`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GetExtendedAgentCardRequest {
    /// Optional tenant identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Response for the extended agent card.
pub type GetExtendedAgentCardResponse = AgentCard;
