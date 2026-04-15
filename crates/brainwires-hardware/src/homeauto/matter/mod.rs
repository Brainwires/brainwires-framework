// ─────────────────────────────────────────────────────────────────────────────
// ⚠️  EXPERIMENTAL — This Matter implementation is a development preview.
//
// What works:
//   • PASE (Password Authenticated Session Establishment) over UDP
//   • mDNS commissionable-device discovery
//   • Basic Interaction Model: read/write attributes, invoke commands
//   • Manual pairing code and QR code parsing
//   • In-memory device tracking after PASE
//
// Known limitations:
//   • No CASE (Certificate Authenticated Session Establishment) — operational
//     sessions after commissioning cannot be established
//   • Commissioned devices are NOT persisted — state is lost on restart
//   • No fabric or operational credential management
//   • Pairing code check digit is hardcoded (Verhoeff not implemented)
//   • BLE transport is not wired (feature-gated but unimplemented)
//   • Event streaming returns an empty stream (stub)
//   • Not tested against real Matter controllers or certified devices
//
// Do not rely on this module for production home automation deployments.
// ─────────────────────────────────────────────────────────────────────────────

/// BLE commissioning peripheral — BTP handshake and transport channels.
#[cfg(feature = "matter-ble")]
pub mod ble;
/// Typed cluster helpers (TLV-encoded command and attribute payloads).
pub mod clusters;
/// Matter commissioning payload parser (QR code + manual pairing code).
pub mod commissioning;
/// Matter controller — commissions and controls Matter devices (PASE only, experimental).
pub mod controller;
/// Matter cryptographic stack: KDF helpers and SPAKE2+ PAKE.
pub mod crypto;
/// Matter data model — cluster servers, ACL, and node dispatch.
pub mod data_model;
/// Matter device discovery — commissionable and operational DNS-SD.
pub mod discovery;
/// Typed errors wrapping rs-matter.
pub mod error;
/// Matter fabric management — root CA, NOC issuance, and fabric storage (incomplete).
pub mod fabric;
/// Matter Interaction Model — read, write, invoke, and subscribe messages.
pub mod interaction_model;
/// Matter secure channel — PASE (commissioning) and CASE (operational, not yet functional).
pub mod secure_channel;
/// Matter device server — exposes agents as Matter devices (PASE only).
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
