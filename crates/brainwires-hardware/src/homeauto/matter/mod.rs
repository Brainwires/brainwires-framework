/// Matter commissioning payload parser (QR code + manual pairing code).
pub mod commissioning;
/// Typed cluster helpers (TLV-encoded command and attribute payloads).
pub mod clusters;
/// Typed errors wrapping rs-matter.
pub mod error;
/// Matter device types, cluster IDs, and configuration.
pub mod types;
/// Matter controller — commissions and controls Matter devices.
pub mod controller;
/// Matter device server — exposes agents as Matter devices.
pub mod server;

pub use commissioning::{parse_manual_code, parse_qr_code, CommissioningPayload};
pub use controller::MatterController;
pub use error::{MatterError, MatterResult};
pub use server::{MatterDeviceServer, OnOffHandler};
pub use types::{
    cluster_id, device_type, MatterDevice, MatterDeviceConfig, MatterDeviceConfigBuilder,
    MatterEndpoint,
};
