use serde::{Deserialize, Serialize};

/// Which protocol a device speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    Zigbee,
    ZWave,
    Thread,
    Matter,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Zigbee => write!(f, "Zigbee"),
            Protocol::ZWave => write!(f, "Z-Wave"),
            Protocol::Thread => write!(f, "Thread"),
            Protocol::Matter => write!(f, "Matter"),
        }
    }
}

/// High-level capability that a home device exposes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    OnOff,
    Dimming,
    ColorTemperature,
    ColorRgb,
    Temperature,
    Humidity,
    Pressure,
    Motion,
    Contact,
    Lock,
    Thermostat,
    EnergyMonitoring,
    WindowCovering,
    Custom(String),
}

/// A unified home automation device record (protocol-agnostic view).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeDevice {
    /// Protocol-specific unique identifier (IEEE address, node ID, etc.).
    pub id: String,
    pub name: Option<String>,
    pub protocol: Protocol,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub firmware_version: Option<String>,
    pub capabilities: Vec<Capability>,
}

impl HomeDevice {
    pub fn new(id: impl Into<String>, protocol: Protocol) -> Self {
        Self {
            id: id.into(),
            name: None,
            protocol,
            manufacturer: None,
            model: None,
            firmware_version: None,
            capabilities: Vec::new(),
        }
    }
}

/// Typed value returned from an attribute read or carried in an event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeValue {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Null,
}

impl std::fmt::Display for AttributeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeValue::Bool(v) => write!(f, "{v}"),
            AttributeValue::U8(v) => write!(f, "{v}"),
            AttributeValue::U16(v) => write!(f, "{v}"),
            AttributeValue::U32(v) => write!(f, "{v}"),
            AttributeValue::U64(v) => write!(f, "{v}"),
            AttributeValue::I8(v) => write!(f, "{v}"),
            AttributeValue::I16(v) => write!(f, "{v}"),
            AttributeValue::I32(v) => write!(f, "{v}"),
            AttributeValue::F32(v) => write!(f, "{v}"),
            AttributeValue::F64(v) => write!(f, "{v}"),
            AttributeValue::String(v) => write!(f, "{v}"),
            AttributeValue::Bytes(v) => write!(f, "0x{}", hex_encode(v)),
            AttributeValue::Null => write!(f, "null"),
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Events emitted by any home automation hub/coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HomeAutoEvent {
    /// A new device joined the network.
    DeviceJoined(HomeDevice),

    /// A device left or was removed from the network.
    DeviceLeft { id: String, protocol: Protocol },

    /// An attribute value changed (e.g. temperature sensor update, switch toggled).
    AttributeChanged {
        device_id: String,
        protocol: Protocol,
        /// Human-readable cluster name or hex ID.
        cluster: String,
        /// Human-readable attribute name or hex ID.
        attribute: String,
        value: AttributeValue,
    },

    /// A command was successfully sent to a device.
    CommandSent {
        device_id: String,
        protocol: Protocol,
        cluster: String,
        command: String,
    },
}
