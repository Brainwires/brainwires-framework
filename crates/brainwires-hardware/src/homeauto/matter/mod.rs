/// Matter commissioning payload parser (QR code + manual pairing code).
pub mod commissioning;
/// Typed cluster helpers (TLV-encoded command and attribute payloads).
pub mod clusters;
/// Matter 1.3 cryptographic stack: KDF helpers and SPAKE2+ PAKE.
pub mod crypto;
/// Typed errors wrapping rs-matter.
pub mod error;
/// Matter Interaction Model — read, write, invoke, and subscribe messages.
pub mod interaction_model;
/// Matter device types, cluster IDs, and configuration.
pub mod types;
/// Matter controller — commissions and controls Matter devices.
pub mod controller;
/// Matter device server — exposes agents as Matter devices.
pub mod server;

pub use commissioning::{parse_manual_code, parse_qr_code, CommissioningPayload};
pub use controller::MatterController;
pub use crypto::{
    kdf::{derive_passcode_verifier, hkdf_expand_label},
    spake2plus::{Spake2PlusKeys, Spake2PlusProver, Spake2PlusVerifier},
};
pub use error::{MatterError, MatterResult};
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
