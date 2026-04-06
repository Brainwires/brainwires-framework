/// Matter controller — commissions and controls Matter devices.
///
/// Implements:
/// - Commissioning payload parsing (complete)
/// - PASE session establishment via SPAKE2+ over UDP (complete)
/// - CASE session establishment via SIGMA over UDP (complete)
/// - Cluster TLV encoding (complete)
/// - Invoke and read operations over established sessions (complete)
/// - Session caching to reuse CASE sessions across calls
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::clusters;
use super::clusters::{AttributePath, CommandPath};
use super::commissioning::{parse_manual_code, parse_qr_code};
use super::discovery::operational::{OperationalBrowser, derive_compressed_fabric_id};
use super::fabric::FabricManager;
use super::interaction_model::{
    ImOpcode, InteractionStatus, InvokeRequest, InvokeResponse, InvokeResponseItem, ReadRequest,
    ReportData,
};
use super::secure_channel::{
    CaseInitiator, EstablishedSession, PaseCommissioner, SECURE_CHANNEL_PROTOCOL_ID,
    SecureChannelOpcode,
};
use super::server::{build_payload, parse_payload_header};
use super::transport::message::{MatterMessage, MessageHeader, SessionType};
use super::transport::{SessionKeys, UdpTransport};
use super::types::MatterDevice;
use crate::homeauto::BoxStream;
use crate::homeauto::error::{HomeAutoError, HomeAutoResult};
use crate::homeauto::types::{AttributeValue, HomeAutoEvent};

// IM protocol ID
const IM_PROTOCOL_ID: u16 = 0x0001;

/// Monotonic message counter (global, per-process).
static MSG_COUNTER: AtomicU32 = AtomicU32::new(1);

fn next_counter() -> u32 {
    MSG_COUNTER.fetch_add(1, Ordering::SeqCst)
}

// ── Session cache entry ───────────────────────────────────────────────────────

struct CachedSession {
    addr: SocketAddr,
    session: EstablishedSession,
}

// ── ControllerInner ───────────────────────────────────────────────────────────

struct ControllerInner {
    /// Commissioned devices keyed by node_id.
    devices: HashMap<u64, MatterDevice>,
    /// Next node ID to assign on commissioning (reserved for future auto-assignment).
    #[allow(dead_code)]
    next_node_id: u64,
    /// CASE session cache keyed by node_id.
    session_cache: HashMap<u64, CachedSession>,
}

/// A Matter commissioner and cluster client.
///
/// Supports commissioning devices via QR code or manual pairing code,
/// operational device discovery via mDNS, and cluster command invocation.
pub struct MatterController {
    /// Fabric label stored in NOC (informational).
    #[allow(dead_code)]
    fabric_name: String,
    storage_path: std::path::PathBuf,
    inner: Arc<Mutex<ControllerInner>>,
}

impl MatterController {
    /// Create a new controller. `fabric_name` is stored in the fabric label.
    /// `storage_path` is where the fabric certificate and node data are persisted.
    pub async fn new(fabric_name: impl Into<String>, storage_path: &Path) -> HomeAutoResult<Self> {
        tokio::fs::create_dir_all(storage_path)
            .await
            .map_err(HomeAutoError::Io)?;
        let fabric_name = fabric_name.into();
        info!("MatterController initialised (fabric: {})", fabric_name);
        Ok(Self {
            fabric_name,
            storage_path: storage_path.to_path_buf(),
            inner: Arc::new(Mutex::new(ControllerInner {
                devices: HashMap::new(),
                next_node_id: 1,
                session_cache: HashMap::new(),
            })),
        })
    }

    /// Commission a device using its QR code (`MT:...`).
    ///
    /// Parses the commissioning payload, discovers the device via mDNS,
    /// establishes a PASE session over UDP, and returns the commissioned device.
    pub async fn commission_qr(&self, qr_code: &str, node_id: u64) -> HomeAutoResult<MatterDevice> {
        let payload = parse_qr_code(qr_code)
            .map_err(|e| HomeAutoError::MatterCommissioning(e.to_string()))?;
        debug!(
            "Commissioning via QR: VID={:#06x} PID={:#06x} disc={} node_id={node_id}",
            payload.vendor_id, payload.product_id, payload.discriminator
        );

        // Discover device via mDNS — browse _matterc._udp by discriminator.
        // We attempt mDNS discovery with a 10-second timeout.
        let peer_addr = self
            .discover_commissionable(payload.discriminator)
            .await
            .map_err(|e| {
                HomeAutoError::MatterCommissioning(format!(
                    "device discovery failed (disc={}): {e}",
                    payload.discriminator
                ))
            })?;

        info!("Commissioning: found device at {peer_addr}");

        // Run PASE over UDP
        let transport = UdpTransport::bind_addr("0.0.0.0:0")
            .await
            .map_err(|e| HomeAutoError::MatterCommissioning(format!("UDP bind: {e}")))?;

        let session = self
            .run_pase(&transport, peer_addr, payload.passcode)
            .await?;

        // Register session keys
        transport.sessions.lock().await.insert(
            session.session_id,
            SessionKeys {
                encrypt_key: session.encrypt_key,
                decrypt_key: session.decrypt_key,
            },
        );

        info!("PASE commissioned: session_id={}", session.session_id);

        let device = MatterDevice {
            node_id,
            fabric_index: 0,
            name: None,
            vendor_id: payload.vendor_id,
            product_id: payload.product_id,
            endpoints: Vec::new(),
            online: true,
        };
        self.inner
            .lock()
            .await
            .devices
            .insert(node_id, device.clone());
        Ok(device)
    }

    /// Commission a device using its 11-digit manual pairing code.
    pub async fn commission_code(
        &self,
        pairing_code: &str,
        node_id: u64,
    ) -> HomeAutoResult<MatterDevice> {
        let payload = parse_manual_code(pairing_code)
            .map_err(|e| HomeAutoError::MatterCommissioning(e.to_string()))?;
        debug!(
            "Commissioning via manual code: disc={} node_id={node_id}",
            payload.discriminator
        );

        // Discover device via mDNS
        let peer_addr = self
            .discover_commissionable(payload.discriminator)
            .await
            .map_err(|e| {
                HomeAutoError::MatterCommissioning(format!(
                    "device discovery failed (disc={}): {e}",
                    payload.discriminator
                ))
            })?;

        info!("Commissioning: found device at {peer_addr}");

        let transport = UdpTransport::bind_addr("0.0.0.0:0")
            .await
            .map_err(|e| HomeAutoError::MatterCommissioning(format!("UDP bind: {e}")))?;

        let session = self
            .run_pase(&transport, peer_addr, payload.passcode)
            .await?;

        transport.sessions.lock().await.insert(
            session.session_id,
            SessionKeys {
                encrypt_key: session.encrypt_key,
                decrypt_key: session.decrypt_key,
            },
        );

        info!("PASE commissioned: session_id={}", session.session_id);

        let device = MatterDevice {
            node_id,
            fabric_index: 0,
            name: None,
            vendor_id: payload.vendor_id,
            product_id: payload.product_id,
            endpoints: Vec::new(),
            online: true,
        };
        self.inner
            .lock()
            .await
            .devices
            .insert(node_id, device.clone());
        Ok(device)
    }

    /// Return all commissioned devices.
    pub async fn devices(&self) -> HomeAutoResult<Vec<MatterDevice>> {
        Ok(self.inner.lock().await.devices.values().cloned().collect())
    }

    // ── Convenience cluster helpers ───────────────────────────────────────────

    /// Turn a device's On/Off endpoint on or off.
    pub async fn on_off(
        &self,
        device: &MatterDevice,
        endpoint: u16,
        on: bool,
    ) -> HomeAutoResult<()> {
        let (cmd, tlv) = if on {
            (clusters::on_off::CMD_ON, clusters::on_off::on_tlv())
        } else {
            (clusters::on_off::CMD_OFF, clusters::on_off::off_tlv())
        };
        self.invoke(device, endpoint, clusters::on_off::CLUSTER_ID, cmd, &tlv)
            .await
    }

    /// Set the level on a Level Control endpoint (0–254).
    pub async fn set_level(
        &self,
        device: &MatterDevice,
        endpoint: u16,
        level: u8,
    ) -> HomeAutoResult<()> {
        let tlv = clusters::level_control::move_to_level_tlv(level, None);
        self.invoke(
            device,
            endpoint,
            clusters::level_control::CLUSTER_ID,
            clusters::level_control::CMD_MOVE_TO_LEVEL_WITH_ON_OFF,
            &tlv,
        )
        .await
    }

    /// Move a window covering up or down.
    pub async fn window_covering(
        &self,
        device: &MatterDevice,
        endpoint: u16,
        up: bool,
    ) -> HomeAutoResult<()> {
        let cmd = if up {
            clusters::window_covering::CMD_UP_OR_OPEN
        } else {
            clusters::window_covering::CMD_DOWN_OR_CLOSE
        };
        self.invoke(
            device,
            endpoint,
            clusters::window_covering::CLUSTER_ID,
            cmd,
            &[],
        )
        .await
    }

    /// Lock or unlock a door lock endpoint.
    pub async fn door_lock(
        &self,
        device: &MatterDevice,
        endpoint: u16,
        lock: bool,
        pin: Option<&[u8]>,
    ) -> HomeAutoResult<()> {
        let cmd = if lock {
            clusters::door_lock::CMD_LOCK_DOOR
        } else {
            clusters::door_lock::CMD_UNLOCK_DOOR
        };
        let tlv = clusters::door_lock::lock_tlv(pin);
        self.invoke(device, endpoint, clusters::door_lock::CLUSTER_ID, cmd, &tlv)
            .await
    }

    // ── Generic interaction model operations ──────────────────────────────────

    /// Invoke a cluster command on a device endpoint.
    ///
    /// Establishes or reuses a CASE session, sends an InvokeRequest, and
    /// awaits the InvokeResponse.
    pub async fn invoke(
        &self,
        device: &MatterDevice,
        endpoint: u16,
        cluster: u32,
        cmd: u32,
        tlv: &[u8],
    ) -> HomeAutoResult<()> {
        debug!(
            "Matter invoke: node={} ep={endpoint} cluster={cluster:#010x} cmd={cmd:#010x} payload_len={}",
            device.node_id,
            tlv.len()
        );

        let (transport, session, peer) = self.get_or_establish_session(device).await?;

        let path = CommandPath::new(endpoint, cluster, cmd);
        let req = InvokeRequest::new(path, tlv.to_vec());
        let exchange_id = (next_counter() & 0xFFFF) as u16;

        let wire_payload = build_payload(
            ImOpcode::InvokeRequest as u8,
            exchange_id,
            IM_PROTOCOL_ID,
            &req.encode(),
        );
        let msg = build_matter_message(session.session_id, next_counter(), wire_payload);

        let resp_msg = send_and_recv(&transport, peer, msg).await?;

        // Parse the response payload header
        let (_, resp_opcode, _, resp_proto, resp_app) = parse_payload_header(&resp_msg.payload)
            .ok_or_else(|| HomeAutoError::Matter("invoke: bad response payload header".into()))?;

        if resp_proto != IM_PROTOCOL_ID {
            return Err(HomeAutoError::Matter(format!(
                "invoke: unexpected response protocol {resp_proto:#06x}"
            )));
        }

        if resp_opcode != ImOpcode::InvokeResponse as u8 {
            return Err(HomeAutoError::Matter(format!(
                "invoke: expected InvokeResponse (0x09), got {resp_opcode:#04x}"
            )));
        }

        let resp = InvokeResponse::decode(resp_app)
            .map_err(|e| HomeAutoError::Matter(format!("invoke: decode InvokeResponse: {e}")))?;

        // Check for any failure status in the response
        for item in &resp.invoke_responses {
            if let InvokeResponseItem::Status { path: _, status } = item {
                if *status != InteractionStatus::Success {
                    return Err(HomeAutoError::MatterCluster {
                        cluster,
                        cmd,
                        msg: format!("invoke failed with status {:?}", status),
                    });
                }
            }
        }

        Ok(())
    }

    /// Read an attribute from a device endpoint.
    ///
    /// Establishes or reuses a CASE session, sends a ReadRequest, and
    /// returns the decoded attribute value.
    pub async fn read_attr(
        &self,
        device: &MatterDevice,
        endpoint: u16,
        cluster: u32,
        attr: u32,
    ) -> HomeAutoResult<AttributeValue> {
        debug!(
            "Matter read_attr: node={} ep={endpoint} cluster={cluster:#010x} attr={attr:#010x}",
            device.node_id
        );

        let (transport, session, peer) = self.get_or_establish_session(device).await?;

        let path = AttributePath::specific(endpoint, cluster, attr);
        let req = ReadRequest::new(vec![path.clone()]);
        let exchange_id = (next_counter() & 0xFFFF) as u16;

        let wire_payload = build_payload(
            ImOpcode::ReadRequest as u8,
            exchange_id,
            IM_PROTOCOL_ID,
            &req.encode(),
        );
        let msg = build_matter_message(session.session_id, next_counter(), wire_payload);

        let resp_msg = send_and_recv(&transport, peer, msg).await?;

        let (_, resp_opcode, _, resp_proto, resp_app) = parse_payload_header(&resp_msg.payload)
            .ok_or_else(|| HomeAutoError::Matter("read_attr: bad response header".into()))?;

        if resp_proto != IM_PROTOCOL_ID || resp_opcode != ImOpcode::ReportData as u8 {
            return Err(HomeAutoError::Matter(format!(
                "read_attr: expected ReportData, got proto={resp_proto:#06x} opcode={resp_opcode:#04x}"
            )));
        }

        let report = ReportData::decode(resp_app)
            .map_err(|e| HomeAutoError::Matter(format!("read_attr: decode ReportData: {e}")))?;

        let attr_data = report
            .attribute_reports
            .into_iter()
            .find(|d| {
                d.path.endpoint_id == Some(endpoint)
                    && d.path.cluster_id == Some(cluster)
                    && d.path.attribute_id == Some(attr)
            })
            .ok_or_else(|| HomeAutoError::Matter(format!(
                "read_attr: attribute ep={endpoint} cluster={cluster:#010x} attr={attr:#010x} not in response"
            )))?;

        // Convert raw TLV bytes to AttributeValue
        Ok(tlv_to_attribute_value(&attr_data.data))
    }

    /// Subscribe to a stream of events from all commissioned devices.
    pub fn events(&self) -> BoxStream<'static, HomeAutoEvent> {
        Box::pin(futures::stream::empty())
    }

    // ── Internal session management ───────────────────────────────────────────

    /// Get or establish a CASE session to the given device.
    ///
    /// Returns `(transport, established_session, peer_addr)`.
    async fn get_or_establish_session(
        &self,
        device: &MatterDevice,
    ) -> HomeAutoResult<(Arc<UdpTransport>, EstablishedSession, SocketAddr)> {
        // Check session cache
        {
            let inner = self.inner.lock().await;
            if let Some(cached) = inner.session_cache.get(&device.node_id) {
                let transport = UdpTransport::bind_addr("0.0.0.0:0")
                    .await
                    .map_err(|e| HomeAutoError::Matter(format!("UDP bind: {e}")))?;
                let transport = Arc::new(transport);
                transport.sessions.lock().await.insert(
                    cached.session.session_id,
                    SessionKeys {
                        encrypt_key: cached.session.encrypt_key,
                        decrypt_key: cached.session.decrypt_key,
                    },
                );
                return Ok((transport, cached.session.clone(), cached.addr));
            }
        }

        // Need to establish a new CASE session — discover node and load fabric
        let fabric_manager = FabricManager::load(&self.storage_path)
            .await
            .map_err(|e| HomeAutoError::Matter(format!("FabricManager load: {e}")))?;

        // We need at least one fabric to do CASE
        let fabrics = fabric_manager.fabrics();
        if fabrics.is_empty() {
            return Err(HomeAutoError::Matter(
                "no fabric found — commission the device first".into(),
            ));
        }
        let fabric_entry = &fabrics[0];
        let fabric = &fabric_entry.descriptor;

        // Discover the device via mDNS operational browsing
        let cfid = derive_compressed_fabric_id(fabric);
        let browser = OperationalBrowser::new()
            .map_err(|e| HomeAutoError::Matter(format!("OperationalBrowser: {e}")))?;
        let peer = browser
            .discover_node(cfid, device.node_id, 10_000)
            .await
            .map_err(|e| HomeAutoError::Matter(format!("discover_node: {e}")))?;

        // Load the node's private key from fabric entry
        let sk_bytes: [u8; 32] = fabric_entry.private_key_bytes[..32]
            .try_into()
            .map_err(|_| HomeAutoError::Matter("invalid private key length".into()))?;
        let node_key = p256::SecretKey::from_bytes(&sk_bytes.into())
            .map_err(|e| HomeAutoError::Matter(format!("parse node key: {e}")))?;

        let noc = super::fabric::MatterCert::decode(&fabric_entry.noc_der)
            .map_err(|e| HomeAutoError::Matter(format!("decode NOC: {e}")))?;
        let icac = fabric_entry
            .icac_der
            .as_deref()
            .and_then(|d| super::fabric::MatterCert::decode(d).ok());

        let transport = Arc::new(
            UdpTransport::bind_addr("0.0.0.0:0")
                .await
                .map_err(|e| HomeAutoError::Matter(format!("UDP bind: {e}")))?,
        );

        // Run CASE (SIGMA protocol)
        let mut initiator = CaseInitiator::new(node_key, noc, icac, fabric.clone());

        let (session_id, sigma1) = initiator
            .build_sigma1()
            .map_err(|e| HomeAutoError::Matter(format!("CASE Sigma1: {e}")))?;

        // Send Sigma1, receive Sigma2
        let exchange_id = (next_counter() & 0xFFFF) as u16;
        let wire1 = build_payload(
            SecureChannelOpcode::Sigma1 as u8,
            exchange_id,
            SECURE_CHANNEL_PROTOCOL_ID,
            &sigma1,
        );
        let sigma1_msg = build_matter_message(0, next_counter(), wire1);
        let sigma2_resp = send_and_recv(&transport, peer, sigma1_msg).await?;

        let (_, op2, _, _, sigma2_app) = parse_payload_header(&sigma2_resp.payload)
            .ok_or_else(|| HomeAutoError::Matter("CASE: bad Sigma2 header".into()))?;
        if op2 != SecureChannelOpcode::Sigma2 as u8 {
            return Err(HomeAutoError::Matter(format!(
                "CASE: expected Sigma2, got opcode {op2:#04x}"
            )));
        }

        // Process Sigma2, produce Sigma3
        let sigma3 = initiator
            .handle_sigma2(sigma2_app)
            .map_err(|e| HomeAutoError::Matter(format!("CASE handle_sigma2: {e}")))?;

        // Send Sigma3
        let wire3 = build_payload(
            SecureChannelOpcode::Sigma3 as u8,
            exchange_id,
            SECURE_CHANNEL_PROTOCOL_ID,
            &sigma3,
        );
        let sigma3_msg = build_matter_message(0, next_counter(), wire3);
        let status_resp = send_and_recv(&transport, peer, sigma3_msg).await?;

        // Parse StatusReport to confirm success
        let (_, op_sr, _, _, _) = parse_payload_header(&status_resp.payload)
            .ok_or_else(|| HomeAutoError::Matter("CASE: bad StatusReport header".into()))?;
        if op_sr != SecureChannelOpcode::StatusReport as u8 {
            warn!("CASE: expected StatusReport, got {op_sr:#04x}");
        }

        // Extract the established session
        let session = initiator
            .established_session()
            .ok_or_else(|| HomeAutoError::Matter("CASE: session not established".into()))?
            .clone();

        // Register session keys
        transport.sessions.lock().await.insert(
            session_id,
            SessionKeys {
                encrypt_key: session.encrypt_key,
                decrypt_key: session.decrypt_key,
            },
        );

        info!("CASE: session {session_id} established with {peer}");

        // Cache the session
        self.inner.lock().await.session_cache.insert(
            device.node_id,
            CachedSession {
                addr: peer,
                session: session.clone(),
            },
        );

        Ok((transport, session, peer))
    }

    // ── PASE commissioning helper ─────────────────────────────────────────────

    /// Run the full PASE handshake against `peer` using `passcode`.
    ///
    /// Returns the established PASE session.
    async fn run_pase(
        &self,
        transport: &UdpTransport,
        peer: SocketAddr,
        passcode: u32,
    ) -> HomeAutoResult<EstablishedSession> {
        let mut commissioner = PaseCommissioner::new(passcode);

        // Step 1: send PBKDFParamRequest
        let (_session_id, param_req) = commissioner
            .build_param_request()
            .map_err(|e| HomeAutoError::MatterCommissioning(format!("PBKDFParamRequest: {e}")))?;

        let exchange_id = (next_counter() & 0xFFFF) as u16;
        let wire_req = build_payload(
            SecureChannelOpcode::PbkdfParamRequest as u8,
            exchange_id,
            SECURE_CHANNEL_PROTOCOL_ID,
            &param_req,
        );
        let param_req_msg = build_matter_message(0, next_counter(), wire_req);
        let param_resp_msg = send_and_recv(transport, peer, param_req_msg)
            .await
            .map_err(|e| {
                HomeAutoError::MatterCommissioning(format!("PBKDFParamResponse recv: {e}"))
            })?;

        let (_, op_r, _, _, param_resp_app) = parse_payload_header(&param_resp_msg.payload)
            .ok_or_else(|| {
                HomeAutoError::MatterCommissioning("bad PBKDFParamResponse header".into())
            })?;
        if op_r != SecureChannelOpcode::PbkdfParamResponse as u8 {
            return Err(HomeAutoError::MatterCommissioning(format!(
                "expected PBKDFParamResponse, got {op_r:#04x}"
            )));
        }

        // Step 2: send Pake1
        let pake1 = commissioner
            .handle_param_response(param_resp_app)
            .map_err(|e| {
                HomeAutoError::MatterCommissioning(format!("handle_param_response: {e}"))
            })?;

        let wire_pake1 = build_payload(
            SecureChannelOpcode::Pake1 as u8,
            exchange_id,
            SECURE_CHANNEL_PROTOCOL_ID,
            &pake1,
        );
        let pake1_msg = build_matter_message(0, next_counter(), wire_pake1);
        let pake2_resp = send_and_recv(transport, peer, pake1_msg)
            .await
            .map_err(|e| HomeAutoError::MatterCommissioning(format!("Pake2 recv: {e}")))?;

        let (_, op_2, _, _, pake2_app) = parse_payload_header(&pake2_resp.payload)
            .ok_or_else(|| HomeAutoError::MatterCommissioning("bad Pake2 header".into()))?;
        if op_2 != SecureChannelOpcode::Pake2 as u8 {
            return Err(HomeAutoError::MatterCommissioning(format!(
                "expected Pake2, got {op_2:#04x}"
            )));
        }

        // Step 3: send Pake3
        let pake3 = commissioner
            .handle_pake2(pake2_app)
            .map_err(|e| HomeAutoError::MatterCommissioning(format!("handle_pake2: {e}")))?;

        let wire_pake3 = build_payload(
            SecureChannelOpcode::Pake3 as u8,
            exchange_id,
            SECURE_CHANNEL_PROTOCOL_ID,
            &pake3,
        );
        let pake3_msg = build_matter_message(0, next_counter(), wire_pake3);
        let status_msg = send_and_recv(transport, peer, pake3_msg)
            .await
            .map_err(|e| HomeAutoError::MatterCommissioning(format!("StatusReport recv: {e}")))?;

        // Parse StatusReport for success
        let (_, op_sr, _, _, _) = parse_payload_header(&status_msg.payload)
            .ok_or_else(|| HomeAutoError::MatterCommissioning("bad StatusReport header".into()))?;
        if op_sr != SecureChannelOpcode::StatusReport as u8 {
            warn!("PASE: expected StatusReport, got {op_sr:#04x}");
        }

        commissioner.established_session().cloned().ok_or_else(|| {
            HomeAutoError::MatterCommissioning("PASE: session not established after Pake3".into())
        })
    }

    // ── mDNS discovery helper ─────────────────────────────────────────────────

    /// Discover a commissionable device by discriminator via mDNS.
    ///
    /// Browses `_matterc._udp` for up to 10 seconds.  Returns a `SocketAddr`
    /// suitable for PASE commissioning.
    async fn discover_commissionable(&self, discriminator: u16) -> HomeAutoResult<SocketAddr> {
        use mdns_sd::{ServiceDaemon, ServiceEvent};
        use std::time::Duration;

        let daemon =
            ServiceDaemon::new().map_err(|e| HomeAutoError::Matter(format!("mDNS daemon: {e}")))?;

        let receiver = daemon
            .browse("_matterc._udp")
            .map_err(|e| HomeAutoError::Matter(format!("mDNS browse: {e}")))?;

        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let disc_str = discriminator.to_string();

        loop {
            let remaining = deadline
                .checked_duration_since(std::time::Instant::now())
                .unwrap_or(Duration::ZERO);
            if remaining.is_zero() {
                break;
            }

            match receiver.recv_timeout(remaining) {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    // Check the D TXT record for a discriminator match
                    let d_val_owned;
                    let d_val = if let Some(prop) = info.get_properties().get("D") {
                        d_val_owned = prop.val_str().to_string();
                        d_val_owned.as_str()
                    } else {
                        ""
                    };
                    if d_val == disc_str {
                        let port = info.get_port();
                        let addr = info
                            .get_addresses()
                            .iter()
                            .find(|a| matches!(a, std::net::IpAddr::V4(_)))
                            .or_else(|| info.get_addresses().iter().next())
                            .copied()
                            .ok_or_else(|| {
                                HomeAutoError::Matter("mDNS: no address for device".into())
                            })?;
                        let _ = daemon.stop_browse("_matterc._udp");
                        return Ok(SocketAddr::new(addr, port));
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }

        let _ = daemon.stop_browse("_matterc._udp");
        Err(HomeAutoError::Matter(format!(
            "commissionable device with discriminator={discriminator} not found within 10s"
        )))
    }
}

// ── Transport helpers ─────────────────────────────────────────────────────────

/// Build a MatterMessage with the given session_id, counter, and payload.
fn build_matter_message(session_id: u16, counter: u32, payload: Vec<u8>) -> MatterMessage {
    MatterMessage {
        header: MessageHeader {
            version: 0,
            session_id,
            session_type: SessionType::Unicast,
            source_node_id: None,
            dest_node_id: None,
            message_counter: counter,
            security_flags: 0x00,
        },
        payload,
    }
}

/// Send a `MatterMessage` and wait for one response datagram.
///
/// Uses a 5-second timeout.
async fn send_and_recv(
    transport: &UdpTransport,
    peer: SocketAddr,
    msg: MatterMessage,
) -> HomeAutoResult<MatterMessage> {
    transport
        .send(&msg, peer)
        .await
        .map_err(|e| HomeAutoError::Matter(format!("send_and_recv: send: {e}")))?;

    // Wait for response with timeout
    match tokio::time::timeout(std::time::Duration::from_secs(5), transport.recv()).await {
        Ok(Ok((resp, _))) => Ok(resp),
        Ok(Err(e)) => Err(HomeAutoError::Matter(format!("send_and_recv: recv: {e}"))),
        Err(_) => Err(HomeAutoError::Timeout),
    }
}

// ── TLV → AttributeValue conversion ──────────────────────────────────────────

/// Convert raw TLV bytes (attribute value blob) to an `AttributeValue`.
///
/// This handles the common cases: uint8, uint16, uint32, bool, and raw bytes.
/// Unknown encodings are returned as `AttributeValue::Bytes`.
fn tlv_to_attribute_value(data: &[u8]) -> AttributeValue {
    if data.is_empty() {
        return AttributeValue::Null;
    }

    // The data may be wrapped in a TLV struct — skip outer struct wrapper if present
    let inner = if data[0] == 0x15 {
        // Anonymous struct: peek inside at the first element
        if data.len() >= 3 { &data[1..] } else { data }
    } else {
        data
    };

    if inner.is_empty() {
        return AttributeValue::Null;
    }

    let ctrl = inner[0];
    let val_type = ctrl & 0x1F;
    let tag_type = (ctrl >> 5) & 0x07;

    // Skip tag bytes
    let value_start = 1 + if tag_type == 1 { 1usize } else { 0usize };

    match val_type {
        0x08 => AttributeValue::Bool(false),
        0x09 => AttributeValue::Bool(true),
        0x04 if inner.len() > value_start => AttributeValue::U8(inner[value_start]),
        0x05 if inner.len() >= value_start + 2 => AttributeValue::U16(u16::from_le_bytes([
            inner[value_start],
            inner[value_start + 1],
        ])),
        0x06 if inner.len() >= value_start + 4 => {
            let bytes: [u8; 4] = inner[value_start..value_start + 4].try_into().unwrap();
            AttributeValue::U32(u32::from_le_bytes(bytes))
        }
        _ => AttributeValue::Bytes(data.to_vec()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_matter_message_has_correct_session_id() {
        let msg = build_matter_message(0x0042, 1, vec![0xDE, 0xAD]);
        assert_eq!(msg.header.session_id, 0x0042);
        assert_eq!(msg.header.message_counter, 1);
        assert_eq!(msg.payload, vec![0xDE, 0xAD]);
    }

    #[test]
    fn tlv_to_attribute_value_uint8() {
        // ctrl = 0x04 (anonymous uint8), val = 127
        let data = vec![0x04u8, 127];
        assert_eq!(tlv_to_attribute_value(&data), AttributeValue::U8(127));
    }

    #[test]
    fn tlv_to_attribute_value_bool_true() {
        let data = vec![0x09u8]; // anonymous bool true
        assert_eq!(tlv_to_attribute_value(&data), AttributeValue::Bool(true));
    }

    #[test]
    fn tlv_to_attribute_value_bool_false() {
        let data = vec![0x08u8]; // anonymous bool false
        assert_eq!(tlv_to_attribute_value(&data), AttributeValue::Bool(false));
    }

    #[test]
    fn tlv_to_attribute_value_uint16() {
        let data = vec![0x05u8, 0x01, 0x00]; // uint16 = 1
        assert_eq!(tlv_to_attribute_value(&data), AttributeValue::U16(1));
    }

    #[test]
    fn tlv_to_attribute_value_empty_is_null() {
        assert_eq!(tlv_to_attribute_value(&[]), AttributeValue::Null);
    }
}
