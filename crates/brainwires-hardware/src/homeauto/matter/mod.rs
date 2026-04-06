/// BLE commissioning peripheral — BTP handshake and transport channels.
#[cfg(feature = "matter-ble")]
pub mod ble;
/// Typed cluster helpers (TLV-encoded command and attribute payloads).
pub mod clusters;
/// Matter commissioning payload parser (QR code + manual pairing code).
pub mod commissioning;
/// Matter controller — commissions and controls Matter devices.
pub mod controller;
/// Matter 1.3 cryptographic stack: KDF helpers and SPAKE2+ PAKE.
pub mod crypto;
/// Matter data model — cluster servers, ACL, and node dispatch.
pub mod data_model;
/// Matter device discovery — commissionable and operational DNS-SD.
pub mod discovery;
/// Typed errors wrapping rs-matter.
pub mod error;
/// Matter fabric management — root CA, NOC issuance, and fabric storage.
pub mod fabric;
/// Matter Interaction Model — read, write, invoke, and subscribe messages.
pub mod interaction_model;
/// Matter secure channel — PASE (commissioning) and CASE (operational) session establishment.
pub mod secure_channel;
/// Matter device server — exposes agents as Matter devices.
pub mod server;
/// Matter transport layer: message framing, MRP, and UDP/BLE I/O.
pub mod transport;
/// Matter device types, cluster IDs, and configuration.
pub mod types;

pub use commissioning::{CommissioningPayload, parse_manual_code, parse_qr_code};
pub use controller::MatterController;
pub use crypto::{
    kdf::{derive_passcode_verifier, hkdf_expand_label},
    spake2plus::{Spake2PlusKeys, Spake2PlusProver, Spake2PlusVerifier},
};
pub use error::{MatterError, MatterResult};
pub use fabric::{FabricDescriptor, FabricIndex, MatterCert};
pub use interaction_model::{
    AttributeData, AttributeStatus, ImOpcode, InteractionStatus, InvokeRequest, InvokeResponse,
    InvokeResponseItem, PROTOCOL_ID as IM_PROTOCOL_ID, ReadRequest, ReportData, SubscribeRequest,
    SubscribeResponse, WriteRequest, WriteResponse,
};
pub use secure_channel::{
    CaseInitiator, CaseResponder, EstablishedSession, PaseCommissionee, PaseCommissioner,
    SECURE_CHANNEL_PROTOCOL_ID, SecureChannelOpcode,
};
pub use server::{MatterDeviceServer, OnOffHandler};
pub use types::{
    MatterDevice, MatterDeviceConfig, MatterDeviceConfigBuilder, MatterEndpoint, cluster_id,
    device_type,
};
