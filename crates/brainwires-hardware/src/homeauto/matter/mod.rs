// ─────────────────────────────────────────────────────────────────────────────
// ⚠️  EXPERIMENTAL — This Matter implementation is a development preview.
//
// What works:
//   • PASE (SPAKE2+ Password Authenticated Session Establishment) over UDP
//   • CASE (Certificate Authenticated Session Establishment) — Sigma 1/2/3 with
//     real P-256 ECDH, ECDSA signing, HKDF session keys, AES-128-CCM framing
//   • Fabric management + on-disk persistence (fabrics.json via tokio::fs)
//   • mDNS commissionable-device discovery
//   • Basic Interaction Model: read/write attributes, invoke commands
//   • Manual pairing code + QR code parsing (Verhoeff check digit validated)
//   • BLE commissioning transport (btleplug, Linux/macOS) behind `matter-ble`
//
// Known limitations:
//   • Device Attestation Key (DAK) is stubbed — AttestationResponse signatures
//     are zeroed, so real Matter commissioners reject this device. Provisioning
//     hook is in progress.
//   • Subscribe/ReportData only encodes/decodes TLV — there is no active
//     subscription registry, so attribute mutations do not propagate to
//     subscribers.
//   • Commissioning orchestration (BLE → PASE → AddNOC → CASE state machine)
//     is not yet wired; QR/manual code parsing and each handshake work in
//     isolation but not as an end-to-end chain.
//   • Not tested against real certified Matter controllers.
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
/// State machine for the commissioner-driven commissioning flow.
pub mod commissioning_session;
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
/// Subscription registry for Interaction Model Subscribe/Report.
pub mod subscription_manager;
/// Verhoeff check-digit algorithm used by the 11-digit manual pairing code.
pub mod verhoeff;

pub use commissioning::{CommissioningPayload, parse_manual_code, parse_qr_code};
pub use commissioning_session::{CommissioningEvent, CommissioningSession, Phase};
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
pub use subscription_manager::{Subscription, SubscriptionManager};
pub use types::{
    MatterDevice, MatterDeviceConfig, MatterDeviceConfigBuilder, MatterEndpoint, cluster_id,
    device_type,
};
