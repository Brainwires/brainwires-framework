use thiserror::Error;

/// Unified error type for all home automation protocol operations.
#[derive(Debug, Error)]
pub enum HomeAutoError {
    // ── Serial / transport ──────────────────────────────────────────────────
    #[cfg(any(feature = "zigbee", feature = "zwave"))]
    #[error("serial port error: {0}")]
    Serial(#[from] tokio_serial::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("connection timed out")]
    Timeout,

    #[error("serial frame error: {0}")]
    FrameError(String),

    // ── Zigbee ──────────────────────────────────────────────────────────────
    #[error("Zigbee coordinator error: {0}")]
    ZigbeeCoordinator(String),

    #[error("Zigbee device not found: {addr:016x}")]
    ZigbeeDeviceNotFound { addr: u64 },

    #[error("Zigbee attribute error (cluster {cluster:#06x} attr {attr:#06x}): {msg}")]
    ZigbeeAttribute {
        cluster: u16,
        attr: u16,
        msg: String,
    },

    #[error("Zigbee EZSP error (status {status:#04x}): {msg}")]
    EzspStatus { status: u8, msg: String },

    #[error("Zigbee ZNP error (status {status:#04x}): {msg}")]
    ZnpStatus { status: u8, msg: String },

    // ── Z-Wave ───────────────────────────────────────────────────────────────
    #[error("Z-Wave controller error: {0}")]
    ZWaveController(String),

    #[error("Z-Wave node {node_id} not found")]
    ZWaveNodeNotFound { node_id: u8 },

    #[error("Z-Wave transmission failed (node {node_id}): {msg}")]
    ZWaveTransmit { node_id: u8, msg: String },

    #[error("Z-Wave NAK received after {retries} retries")]
    ZWaveNak { retries: u8 },

    // ── Thread ───────────────────────────────────────────────────────────────
    #[error("Thread border router HTTP error: {0}")]
    ThreadHttp(String),

    #[error("Thread border router response parse error: {0}")]
    ThreadParse(String),

    // ── Matter ───────────────────────────────────────────────────────────────
    #[error("Matter error: {0}")]
    Matter(String),

    #[error("Matter commissioning error: {0}")]
    MatterCommissioning(String),

    #[error("Matter cluster invoke error (cluster {cluster:#010x} cmd {cmd:#010x}): {msg}")]
    MatterCluster { cluster: u32, cmd: u32, msg: String },

    // ── General ──────────────────────────────────────────────────────────────
    #[error("not supported: {0}")]
    Unsupported(String),

    #[error("channel closed")]
    ChannelClosed,
}

/// Convenience alias.
pub type HomeAutoResult<T> = Result<T, HomeAutoError>;
