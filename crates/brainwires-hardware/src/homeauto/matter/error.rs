use thiserror::Error;

/// Matter-specific errors.
///
/// Note: We implement the Matter protocol ourselves (TLV + commissioning + UDP transport)
/// rather than using rs-matter, to avoid an `embassy-time` links conflict with the
/// burn ML ecosystem in the workspace.
#[derive(Debug, Error)]
pub enum MatterError {
    #[error("commissioning failed: {0}")]
    Commissioning(String),

    #[error("QR code parse error: {0}")]
    QrCode(&'static str),

    #[error("cluster invoke error (cluster {cluster:#010x} cmd {cmd:#010x}): {msg}")]
    ClusterInvoke { cluster: u32, cmd: u32, msg: String },

    #[error("attribute read error (cluster {cluster:#010x} attr {attr:#010x}): {msg}")]
    AttributeRead { cluster: u32, attr: u32, msg: String },

    #[error("device not found: node_id={node_id}")]
    DeviceNotFound { node_id: u64 },

    #[error("transport error: {0}")]
    Transport(String),

    #[error("mDNS error: {0}")]
    Mdns(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("SPAKE2+ error: {0}")]
    Spake2(String),

    #[error("session {session_id} error: {msg}")]
    Session { session_id: u16, msg: String },

    #[error("protocol error (opcode {opcode:#04x}): {msg}")]
    Protocol { opcode: u8, msg: String },

    #[error("access denied")]
    AccessDenied,

    #[error("fabric not found")]
    FabricNotFound,
}

pub type MatterResult<T> = Result<T, MatterError>;
