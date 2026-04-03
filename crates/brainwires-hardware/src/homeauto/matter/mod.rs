/// Matter commissioning payload parser (QR code + manual pairing code).
pub mod commissioning;
/// Typed cluster helpers (TLV-encoded command and attribute payloads).
pub mod clusters;
/// Matter 1.3 cryptographic stack: KDF helpers and SPAKE2+ PAKE.
pub mod crypto;
/// Typed errors wrapping rs-matter.
pub mod error;
/// Matter fabric management — root CA, NOC issuance, and fabric storage.
pub mod fabric;
/// Matter Interaction Model — read, write, invoke, and subscribe messages.
pub mod interaction_model;
/// Matter secure channel — PASE (commissioning) and CASE (operational) session establishment.
pub mod secure_channel;
/// Matter device types, cluster IDs, and configuration.
pub mod types;
/// Matter controller — commissions and controls Matter devices.
pub mod controller;
/// Matter device discovery — commissionable and operational DNS-SD.
pub mod discovery;
/// Matter device server — exposes agents as Matter devices.
pub mod server;
/// Matter transport layer: message framing, MRP, and UDP/BLE I/O.
pub mod transport;
/// Matter data model — cluster servers, ACL, and node dispatch.
pub mod data_model;
/// BLE commissioning peripheral — BTP handshake and transport channels.
#[cfg(feature = "matter-ble")]
pub mod ble;

pub use commissioning::{parse_manual_code, parse_qr_code, CommissioningPayload};
pub use controller::MatterController;
pub use crypto::{
    kdf::{derive_passcode_verifier, hkdf_expand_label},
    spake2plus::{Spake2PlusKeys, Spake2PlusProver, Spake2PlusVerifier},
};
pub use error::{MatterError, MatterResult};
pub use fabric::{FabricDescriptor, FabricIndex, MatterCert};
pub use secure_channel::{
    CaseInitiator, CaseResponder, EstablishedSession, PaseCommissionee, PaseCommissioner,
    SecureChannelOpcode, SECURE_CHANNEL_PROTOCOL_ID,
};
pub use interaction_model::{
    ImOpcode, InvokeRequest, InvokeResponse, InvokeResponseItem,
    AttributeData, ReadRequest, ReportData,
    SubscribeRequest, SubscribeResponse,
    AttributeStatus, InteractionStatus, WriteRequest, WriteResponse,
    PROTOCOL_ID as IM_PROTOCOL_ID,
};
pub use server::{MatterDeviceServer, OnOffHandler};
pub use types::{
    cluster_id, device_type, MatterDevice, MatterDeviceConfig, MatterDeviceConfigBuilder,
    MatterEndpoint,
};
