/// EZSP v8 command ID constants and typed request/response helpers.
///
/// Sources: Silicon Labs UG100, AN0042, EZSP Reference Guide (EmberZNet 7.x).

// ── System / Configuration ────────────────────────────────────────────────────

/// Get the version of the NCP firmware and EZSP protocol.
pub const VERSION: u16 = 0x0000;
/// Get/set a configuration value.
pub const GET_CONFIG_VALUE: u16 = 0x0052;
pub const SET_CONFIG_VALUE: u16 = 0x0053;
/// Get/set a policy.
pub const GET_POLICY: u16 = 0x0056;
pub const SET_POLICY: u16 = 0x0055;
/// Get/set a value (extended config).
pub const GET_VALUE: u16 = 0x00AA;
pub const SET_VALUE: u16 = 0x00AB;

// ── Network ───────────────────────────────────────────────────────────────────

pub const FORM_NETWORK: u16 = 0x001E;
pub const JOIN_NETWORK: u16 = 0x001F;
pub const LEAVE_NETWORK: u16 = 0x0020;
pub const PERMIT_JOINING: u16 = 0x0022;
pub const GET_NETWORK_PARAMETERS: u16 = 0x0028;
pub const NETWORK_STATE: u16 = 0x0018;
/// Stack status callback (NCP→host).
pub const STACK_STATUS_HANDLER: u16 = 0x0019;

// ── Node identity ─────────────────────────────────────────────────────────────

/// Read this node's EUI-64 (IEEE address).
pub const GET_EUI64: u16 = 0x0026;
/// Read this node's 16-bit network address.
pub const GET_NODE_ID: u16 = 0x0027;

// ── Messaging ─────────────────────────────────────────────────────────────────

pub const SEND_UNICAST: u16 = 0x0034;
pub const SEND_BROADCAST: u16 = 0x0036;
pub const SEND_MULTICAST: u16 = 0x0038;
/// Message-sent status callback.
pub const MESSAGE_SENT_HANDLER: u16 = 0x003F;
/// Incoming message callback (NCP→host).
pub const INCOMING_MESSAGE_HANDLER: u16 = 0x0045;

// ── Trust Center / Security ───────────────────────────────────────────────────

pub const TRUST_CENTER_JOIN_HANDLER: u16 = 0x0024;
pub const SET_INITIAL_SECURITY_STATE: u16 = 0x0068;
pub const GET_CURRENT_SECURITY_STATE: u16 = 0x0069;
pub const GET_KEY: u16 = 0x006A;
pub const SET_KEY: u16 = 0x00A9;

// ── Neighbor / device management ─────────────────────────────────────────────

pub const GET_NEIGHBOR: u16 = 0x0079;
pub const NEIGHBOR_COUNT: u16 = 0x007A;
pub const GET_ROUTE_TABLE_ENTRY: u16 = 0x007B;
pub const ADDRESS_TABLE_ENTRY: u16 = 0x0077;
pub const GET_ADDRESS_TABLE_REMOTE_EUI64: u16 = 0x004E;
pub const GET_ADDRESS_TABLE_REMOTE_NODE_ID: u16 = 0x004F;

// ── EZSP status codes ─────────────────────────────────────────────────────────

pub const STATUS_SUCCESS: u8 = 0x00;
pub const STATUS_ERR_FATAL: u8 = 0x01;
pub const STATUS_INVALID_FRAME_ID: u8 = 0x28;
pub const STATUS_VERSION_NOT_SUPPORTED: u8 = 0x31;

// ── Typed helpers ─────────────────────────────────────────────────────────────

/// Encode a PERMIT_JOINING command payload.
/// `duration`: 0 = disable, 0xFF = forever, 1–254 = seconds.
pub fn permit_joining_payload(duration: u8) -> Vec<u8> {
    vec![duration]
}

/// Encode a SEND_UNICAST payload header.
///
/// Full unicast frame:
/// `type(1) | indexOrDest(2) | apsFrame(11+) | msgTag(1) | msgLen(1) | msg(msgLen)`
pub fn send_unicast_payload(
    dest_nwk: u16,
    src_endpoint: u8,
    dst_endpoint: u8,
    cluster_id: u16,
    profile_id: u16,
    sequence: u8,
    msg_tag: u8,
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::new();
    // type = EMBER_OUTGOING_DIRECT = 0x00
    buf.push(0x00);
    // indexOrDest = destination NWK address
    buf.extend_from_slice(&dest_nwk.to_le_bytes());
    // APS frame: options(2) | profileId(2) | clusterId(2) | srcEp(1) | dstEp(1) | groupId(2) | seq(1)
    buf.extend_from_slice(&0x0000u16.to_le_bytes()); // options
    buf.extend_from_slice(&profile_id.to_le_bytes());
    buf.extend_from_slice(&cluster_id.to_le_bytes());
    buf.push(src_endpoint);
    buf.push(dst_endpoint);
    buf.extend_from_slice(&0x0000u16.to_le_bytes()); // groupId
    buf.push(sequence);
    // msgTag
    buf.push(msg_tag);
    // message length + content
    buf.push(payload.len() as u8);
    buf.extend_from_slice(payload);
    buf
}

/// Decode an INCOMING_MESSAGE_HANDLER callback payload.
/// Returns (message_type, aps_frame_cluster_id, src_nwk, src_endpoint, payload) or None.
pub fn decode_incoming_message(params: &[u8]) -> Option<(u8, u16, u16, u8, &[u8])> {
    if params.len() < 12 {
        return None;
    }
    let msg_type = params[0];
    // APS frame starts at byte 1: options(2)|profileId(2)|clusterId(2)|srcEp(1)|dstEp(1)|groupId(2)|seq(1)
    let cluster_id = u16::from_le_bytes([params[3], params[4]]);
    let src_endpoint = params[6];
    // srcNwkAddr at offset 11
    let src_nwk = u16::from_le_bytes([params[11], params[12]]);
    let msg_len = *params.get(13)? as usize;
    let msg_start = 14;
    let payload = params.get(msg_start..msg_start + msg_len)?;
    Some((msg_type, cluster_id, src_nwk, src_endpoint, payload))
}
