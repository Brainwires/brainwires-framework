/// OperationalCredentials cluster server (cluster ID 0x003E).
///
/// Handles NOC management, CSR generation, and attestation during commissioning.
/// Matter spec §11.17.
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::homeauto::matter::clusters::tlv;
use crate::homeauto::matter::data_model::ClusterServer;
use crate::homeauto::matter::error::{MatterError, MatterResult};

// ── Attribute IDs ─────────────────────────────────────────────────────────────

pub const ATTR_NOCS: u32 = 0x0000;
pub const ATTR_FABRICS: u32 = 0x0001;
pub const ATTR_SUPPORTED_FABRICS: u32 = 0x0002;
pub const ATTR_COMMISSIONED_FABRICS: u32 = 0x0003;

// ── Command IDs ───────────────────────────────────────────────────────────────

pub const CMD_ATTESTATION_REQUEST: u32 = 0x00;
pub const CMD_CSR_REQUEST: u32 = 0x02;
pub const CMD_ADD_NOC: u32 = 0x06;
pub const CMD_UPDATE_FABRIC_LABEL: u32 = 0x0B;
pub const CMD_REMOVE_FABRIC: u32 = 0x0C;

const CLUSTER_ID: u32 = 0x003E;

// ── TLV encoding helpers (local) ──────────────────────────────────────────────

fn tlv_uint8(tag: u8, val: u8) -> Vec<u8> {
    vec![tlv::TAG_CONTEXT_1 | tlv::TYPE_UNSIGNED_INT_1, tag, val]
}

fn tlv_octet_string(tag: u8, data: &[u8]) -> Vec<u8> {
    // TYPE_OCTET_STRING_1 = 0x10 (1-byte length)
    let mut v = vec![tlv::TAG_CONTEXT_1 | 0x10, tag, data.len() as u8];
    v.extend_from_slice(data);
    v
}

fn tlv_uint32(tag: u8, val: u32) -> Vec<u8> {
    let mut v = vec![tlv::TAG_CONTEXT_1 | tlv::TYPE_UNSIGNED_INT_4, tag];
    v.extend_from_slice(&val.to_le_bytes());
    v
}

fn wrap_struct(inner: &[u8]) -> Vec<u8> {
    let mut v = vec![tlv::TYPE_STRUCTURE];
    v.extend_from_slice(inner);
    v.push(tlv::TYPE_END_OF_CONTAINER);
    v
}

fn wrap_list(inner: &[u8]) -> Vec<u8> {
    let mut v = vec![tlv::TYPE_LIST];
    v.extend_from_slice(inner);
    v.push(tlv::TYPE_END_OF_CONTAINER);
    v
}

/// Build a NOCResponse: `struct { StatusCode(0): uint8, FabricIndex(1): uint8 }`
fn noc_response(status_code: u8, fabric_index: u8) -> Vec<u8> {
    let mut inner = tlv_uint8(0, status_code);
    inner.extend_from_slice(&tlv_uint8(1, fabric_index));
    wrap_struct(&inner)
}

// ── State ─────────────────────────────────────────────────────────────────────

/// Stored NOC entry: the raw NOC bytes and optional ICAC.
#[derive(Debug, Clone)]
pub struct NocEntry {
    pub noc: Vec<u8>,
    pub icac: Option<Vec<u8>>,
    pub fabric_index: u8,
    pub label: String,
}

/// Mutable state for the OperationalCredentials cluster.
#[derive(Debug, Default)]
pub struct OpCredState {
    /// P-256 node keypair secret key (stored as raw 32-byte scalar).
    pub noc_keypair_bytes: Option<[u8; 32]>,
    /// NOC entries indexed by fabric index.
    pub noc_entries: Vec<NocEntry>,
    /// Next fabric index to assign.
    pub next_fabric_index: u8,
}

impl OpCredState {
    pub fn new() -> Self {
        Self {
            noc_keypair_bytes: None,
            noc_entries: Vec::new(),
            next_fabric_index: 1,
        }
    }
}

// ── OperationalCredentialsCluster ─────────────────────────────────────────────

/// Server for the OperationalCredentials cluster (0x003E).
pub struct OperationalCredentialsCluster {
    state: Arc<Mutex<OpCredState>>,
}

impl OperationalCredentialsCluster {
    /// Create a new cluster server with fresh state.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(OpCredState::new())),
        }
    }
}

impl Default for OperationalCredentialsCluster {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ClusterServer for OperationalCredentialsCluster {
    fn cluster_id(&self) -> u32 {
        CLUSTER_ID
    }

    async fn read_attribute(&self, attr_id: u32) -> MatterResult<Vec<u8>> {
        match attr_id {
            ATTR_NOCS => {
                let st = self.state.lock().unwrap();
                let mut items = Vec::new();
                for entry in &st.noc_entries {
                    let mut inner = tlv_octet_string(1, &entry.noc);
                    if let Some(icac) = &entry.icac {
                        inner.extend_from_slice(&tlv_octet_string(2, icac));
                    }
                    items.extend_from_slice(&wrap_struct(&inner));
                }
                Ok(wrap_list(&items))
            }
            ATTR_FABRICS => {
                let st = self.state.lock().unwrap();
                let mut items = Vec::new();
                for entry in &st.noc_entries {
                    let mut inner = tlv_uint8(0, entry.fabric_index);
                    // FabricDescriptor: minimal fields.
                    let label_bytes = entry.label.as_bytes();
                    let mut lbl = vec![tlv::TAG_CONTEXT_1 | 0x0C, 5u8, label_bytes.len() as u8];
                    lbl.extend_from_slice(label_bytes);
                    inner.extend_from_slice(&lbl);
                    items.extend_from_slice(&wrap_struct(&inner));
                }
                Ok(wrap_list(&items))
            }
            ATTR_SUPPORTED_FABRICS => Ok(tlv_uint8(0, 5)),
            ATTR_COMMISSIONED_FABRICS => {
                let count = self.state.lock().unwrap().noc_entries.len() as u8;
                Ok(tlv_uint8(0, count))
            }
            _ => Err(MatterError::Transport("unsupported attribute".into())),
        }
    }

    async fn write_attribute(&self, _attr_id: u32, _value: &[u8]) -> MatterResult<()> {
        Err(MatterError::Transport(
            "OperationalCredentials attributes are not writable".into(),
        ))
    }

    async fn invoke_command(&self, cmd_id: u32, args: &[u8]) -> MatterResult<Vec<u8>> {
        match cmd_id {
            CMD_ATTESTATION_REQUEST => {
                // AttestationRequest { AttestationNonce: bytes(32) }
                // Extract nonce: find octet_string at tag 0.
                let nonce = extract_octet_string_tag(args, 0).unwrap_or_else(|| vec![0u8; 32]);

                // AttestationElements TLV: { tag 1: CD (16 zero bytes), tag 2: nonce, tag 3: timestamp }
                let cd = vec![0u8; 16];
                let timestamp: u32 = 0;
                let mut elem_inner = tlv_octet_string(1, &cd);
                elem_inner.extend_from_slice(&tlv_octet_string(2, &nonce));
                elem_inner.extend_from_slice(&tlv_uint32(3, timestamp));
                let attestation_elements = wrap_struct(&elem_inner);

                // AttestationSignature: 64 zero bytes (stub).
                let sig = vec![0u8; 64];

                let mut resp_inner = tlv_octet_string(0, &attestation_elements);
                resp_inner.extend_from_slice(&tlv_octet_string(1, &sig));
                Ok(wrap_struct(&resp_inner))
            }

            CMD_CSR_REQUEST => {
                // CSRRequest { CSRNonce: bytes(32) }
                let csr_nonce = extract_octet_string_tag(args, 0).unwrap_or_else(|| vec![0u8; 32]);

                // Generate a P-256 keypair scalar (32 random bytes as stub).
                let scalar = generate_ephemeral_scalar();
                {
                    let mut st = self.state.lock().unwrap();
                    st.noc_keypair_bytes = Some(scalar);
                }

                // Derive a 65-byte uncompressed public key stub from scalar.
                let pubkey = derive_stub_pubkey(&scalar);

                // NOCSRElements TLV: { tag 1: csr (pubkey as stub), tag 2: CSRNonce }
                let mut noecsr_inner = tlv_octet_string(1, &pubkey);
                noecsr_inner.extend_from_slice(&tlv_octet_string(2, &csr_nonce));
                let nocsr_elements = wrap_struct(&noecsr_inner);

                // Signature over NOCSRElements: 64 zero bytes (stub).
                let sig = vec![0u8; 64];

                let mut resp_inner = tlv_octet_string(0, &nocsr_elements);
                resp_inner.extend_from_slice(&tlv_octet_string(1, &sig));
                Ok(wrap_struct(&resp_inner))
            }

            CMD_ADD_NOC => {
                // AddNOC { NOCValue(0): bytes, ICACValue(1)?: bytes, IPKValue(2): bytes(16),
                //          CaseAdminSubject(3): uint64, AdminVendorId(4): uint16 }
                let noc_value = extract_octet_string_tag(args, 0)
                    .ok_or_else(|| MatterError::Transport("AddNOC: missing NOCValue".into()))?;
                let icac_value = extract_octet_string_tag(args, 1);

                let fabric_index = {
                    let mut st = self.state.lock().unwrap();
                    let idx = st.next_fabric_index;
                    st.next_fabric_index = st.next_fabric_index.saturating_add(1);
                    st.noc_entries.push(NocEntry {
                        noc: noc_value,
                        icac: icac_value,
                        fabric_index: idx,
                        label: String::new(),
                    });
                    idx
                };

                Ok(noc_response(0, fabric_index))
            }

            CMD_UPDATE_FABRIC_LABEL => {
                // UpdateFabricLabel { Label(0): string } → NOCResponse
                // The fabric index context is carried by the CASE session; for the
                // server stub we just return success on the first fabric.
                let fabric_index = self
                    .state
                    .lock()
                    .unwrap()
                    .noc_entries
                    .first()
                    .map(|e| e.fabric_index)
                    .unwrap_or(1);
                Ok(noc_response(0, fabric_index))
            }

            CMD_REMOVE_FABRIC => {
                // RemoveFabric { FabricIndex(0): uint8 }
                let fi = extract_uint8_tag(args, 0).unwrap_or(1);
                {
                    let mut st = self.state.lock().unwrap();
                    st.noc_entries.retain(|e| e.fabric_index != fi);
                }
                Ok(noc_response(0, fi))
            }

            _ => Err(MatterError::Transport(format!(
                "unknown command {cmd_id:#06x}"
            ))),
        }
    }

    fn attribute_ids(&self) -> Vec<u32> {
        vec![
            ATTR_NOCS,
            ATTR_FABRICS,
            ATTR_SUPPORTED_FABRICS,
            ATTR_COMMISSIONED_FABRICS,
        ]
    }

    fn command_ids(&self) -> Vec<u32> {
        vec![
            CMD_ATTESTATION_REQUEST,
            CMD_CSR_REQUEST,
            CMD_ADD_NOC,
            CMD_UPDATE_FABRIC_LABEL,
            CMD_REMOVE_FABRIC,
        ]
    }
}

// ── TLV argument extraction helpers ──────────────────────────────────────────

/// Extract an octet string at the given context tag from TLV bytes.
///
/// Handles both struct-wrapped (`TYPE_STRUCTURE` opener) and raw bodies.
fn extract_octet_string_tag(args: &[u8], tag: u8) -> Option<Vec<u8>> {
    let ctrl = tlv::TAG_CONTEXT_1 | 0x10; // TYPE_OCTET_STRING_1
    let mut i = 0;
    if args.first() == Some(&tlv::TYPE_STRUCTURE) {
        i += 1;
    }
    while i + 2 < args.len() {
        if args[i] == ctrl && args[i + 1] == tag {
            let len = args[i + 2] as usize;
            let start = i + 3;
            if start + len <= args.len() {
                return Some(args[start..start + len].to_vec());
            }
        }
        i += 1;
    }
    None
}

/// Extract a uint8 at the given context tag from TLV bytes.
fn extract_uint8_tag(args: &[u8], tag: u8) -> Option<u8> {
    let ctrl = tlv::TAG_CONTEXT_1 | tlv::TYPE_UNSIGNED_INT_1;
    let mut i = 0;
    if args.first() == Some(&tlv::TYPE_STRUCTURE) {
        i += 1;
    }
    while i + 2 < args.len() {
        if args[i] == ctrl && args[i + 1] == tag {
            return Some(args[i + 2]);
        }
        i += 1;
    }
    None
}

// ── Stub cryptographic helpers ────────────────────────────────────────────────

/// Generate a 32-byte ephemeral scalar.
///
/// This is a stub implementation that produces a deterministic pseudo-random
/// value based on a counter.  Production code would use a CSPRNG.
fn generate_ephemeral_scalar() -> [u8; 32] {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&n.to_le_bytes());
    out[8] = 0x42; // sentinel for testing
    out
}

/// Derive a stub 65-byte uncompressed P-256 public key from a 32-byte scalar.
///
/// For a real implementation this would perform the EC scalar multiplication.
/// Here we just use a recognisable byte pattern so tests can verify structure.
fn derive_stub_pubkey(scalar: &[u8; 32]) -> Vec<u8> {
    let mut pk = vec![0x04u8]; // uncompressed point prefix
    pk.extend_from_slice(scalar); // X coordinate = scalar (stub)
    pk.extend_from_slice(scalar); // Y coordinate = scalar (stub)
    pk.truncate(65);
    pk
}
