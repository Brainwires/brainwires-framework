use serde::{Deserialize, Serialize};
use super::super::types::Capability;

/// A commissioned Matter device on the fabric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatterDevice {
    /// 64-bit node ID assigned during commissioning.
    pub node_id: u64,
    /// Fabric index (0-based).
    pub fabric_index: u8,
    /// Human-readable name (optional, user-assigned).
    pub name: Option<String>,
    /// Vendor ID.
    pub vendor_id: u16,
    /// Product ID.
    pub product_id: u16,
    /// List of endpoints exposed by this device.
    pub endpoints: Vec<MatterEndpoint>,
    /// Whether the device is currently reachable.
    pub online: bool,
}

impl MatterDevice {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            fabric_index: 0,
            name: None,
            vendor_id: 0,
            product_id: 0,
            endpoints: Vec::new(),
            online: false,
        }
    }
}

/// A Matter endpoint (logical device within a node).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatterEndpoint {
    pub endpoint_id: u16,
    /// Device type ID from the Matter spec (e.g. 0x0100 = On/Off Light).
    pub device_type: u32,
    /// Cluster IDs supported by this endpoint (server-side).
    pub clusters: Vec<u32>,
    pub capabilities: Vec<Capability>,
}

/// Configuration for a [`MatterDeviceServer`] instance.
///
/// Use [`MatterDeviceConfig::builder`] for ergonomic construction.
#[derive(Debug, Clone)]
pub struct MatterDeviceConfig {
    /// Device name as advertised over mDNS and in the Basic Information cluster.
    pub device_name: String,
    /// Vendor ID (0xFFF1 = test/development).
    pub vendor_id: u16,
    /// Product ID.
    pub product_id: u16,
    /// 12-bit discriminator (0–4095) used to identify the device during commissioning.
    pub discriminator: u16,
    /// SPAKE2+ commissioning passcode (PIN). Must not be a forbidden value.
    pub passcode: u32,
    /// Path to store persistent fabric data (certificates, node IDs, etc.).
    pub storage_path: std::path::PathBuf,
    /// UDP port to listen on (default: 5540, the standard Matter port).
    pub port: u16,
}

impl MatterDeviceConfig {
    pub fn builder() -> MatterDeviceConfigBuilder {
        MatterDeviceConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct MatterDeviceConfigBuilder {
    device_name: Option<String>,
    vendor_id: Option<u16>,
    product_id: Option<u16>,
    discriminator: Option<u16>,
    passcode: Option<u32>,
    storage_path: Option<std::path::PathBuf>,
    port: Option<u16>,
}

impl MatterDeviceConfigBuilder {
    pub fn device_name(mut self, name: impl Into<String>) -> Self {
        self.device_name = Some(name.into());
        self
    }
    pub fn vendor_id(mut self, vid: u16) -> Self {
        self.vendor_id = Some(vid);
        self
    }
    pub fn product_id(mut self, pid: u16) -> Self {
        self.product_id = Some(pid);
        self
    }
    pub fn discriminator(mut self, d: u16) -> Self {
        self.discriminator = Some(d & 0x0FFF);
        self
    }
    pub fn passcode(mut self, p: u32) -> Self {
        self.passcode = Some(p);
        self
    }
    pub fn storage_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.storage_path = Some(path.into());
        self
    }
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    pub fn build(self) -> MatterDeviceConfig {
        MatterDeviceConfig {
            device_name: self.device_name.unwrap_or_else(|| "Brainwires Device".into()),
            vendor_id: self.vendor_id.unwrap_or(0xFFF1), // test VID
            product_id: self.product_id.unwrap_or(0x8001),
            discriminator: self.discriminator.unwrap_or(3840),
            passcode: self.passcode.unwrap_or(20202021),
            storage_path: self
                .storage_path
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp/brainwires-matter")),
            port: self.port.unwrap_or(5540),
        }
    }
}

// ── Well-known Matter device type IDs (Matter 1.3) ────────────────────────────

pub mod device_type {
    pub const ON_OFF_LIGHT: u32 = 0x0100;
    pub const DIMMABLE_LIGHT: u32 = 0x0101;
    pub const COLOR_TEMPERATURE_LIGHT: u32 = 0x010C;
    pub const EXTENDED_COLOR_LIGHT: u32 = 0x010D;
    pub const ON_OFF_PLUG: u32 = 0x010A;
    pub const DIMMABLE_PLUG: u32 = 0x010B;
    pub const PUMP: u32 = 0x0303;
    pub const THERMOSTAT: u32 = 0x0301;
    pub const FAN: u32 = 0x002B;
    pub const WINDOW_COVERING: u32 = 0x0202;
    pub const DOOR_LOCK: u32 = 0x000A;
    pub const OCCUPANCY_SENSOR: u32 = 0x0107;
    pub const TEMPERATURE_SENSOR: u32 = 0x0302;
    pub const HUMIDITY_SENSOR: u32 = 0x0307;
    pub const LIGHT_SENSOR: u32 = 0x0106;
    pub const CONTACT_SENSOR: u32 = 0x0015;
    pub const FLOW_SENSOR: u32 = 0x0306;
    pub const PRESSURE_SENSOR: u32 = 0x0305;
    pub const EV_CHARGER: u32 = 0x050C; // Matter 1.3
}

// ── Well-known Matter cluster IDs (Matter 1.3) ────────────────────────────────

pub mod cluster_id {
    // Foundation
    pub const BASIC_INFORMATION: u32 = 0x0028;
    pub const OTA_SOFTWARE_UPDATE: u32 = 0x0029;
    pub const GENERAL_COMMISSIONING: u32 = 0x0030;
    pub const NETWORK_COMMISSIONING: u32 = 0x0031;
    pub const DIAGNOSTIC_LOGS: u32 = 0x0032;
    pub const GENERAL_DIAGNOSTICS: u32 = 0x0033;
    pub const OPERATIONAL_CREDENTIALS: u32 = 0x003E;
    pub const NODE_OPERATIONAL_CREDENTIALS: u32 = 0x003E;
    pub const FIXED_LABEL: u32 = 0x0040;
    // Device capabilities
    pub const IDENTIFY: u32 = 0x0003;
    pub const GROUPS: u32 = 0x0004;
    pub const SCENES: u32 = 0x0005;
    pub const ON_OFF: u32 = 0x0006;
    pub const LEVEL_CONTROL: u32 = 0x0008;
    pub const DESCRIPTOR: u32 = 0x001D;
    pub const BINDING: u32 = 0x001E;
    // Color
    pub const COLOR_CONTROL: u32 = 0x0300;
    // Window covering
    pub const WINDOW_COVERING: u32 = 0x0102;
    // HVAC
    pub const THERMOSTAT: u32 = 0x0201;
    pub const THERMOSTAT_UI_CONFIG: u32 = 0x0204;
    pub const FAN_CONTROL: u32 = 0x0202;
    // Security
    pub const DOOR_LOCK: u32 = 0x0101;
    // Sensors
    pub const TEMPERATURE_MEASUREMENT: u32 = 0x0402;
    pub const RELATIVE_HUMIDITY: u32 = 0x0405;
    pub const OCCUPANCY_SENSING: u32 = 0x0406;
    pub const ILLUMINANCE_MEASUREMENT: u32 = 0x0400;
    pub const PRESSURE_MEASUREMENT: u32 = 0x0403;
    pub const FLOW_MEASUREMENT: u32 = 0x0404;
    // Energy (Matter 1.3)
    pub const ELECTRICAL_MEASUREMENT: u32 = 0x0B04;
    pub const POWER_SOURCE: u32 = 0x002F;
    pub const EV_CHARGING: u32 = 0x0099; // Matter 1.3
}
