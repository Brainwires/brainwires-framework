use serde::{Deserialize, Serialize};
use super::super::types::Capability;

/// 64-bit IEEE (EUI-64) extended address.
pub type IeeeAddr = u64;
/// 16-bit network (short) address.
pub type NwkAddr = u16;

/// A Zigbee device address — may be addressed by either form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ZigbeeAddr {
    pub ieee: IeeeAddr,
    pub nwk: NwkAddr,
}

impl ZigbeeAddr {
    pub fn new(ieee: IeeeAddr, nwk: NwkAddr) -> Self {
        Self { ieee, nwk }
    }
}

impl std::fmt::Display for ZigbeeAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016x} ({:#06x})", self.ieee, self.nwk)
    }
}

/// Standard Zigbee cluster IDs (ZCL Foundation, Zigbee 3.0).
#[allow(non_upper_case_globals)]
pub mod cluster_id {
    pub const BASIC: u16 = 0x0000;
    pub const POWER_CONFIG: u16 = 0x0001;
    pub const IDENTIFY: u16 = 0x0003;
    pub const GROUPS: u16 = 0x0004;
    pub const SCENES: u16 = 0x0005;
    pub const ON_OFF: u16 = 0x0006;
    pub const ON_OFF_SWITCH_CONFIG: u16 = 0x0007;
    pub const LEVEL_CONTROL: u16 = 0x0008;
    pub const ALARMS: u16 = 0x0009;
    pub const TIME: u16 = 0x000A;
    pub const OTA_UPGRADE: u16 = 0x0019;
    pub const DOOR_LOCK: u16 = 0x0101;
    pub const WINDOW_COVERING: u16 = 0x0102;
    pub const COLOR_CONTROL: u16 = 0x0300;
    pub const ILLUMINANCE: u16 = 0x0400;
    pub const TEMPERATURE: u16 = 0x0402;
    pub const HUMIDITY: u16 = 0x0405;
    pub const OCCUPANCY: u16 = 0x0406;
    pub const IAS_ZONE: u16 = 0x0500;
    pub const METERING: u16 = 0x0702;
    pub const ELECTRICAL_MEASUREMENT: u16 = 0x0B04;
}

/// Newtype for Zigbee cluster IDs.
pub type ZigbeeClusterId = u16;
/// Newtype for Zigbee attribute IDs.
pub type ZigbeeAttrId = u16;

/// Device kind inferred from ZDO Basic cluster `deviceType` field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZigbeeDeviceKind {
    Light,
    DimmableLight,
    ColorLight,
    Switch,
    TemperatureSensor,
    HumiditySensor,
    OccupancySensor,
    DoorLock,
    Thermostat,
    PowerOutlet,
    Other(u16),
}

/// A Zigbee end-device or router on the coordinator's network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZigbeeDevice {
    pub addr: ZigbeeAddr,
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub kind: ZigbeeDeviceKind,
    /// List of cluster IDs the device supports (server side).
    pub clusters: Vec<ZigbeeClusterId>,
    pub capabilities: Vec<Capability>,
    /// Whether the device is currently online/reachable.
    pub online: bool,
}

impl ZigbeeDevice {
    pub fn new(addr: ZigbeeAddr, kind: ZigbeeDeviceKind) -> Self {
        Self {
            addr,
            name: None,
            manufacturer: None,
            model: None,
            kind,
            clusters: Vec::new(),
            capabilities: Vec::new(),
            online: true,
        }
    }
}
