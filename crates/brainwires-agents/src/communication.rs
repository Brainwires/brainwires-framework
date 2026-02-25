use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};

use crate::resource_locks::ResourceScope;

/// Types of messages agents can send to each other
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentMessage {
    /// Request to execute a task
    TaskRequest {
        task_id: String,
        description: String,
        priority: u8,
    },
    /// Result of task execution
    TaskResult {
        task_id: String,
        success: bool,
        result: String,
    },
    /// Status update
    StatusUpdate {
        agent_id: String,
        status: String,
        details: Option<String>,
    },
    /// Request for help/collaboration
    HelpRequest {
        request_id: String,
        topic: String,
        details: String,
    },
    /// Response to help request
    HelpResponse {
        request_id: String,
        response: String,
    },
    /// Broadcast message to all agents
    Broadcast {
        sender: String,
        message: String,
    },
    /// Custom message with arbitrary data
    Custom {
        message_type: String,
        data: serde_json::Value,
    },
    /// Notification that an agent was spawned
    AgentSpawned {
        agent_id: String,
        task_id: String,
    },
    /// Progress update from an agent
    AgentProgress {
        agent_id: String,
        progress_percent: u8,
        message: String,
    },
    /// Notification that an agent completed
    AgentCompleted {
        agent_id: String,
        task_id: String,
        summary: String,
    },
    /// Notification about lock contention
    LockContention {
        agent_id: String,
        path: String,
        waiting_for: String,
    },
    /// Request for approval (dangerous operation)
    ApprovalRequest {
        request_id: String,
        agent_id: String,
        operation: String,
        details: String,
    },
    /// Response to approval request
    ApprovalResponse {
        request_id: String,
        approved: bool,
        reason: Option<String>,
    },

    // === New messages for agent coordination ===

    /// Notification that an exclusive operation has started
    OperationStarted {
        agent_id: String,
        operation_type: OperationType,
        scope: String,
        estimated_duration_ms: Option<u64>,
        description: String,
    },
    /// Notification that an exclusive operation has completed
    OperationCompleted {
        agent_id: String,
        operation_type: OperationType,
        scope: String,
        success: bool,
        duration_ms: u64,
        summary: String,
    },
    /// Notification that a lock has become available
    LockAvailable {
        operation_type: OperationType,
        scope: String,
        released_by: String,
    },
    /// Update on wait queue position
    WaitQueuePosition {
        agent_id: String,
        operation_type: OperationType,
        scope: String,
        position: usize,
        estimated_wait_ms: Option<u64>,
    },
    /// Git operation started
    GitOperationStarted {
        agent_id: String,
        git_op: GitOperationType,
        branch: Option<String>,
        description: String,
    },
    /// Git operation completed
    GitOperationCompleted {
        agent_id: String,
        git_op: GitOperationType,
        success: bool,
        summary: String,
    },
    /// Build blocked due to conflicts
    BuildBlocked {
        agent_id: String,
        reason: String,
        conflicts: Vec<ConflictInfo>,
        estimated_wait_ms: Option<u64>,
    },
    /// File write blocked due to conflicts
    FileWriteBlocked {
        agent_id: String,
        path: String,
        reason: String,
        conflicts: Vec<ConflictInfo>,
    },
    /// Resource conflict resolved - agent can proceed
    ConflictResolved {
        agent_id: String,
        operation_type: OperationType,
        scope: String,
    },

    // === Saga Protocol Messages ===

    /// A saga (multi-step transaction) has started
    SagaStarted {
        saga_id: String,
        agent_id: String,
        description: String,
        total_steps: usize,
    },
    /// A saga step has completed
    SagaStepCompleted {
        saga_id: String,
        agent_id: String,
        step_index: usize,
        step_name: String,
        success: bool,
    },
    /// A saga has completed (successfully or with compensation)
    SagaCompleted {
        saga_id: String,
        agent_id: String,
        success: bool,
        compensated: bool,
        summary: String,
    },
    /// A saga is being compensated (rolling back)
    SagaCompensating {
        saga_id: String,
        agent_id: String,
        reason: String,
        steps_to_compensate: usize,
    },

    // === Contract-Net Protocol Messages ===

    /// A task has been announced for bidding
    TaskAnnounced {
        task_id: String,
        announcer: String,
        description: String,
        bid_deadline_ms: u64,
    },
    /// An agent has submitted a bid
    BidSubmitted {
        task_id: String,
        agent_id: String,
        capability_score: f32,
        current_load: f32,
    },
    /// A task has been awarded to an agent
    TaskAwarded {
        task_id: String,
        winner: String,
        announcer: String,
    },
    /// An agent has accepted an awarded task
    TaskAccepted {
        task_id: String,
        agent_id: String,
    },
    /// An agent has declined an awarded task
    TaskDeclined {
        task_id: String,
        agent_id: String,
        reason: String,
    },

    // === Market Allocation Messages ===

    /// A resource is available for bidding
    ResourceAvailable {
        resource_id: String,
        resource_type: String,
    },
    /// A resource bid has been submitted
    ResourceBidSubmitted {
        resource_id: String,
        agent_id: String,
        priority: u8,
        urgency: f32,
    },
    /// A resource has been allocated to an agent
    ResourceAllocated {
        resource_id: String,
        agent_id: String,
        price: u32,
    },
    /// A resource has been released
    ResourceReleased {
        resource_id: String,
        agent_id: String,
    },

    // === Worktree Messages ===

    /// A worktree has been created for an agent
    WorktreeCreated {
        agent_id: String,
        worktree_path: String,
        branch: String,
    },
    /// A worktree has been removed
    WorktreeRemoved {
        agent_id: String,
        worktree_path: String,
    },
    /// An agent is switching worktrees
    WorktreeSwitched {
        agent_id: String,
        from_path: Option<String>,
        to_path: String,
    },

    // === Validation Messages ===

    /// A validation check has failed
    ValidationFailed {
        agent_id: String,
        operation: String,
        rule_name: String,
        message: String,
    },
    /// A validation warning was raised
    ValidationWarning {
        agent_id: String,
        operation: String,
        rule_name: String,
        message: String,
    },

    // === Optimistic Concurrency Messages ===

    /// A version conflict was detected
    VersionConflict {
        resource_id: String,
        agent_id: String,
        expected_version: u64,
        actual_version: u64,
    },
    /// A conflict has been resolved
    ConflictResolutionApplied {
        resource_id: String,
        resolution_type: String,
        winning_agent: Option<String>,
    },
}

/// Types of operations that require coordination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationType {
    /// Build operation (cargo build, npm build, etc.)
    Build,
    /// Test operation (cargo test, npm test, etc.)
    Test,
    /// Combined build and test
    BuildTest,
    /// Git index/staging operations
    GitIndex,
    /// Git commit operations
    GitCommit,
    /// Git push operations
    GitPush,
    /// Git pull operations
    GitPull,
    /// Git branch operations
    GitBranch,
    /// File write operation
    FileWrite,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::Build => write!(f, "Build"),
            OperationType::Test => write!(f, "Test"),
            OperationType::BuildTest => write!(f, "BuildTest"),
            OperationType::GitIndex => write!(f, "GitIndex"),
            OperationType::GitCommit => write!(f, "GitCommit"),
            OperationType::GitPush => write!(f, "GitPush"),
            OperationType::GitPull => write!(f, "GitPull"),
            OperationType::GitBranch => write!(f, "GitBranch"),
            OperationType::FileWrite => write!(f, "FileWrite"),
        }
    }
}

/// Git-specific operation types for finer-grained control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitOperationType {
    /// Read-only operations (status, diff, log, fetch)
    ReadOnly,
    /// Staging operations (stage, unstage)
    Staging,
    /// Commit operations
    Commit,
    /// Remote write operations (push)
    RemoteWrite,
    /// Remote read/merge operations (pull)
    RemoteMerge,
    /// Branch operations (create, switch, delete)
    Branch,
    /// Destructive operations (discard)
    Destructive,
}

impl std::fmt::Display for GitOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitOperationType::ReadOnly => write!(f, "ReadOnly"),
            GitOperationType::Staging => write!(f, "Staging"),
            GitOperationType::Commit => write!(f, "Commit"),
            GitOperationType::RemoteWrite => write!(f, "RemoteWrite"),
            GitOperationType::RemoteMerge => write!(f, "RemoteMerge"),
            GitOperationType::Branch => write!(f, "Branch"),
            GitOperationType::Destructive => write!(f, "Destructive"),
        }
    }
}

/// Information about a conflict blocking an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Agent holding the conflicting resource
    pub holder_agent: String,
    /// Resource identifier (path or scope)
    pub resource: String,
    /// How long the conflict has been active (seconds)
    pub duration_secs: u64,
    /// Current status of the blocking operation
    pub status: String,
}

/// Types of conflicts that can block operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConflictType {
    /// File write lock blocks build
    FileWriteBlocksBuild { path: PathBuf },
    /// Build in progress blocks file write
    BuildBlocksFileWrite,
    /// Test in progress blocks file write
    TestBlocksFileWrite,
    /// Git operation blocks file write
    GitBlocksFileWrite,
    /// File write blocks git operation
    FileWriteBlocksGit { path: PathBuf },
    /// Build blocks git operation
    BuildBlocksGit,
}

/// Envelope containing message metadata
#[derive(Debug, Clone)]
pub struct MessageEnvelope {
    pub from: String,
    pub to: String,
    pub message: AgentMessage,
    pub timestamp: std::time::SystemTime,
}

impl MessageEnvelope {
    pub fn new(from: String, to: String, message: AgentMessage) -> Self {
        Self {
            from,
            to,
            message,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Agent communication channel
pub struct AgentChannel {
    sender: mpsc::UnboundedSender<MessageEnvelope>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<MessageEnvelope>>>,
}

impl AgentChannel {
    /// Create a new agent channel
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    /// Send a message on this channel
    pub fn send(&self, envelope: MessageEnvelope) -> Result<()> {
        self.sender
            .send(envelope)
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))
    }

    /// Receive a message from this channel (async, blocking)
    pub async fn receive(&self) -> Option<MessageEnvelope> {
        self.receiver.lock().await.recv().await
    }

    /// Try to receive a message without blocking
    pub async fn try_receive(&self) -> Option<MessageEnvelope> {
        self.receiver.lock().await.try_recv().ok()
    }
}

impl Default for AgentChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// Communication hub for managing multiple agent channels
pub struct CommunicationHub {
    channels: Arc<RwLock<HashMap<String, AgentChannel>>>,
    broadcast_channel: AgentChannel,
}

impl CommunicationHub {
    /// Create a new communication hub
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            broadcast_channel: AgentChannel::new(),
        }
    }

    /// Register an agent with the hub
    #[tracing::instrument(name = "agent.register", skip(self))]
    pub async fn register_agent(&self, agent_id: String) -> Result<()> {
        let mut channels = self.channels.write().await;
        if channels.contains_key(&agent_id) {
            anyhow::bail!("Agent {} is already registered", agent_id);
        }
        channels.insert(agent_id.clone(), AgentChannel::new());
        Ok(())
    }

    /// Unregister an agent from the hub
    #[tracing::instrument(name = "agent.unregister", skip(self))]
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<()> {
        let mut channels = self.channels.write().await;
        if channels.remove(agent_id).is_none() {
            anyhow::bail!("Agent {} is not registered", agent_id);
        }
        Ok(())
    }

    /// Send a message from one agent to another
    #[tracing::instrument(name = "agent.send_message", skip(self, message))]
    pub async fn send_message(
        &self,
        from: String,
        to: String,
        message: AgentMessage,
    ) -> Result<()> {
        let channels = self.channels.read().await;
        let channel = channels
            .get(&to)
            .ok_or_else(|| anyhow::anyhow!("Agent {} is not registered", to))?;

        let envelope = MessageEnvelope::new(from, to, message);
        channel.send(envelope)
    }

    /// Broadcast a message to all agents
    #[tracing::instrument(name = "agent.broadcast", skip(self, message))]
    pub async fn broadcast(&self, from: String, message: AgentMessage) -> Result<()> {
        let channels = self.channels.read().await;
        for (agent_id, channel) in channels.iter() {
            let envelope = MessageEnvelope::new(from.clone(), agent_id.clone(), message.clone());
            channel.send(envelope)?;
        }
        Ok(())
    }

    /// Receive a message for a specific agent
    pub async fn receive_message(&self, agent_id: &str) -> Option<MessageEnvelope> {
        let channels = self.channels.read().await;
        if let Some(channel) = channels.get(agent_id) {
            channel.receive().await
        } else {
            None
        }
    }

    /// Try to receive a message without blocking
    pub async fn try_receive_message(&self, agent_id: &str) -> Option<MessageEnvelope> {
        let channels = self.channels.read().await;
        if let Some(channel) = channels.get(agent_id) {
            channel.try_receive().await
        } else {
            None
        }
    }

    /// Get the number of registered agents
    pub async fn agent_count(&self) -> usize {
        self.channels.read().await.len()
    }

    /// Get list of registered agent IDs
    pub async fn list_agents(&self) -> Vec<String> {
        self.channels.read().await.keys().cloned().collect()
    }

    /// Check if an agent is registered
    pub async fn is_registered(&self, agent_id: &str) -> bool {
        self.channels.read().await.contains_key(agent_id)
    }
}

impl Default for CommunicationHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_channel() {
        let channel = AgentChannel::new();
        let envelope = MessageEnvelope::new(
            "agent-1".to_string(),
            "agent-2".to_string(),
            AgentMessage::StatusUpdate {
                agent_id: "agent-1".to_string(),
                status: "working".to_string(),
                details: None,
            },
        );

        channel.send(envelope.clone()).unwrap();
        let received = channel.receive().await;
        assert!(received.is_some());
        assert_eq!(received.unwrap().from, "agent-1");
    }

    #[tokio::test]
    async fn test_communication_hub_register() {
        let hub = CommunicationHub::new();

        hub.register_agent("agent-1".to_string()).await.unwrap();
        assert_eq!(hub.agent_count().await, 1);
        assert!(hub.is_registered("agent-1").await);

        // Try to register again - should fail
        let result = hub.register_agent("agent-1".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_receive_message() {
        let hub = CommunicationHub::new();

        hub.register_agent("agent-1".to_string()).await.unwrap();
        hub.register_agent("agent-2".to_string()).await.unwrap();

        let message = AgentMessage::TaskRequest {
            task_id: "task-1".to_string(),
            description: "Do something".to_string(),
            priority: 5,
        };

        hub.send_message("agent-1".to_string(), "agent-2".to_string(), message)
            .await
            .unwrap();

        let received = hub.receive_message("agent-2").await;
        assert!(received.is_some());

        let envelope = received.unwrap();
        assert_eq!(envelope.from, "agent-1");
        assert_eq!(envelope.to, "agent-2");
    }

    #[tokio::test]
    async fn test_broadcast() {
        let hub = CommunicationHub::new();

        hub.register_agent("agent-1".to_string()).await.unwrap();
        hub.register_agent("agent-2".to_string()).await.unwrap();
        hub.register_agent("agent-3".to_string()).await.unwrap();

        let message = AgentMessage::Broadcast {
            sender: "orchestrator".to_string(),
            message: "Hello all!".to_string(),
        };

        hub.broadcast("orchestrator".to_string(), message)
            .await
            .unwrap();

        // All agents should receive the message
        assert!(hub.try_receive_message("agent-1").await.is_some());
        assert!(hub.try_receive_message("agent-2").await.is_some());
        assert!(hub.try_receive_message("agent-3").await.is_some());
    }

    #[tokio::test]
    async fn test_unregister() {
        let hub = CommunicationHub::new();

        hub.register_agent("agent-1".to_string()).await.unwrap();
        assert_eq!(hub.agent_count().await, 1);

        hub.unregister_agent("agent-1").await.unwrap();
        assert_eq!(hub.agent_count().await, 0);
        assert!(!hub.is_registered("agent-1").await);
    }
}
