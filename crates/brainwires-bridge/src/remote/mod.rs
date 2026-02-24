pub mod attachments;
pub mod bridge;
pub mod command_queue;
pub mod heartbeat;
pub mod manager;
pub mod protocol;
pub mod realtime;
pub mod telemetry;

pub use command_queue::{CommandQueue, QueueEntry, QueueError, QueueStats};
pub use protocol::{
    AgentEventType, BackendCommand, CommandPriority, CompressionAlgorithm, NegotiatedProtocol,
    PrioritizedCommand, ProtocolAccept, ProtocolCapability, ProtocolHello, RemoteAgentInfo,
    RemoteMessage, RetryPolicy, StreamChunkType,
};
pub use telemetry::{ConnectionQuality, MetricsSnapshot, ProtocolMetrics};

pub use attachments::AttachmentReceiver;
pub use bridge::{BridgeConfig, BridgeState, ConnectionMode, RealtimeCredentials, RemoteBridge};
pub use heartbeat::{AgentEvent, HeartbeatCollector, HeartbeatData};
pub use manager::{RemoteBridgeManager, RemoteBridgeStatus};
pub use realtime::{RealtimeClient, RealtimeConfig, RealtimeState};
