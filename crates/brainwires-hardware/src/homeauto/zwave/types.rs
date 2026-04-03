use serde::{Deserialize, Serialize};
use super::super::types::Capability;

/// Z-Wave node ID (1–232, 0 = invalid).
pub type NodeId = u8;

/// Z-Wave device type classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZWaveNodeKind {
    Switch,
    DimmableSwitch,
    MultiLevelSwitch,
    BinarySensor,
    MultiLevelSensor,
    Thermostat,
    DoorLock,
    Siren,
    PowerStrip,
    EnergyMeter,
    Unknown,
}

/// A Z-Wave node on the network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZWaveNode {
    pub node_id: NodeId,
    pub name: Option<String>,
    pub manufacturer_id: u16,
    pub product_type: u16,
    pub product_id: u16,
    pub kind: ZWaveNodeKind,
    pub capabilities: Vec<Capability>,
    pub command_classes: Vec<u8>,
    pub is_listening: bool,
    pub online: bool,
}

impl ZWaveNode {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            name: None,
            manufacturer_id: 0,
            product_type: 0,
            product_id: 0,
            kind: ZWaveNodeKind::Unknown,
            capabilities: Vec::new(),
            command_classes: Vec::new(),
            is_listening: false,
            online: false,
        }
    }
}

/// Z-Wave Z/IP or Serial API frame type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Request = 0x00,
    Response = 0x01,
}
