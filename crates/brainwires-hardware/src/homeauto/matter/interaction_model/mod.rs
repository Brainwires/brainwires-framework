/// Matter 1.3 Interaction Model (IM) — protocol ID 0x0001.
///
/// All IM messages are TLV-encoded in the decrypted payload of a `MatterMessage`.
/// The opcode byte (embedded as the first byte of the serialized payload) selects
/// the message type.  The sub-modules here implement TLV encode/decode for each
/// message body.
///
/// Reference: Matter spec §8 (Interaction Model).

pub mod invoke;
pub mod read;
pub mod subscribe;
pub mod write;

// ── Protocol constant ─────────────────────────────────────────────────────────

/// IM protocol identifier (used in the Matter exchange header).
pub const PROTOCOL_ID: u16 = 0x0001;

// ── Opcode enum ───────────────────────────────────────────────────────────────

/// IM protocol opcodes (Matter spec §8.10, Table 44).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImOpcode {
    StatusResponse    = 0x01,
    ReadRequest       = 0x02,
    SubscribeRequest  = 0x03,
    SubscribeResponse = 0x04,
    ReportData        = 0x05,
    WriteRequest      = 0x06,
    WriteResponse     = 0x07,
    InvokeRequest     = 0x08,
    InvokeResponse    = 0x09,
    TimedRequest      = 0x0A,
}

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use invoke::{InvokeRequest, InvokeResponse, InvokeResponseItem};
pub use read::{AttributeData, ReadRequest, ReportData};
pub use subscribe::{SubscribeRequest, SubscribeResponse};
pub use write::{AttributeStatus, InteractionStatus, WriteRequest, WriteResponse};
