/// ZNP command constants for TI Z-Stack 3.x.
///
/// Subsystem IDs and command bytes as defined in the TI Z-Stack Monitor and Test API (Z-Stack 3.x).

// ── Subsystem IDs ─────────────────────────────────────────────────────────────

pub const SYS: u8 = 0x21; // System subsystem
pub const MAC: u8 = 0x22; // MAC layer
pub const NWK: u8 = 0x26; // Network layer
pub const AF: u8 = 0x24; // Application Framework
pub const ZDO: u8 = 0x25; // Zigbee Device Object
pub const SAPI: u8 = 0x2F; // Simple API
pub const UTIL: u8 = 0x27; // Utilities
pub const APP_CNF: u8 = 0x26; // App config (overlaps NWK per Z-Stack 3.x)
pub const APP: u8 = 0x29; // Application layer

// ── SYS subsystem commands ────────────────────────────────────────────────────

pub const SYS_RESET_REQ: u8 = 0x09; // Reset NCP (SREQ)
pub const SYS_RESET_IND: u8 = 0x80; // Reset indication (AREQ)
pub const SYS_VERSION: u8 = 0x02; // Get firmware version
pub const SYS_PING: u8 = 0x01; // Ping (connectivity check)
pub const SYS_OSAL_NV_READ: u8 = 0x08; // Read NV item
pub const SYS_OSAL_NV_WRITE: u8 = 0x09; // Write NV item (non-RESET cmd slot)
pub const SYS_GET_EXTADDR: u8 = 0x04; // Get IEEE address

// ── ZDO subsystem commands ────────────────────────────────────────────────────

pub const ZDO_STARTUP_FROM_APP: u8 = 0x40; // Start Zigbee stack (SREQ)
pub const ZDO_NODE_DESC_REQ: u8 = 0x02;
pub const ZDO_ACTIVE_EP_REQ: u8 = 0x05;
pub const ZDO_SIMPLE_DESC_REQ: u8 = 0x04;
pub const ZDO_END_DEVICE_ANNCE_IND: u8 = 0xFF; // New device joined (AREQ)
pub const ZDO_TC_DEV_IND: u8 = 0xCA; // Trust center device indication (AREQ)
pub const ZDO_PERMIT_JOIN_REQ: u8 = 0x36;
pub const ZDO_PERMIT_JOIN_IND: u8 = 0xCB; // Permit join status (AREQ)
pub const ZDO_NWK_ADDR_RSP: u8 = 0x80;
pub const ZDO_IEEE_ADDR_RSP: u8 = 0x81;
pub const ZDO_STATE_CHANGE_IND: u8 = 0xC0; // Network state changed (AREQ)
pub const ZDO_LEAVE_IND: u8 = 0xC9; // Device left (AREQ)

// ── AF subsystem commands ─────────────────────────────────────────────────────

pub const AF_REGISTER: u8 = 0x00; // Register an endpoint
pub const AF_DATA_REQUEST: u8 = 0x01; // Send data (SREQ)
pub const AF_DATA_CONFIRM: u8 = 0x05; // Data confirm (AREQ)
pub const AF_INCOMING_MSG: u8 = 0x81; // Incoming message (AREQ)
pub const AF_DATA_REQUEST_EXT: u8 = 0x02; // Extended data request

// ── APP_CNF subsystem commands ────────────────────────────────────────────────

pub const APP_CNF_BDB_START_COMMISSIONING: u8 = 0x00; // Start BDB commissioning
pub const APP_CNF_BDB_SET_CHANNEL: u8 = 0x08; // Set Zigbee channel mask
pub const APP_CNF_BDB_COMMISSIONING_NOTIFICATION: u8 = 0x80; // AREQ: commissioning done

// ── ZDO network state values ─────────────────────────────────────────────────

pub const DEV_COORDINATOR: u8 = 0x09;
pub const DEV_ROUTER: u8 = 0x08;
pub const DEV_END_DEVICE: u8 = 0x07;
pub const DEV_HOLD: u8 = 0x00;
pub const DEV_INIT: u8 = 0x01;

// ── Status codes ─────────────────────────────────────────────────────────────

pub const ZNP_STATUS_SUCCESS: u8 = 0x00;
pub const ZNP_STATUS_FAILED: u8 = 0x01;
pub const ZNP_STATUS_INVALID_PARAM: u8 = 0x02;

// ── Typed payload helpers ─────────────────────────────────────────────────────

/// Build the payload for ZDO_STARTUP_FROM_APP (start-delay in ms).
pub fn startup_payload(start_delay_ms: u16) -> Vec<u8> {
    start_delay_ms.to_le_bytes().to_vec()
}

/// Build the payload for AF_DATA_REQUEST (send a ZCL message).
///
/// Layout: dstAddr(2) | dstEndpoint(1) | srcEndpoint(1) | clusterId(2) |
///         transId(1) | options(1) | radius(1) | len(1) | data(len)
pub fn af_data_request(
    dst_nwk: u16,
    dst_ep: u8,
    src_ep: u8,
    cluster_id: u16,
    trans_id: u8,
    data: &[u8],
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&dst_nwk.to_le_bytes());
    buf.push(dst_ep);
    buf.push(src_ep);
    buf.extend_from_slice(&cluster_id.to_le_bytes());
    buf.push(trans_id);
    buf.push(0x00); // options: none
    buf.push(0x0F); // radius: 15 hops
    buf.push(data.len() as u8);
    buf.extend_from_slice(data);
    buf
}

/// Build the payload for ZDO_PERMIT_JOIN_REQ.
/// `dest` = 0xFFFC (all routers + coordinator), duration = 0–254 s or 0xFF (forever).
pub fn permit_join_payload(dest: u16, duration: u8) -> Vec<u8> {
    let mut buf = dest.to_le_bytes().to_vec();
    buf.push(duration);
    buf.push(0); // TCSignificance
    buf
}

/// Decode an AF_INCOMING_MSG AREQ payload.
/// Returns (groupId, clusterId, srcAddr, srcEp, dstEp, transId, payload) or None.
pub fn decode_af_incoming(params: &[u8]) -> Option<(u16, u16, u16, u8, u8, u8, &[u8])> {
    if params.len() < 11 {
        return None;
    }
    let group_id = u16::from_le_bytes([params[0], params[1]]);
    let cluster_id = u16::from_le_bytes([params[2], params[3]]);
    let src_addr = u16::from_le_bytes([params[4], params[5]]);
    let src_ep = params[6];
    let dst_ep = params[7];
    let trans_id = params[8];
    // params[9] = broadcast radius, params[10] = link quality
    let data_len = *params.get(11)? as usize;
    let data = params.get(12..12 + data_len)?;
    Some((
        group_id, cluster_id, src_addr, src_ep, dst_ep, trans_id, data,
    ))
}
