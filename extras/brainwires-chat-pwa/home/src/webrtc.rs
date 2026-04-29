//! WebRTC peer wrapper around `webrtc-rs` (the Brainwires fork pinned by
//! the workspace at `0.20.0-alpha.1`).
//!
//! M1 exercises the create-offer / set-answer dance between two local peers
//! and confirms a JSON frame round-trips on a `"a2a"` data channel. The
//! length-prefixed frame codec, ICE-restart reconnect, and Cloudflare Calls
//! TURN credential minting land in M3 and M7 respectively.
//!
//! ## API note
//!
//! Upstream webrtc-rs (the crates.io tree) wires events with closures:
//! `pc.on_ice_candidate(Box::new(|c| async move { ... }))`. The Brainwires
//! fork uses an event-handler trait passed at builder time:
//! `PeerConnectionBuilder::new().with_handler(Arc<dyn PeerConnectionEventHandler>)`.
//! That trait drives the [`build_peer`] / [`PeerHandler`] split below.
//! DataChannel reads are also poll-based on the fork (`dc.poll()`), not
//! callback-based.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use bytes::BytesMut;
use tokio::sync::broadcast;
use webrtc::data_channel::{
    DataChannel as WrtcDataChannel, DataChannelEvent, RTCDataChannelInit,
};
use webrtc::peer_connection::{
    MediaEngine, PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
    RTCConfigurationBuilder, RTCIceConnectionState, RTCIceServer, RTCPeerConnectionIceEvent,
    RTCPeerConnectionState, RTCSignalingState, Registry, register_default_interceptors,
};
use webrtc::media_stream::track_remote::TrackRemote;

/// The canonical data-channel label used by the dial-home protocol.
pub const A2A_CHANNEL_LABEL: &str = "a2a";

/// Lightweight event broadcast for an [`HomePeer`]. Keeps the M1 test (and
/// future signaling-route plumbing) out of the `webrtc-rs` event-handler
/// trait directly.
///
/// `Arc<dyn DataChannel>` does not implement `Debug`, so we hand-roll the
/// formatter rather than `#[derive(Debug)]` it.
#[derive(Clone)]
pub enum PeerEvent {
    /// New ICE candidate the local peer wants to send to the remote.
    LocalIceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_mline_index: Option<u16>,
    },
    /// Connection state changed.
    ConnectionState(RTCPeerConnectionState),
    /// ICE connection state changed.
    IceConnectionState(RTCIceConnectionState),
    /// Signaling state changed.
    SignalingState(RTCSignalingState),
    /// Remote opened a data channel (answerer side).
    DataChannel(Arc<dyn WrtcDataChannel>),
}

impl std::fmt::Debug for PeerEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalIceCandidate {
                candidate,
                sdp_mid,
                sdp_mline_index,
            } => f
                .debug_struct("LocalIceCandidate")
                .field("candidate", candidate)
                .field("sdp_mid", sdp_mid)
                .field("sdp_mline_index", sdp_mline_index)
                .finish(),
            Self::ConnectionState(s) => f.debug_tuple("ConnectionState").field(s).finish(),
            Self::IceConnectionState(s) => f.debug_tuple("IceConnectionState").field(s).finish(),
            Self::SignalingState(s) => f.debug_tuple("SignalingState").field(s).finish(),
            Self::DataChannel(_) => f.debug_tuple("DataChannel").field(&"<dyn DataChannel>").finish(),
        }
    }
}

struct PeerHandler {
    tx: broadcast::Sender<PeerEvent>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for PeerHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        if let Ok(init) = event.candidate.to_json() {
            let _ = self.tx.send(PeerEvent::LocalIceCandidate {
                candidate: init.candidate,
                sdp_mid: init.sdp_mid,
                sdp_mline_index: init.sdp_mline_index,
            });
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        let _ = self.tx.send(PeerEvent::ConnectionState(state));
    }

    async fn on_ice_connection_state_change(&self, state: RTCIceConnectionState) {
        let _ = self.tx.send(PeerEvent::IceConnectionState(state));
    }

    async fn on_signaling_state_change(&self, state: RTCSignalingState) {
        let _ = self.tx.send(PeerEvent::SignalingState(state));
    }

    async fn on_track(&self, _track: Arc<dyn TrackRemote>) {
        // M1 is data-channel only; ignore media tracks.
    }

    async fn on_data_channel(&self, dc: Arc<dyn WrtcDataChannel>) {
        let _ = self.tx.send(PeerEvent::DataChannel(dc));
    }
}

/// One end of a WebRTC connection plus a broadcast bus for its events.
pub struct HomePeer {
    pub pc: Arc<dyn PeerConnection>,
    tx: broadcast::Sender<PeerEvent>,
}

impl HomePeer {
    /// Subscribe to this peer's event stream.
    pub fn subscribe(&self) -> broadcast::Receiver<PeerEvent> {
        self.tx.subscribe()
    }
}

/// Build a default-configured peer.
///
/// `ice_servers` is a list of STUN/TURN URLs. Pass an empty `Vec` to use the
/// default Google STUN. M7 will replace this with a Cloudflare-Calls-minted
/// TURN credential.
pub async fn build_peer(ice_servers: Vec<String>) -> Result<HomePeer> {
    let urls = if ice_servers.is_empty() {
        vec!["stun:stun.l.google.com:19302".to_string()]
    } else {
        ice_servers
    };

    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(|e| anyhow!("register_default_codecs: {e}"))?;
    let registry = register_default_interceptors(Registry::new(), &mut media_engine)
        .map_err(|e| anyhow!("register_default_interceptors: {e}"))?;

    let cfg = RTCConfigurationBuilder::new()
        .with_ice_servers(vec![RTCIceServer {
            urls,
            username: String::new(),
            credential: String::new(),
        }])
        .build();

    let (tx, _) = broadcast::channel::<PeerEvent>(64);
    let handler = Arc::new(PeerHandler { tx: tx.clone() });

    // `PeerConnectionBuilder` is generic over the address type used for UDP/TCP
    // candidate bindings (`A: ToSocketAddrs`). We bind on ephemeral ports on
    // `0.0.0.0` so host-candidate gathering works between two in-process peers
    // without needing TURN. Using `&'static str` lets the inference resolve.
    let pc: Arc<dyn PeerConnection> = Arc::new(
        PeerConnectionBuilder::<&'static str>::new()
            .with_configuration(cfg)
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .with_handler(handler.clone() as Arc<dyn PeerConnectionEventHandler>)
            .with_udp_addrs(vec!["0.0.0.0:0"])
            .build()
            .await
            .map_err(|e| anyhow!("PeerConnectionBuilder::build: {e}"))?,
    );

    Ok(HomePeer { pc, tx })
}

/// Open the canonical `"a2a"` data channel as the offerer.
pub async fn open_a2a_channel(peer: &HomePeer) -> Result<Arc<dyn WrtcDataChannel>> {
    let init = RTCDataChannelInit {
        ordered: true,
        max_retransmits: None,
        max_packet_life_time: None,
        protocol: String::new(),
        negotiated: None,
    };
    peer.pc
        .create_data_channel(A2A_CHANNEL_LABEL, Some(init))
        .await
        .map_err(|e| anyhow!("create_data_channel({A2A_CHANNEL_LABEL}): {e}"))
}

/// Send a UTF-8 text frame on a data channel.
pub async fn send_text(dc: &Arc<dyn WrtcDataChannel>, s: &str) -> Result<()> {
    dc.send_text(s).await.map_err(|e| anyhow!("send_text: {e}"))
}

/// Send a binary frame on a data channel.
pub async fn send_bytes(dc: &Arc<dyn WrtcDataChannel>, data: &[u8]) -> Result<()> {
    dc.send(BytesMut::from(data))
        .await
        .map_err(|e| anyhow!("send_bytes: {e}"))
}

/// Read the next text/binary message off a data channel. Returns `None` when
/// the channel closes.
pub async fn recv_text(dc: &Arc<dyn WrtcDataChannel>) -> Option<String> {
    loop {
        match dc.poll().await {
            Some(DataChannelEvent::OnMessage(msg)) => {
                return Some(String::from_utf8_lossy(&msg.data).into_owned());
            }
            Some(DataChannelEvent::OnClose) | None => return None,
            _ => continue,
        }
    }
}

/// Drive a peer to [`RTCPeerConnectionState::Connected`] (or an error). Used
/// in tests to gate on connection establishment.
pub async fn wait_connected(peer: &HomePeer) -> Result<()> {
    let mut rx = peer.subscribe();
    loop {
        match rx.recv().await {
            Ok(PeerEvent::ConnectionState(RTCPeerConnectionState::Connected)) => return Ok(()),
            Ok(PeerEvent::ConnectionState(RTCPeerConnectionState::Failed)) => {
                return Err(anyhow!("peer entered Failed state before Connected"));
            }
            Ok(PeerEvent::ConnectionState(RTCPeerConnectionState::Closed)) => {
                return Err(anyhow!("peer Closed before Connected"));
            }
            Ok(_) => continue,
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => return Err(anyhow!("peer event stream ended before Connected")),
        }
    }
}

/// Forward local ICE candidates from `from` into `into.add_ice_candidate`.
/// Returns a JoinHandle that exits when `from`'s broadcast closes or the peer
/// reaches Connected/Failed.
pub fn spawn_ice_relay(from: &HomePeer, into: Arc<dyn PeerConnection>) -> tokio::task::JoinHandle<()> {
    let mut rx = from.subscribe();
    tokio::spawn(async move {
        use webrtc::peer_connection::RTCIceCandidateInit;
        loop {
            match rx.recv().await {
                Ok(PeerEvent::LocalIceCandidate {
                    candidate,
                    sdp_mid,
                    sdp_mline_index,
                }) => {
                    let _ = into
                        .add_ice_candidate(RTCIceCandidateInit {
                            candidate,
                            sdp_mid,
                            sdp_mline_index,
                            username_fragment: None,
                            url: None,
                        })
                        .await;
                }
                Ok(PeerEvent::ConnectionState(s))
                    if matches!(
                        s,
                        RTCPeerConnectionState::Connected
                            | RTCPeerConnectionState::Failed
                            | RTCPeerConnectionState::Closed
                    ) =>
                {
                    break;
                }
                Err(broadcast::error::RecvError::Closed) => break,
                _ => continue,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use webrtc::peer_connection::RTCSessionDescription;

    /// Spin up two peers in-process, do the offer/answer dance manually,
    /// open the `"a2a"` data channel, and round-trip a single ping/pong
    /// frame. Validates the webrtc-rs (Brainwires fork) scaffolding before
    /// it gets wired into the signaling server in M3.
    ///
    /// Requires `flavor = "multi_thread"` because webrtc-rs spawns
    /// background tasks that need a real thread pool.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn ping_roundtrip_two_local_peers() -> Result<()> {
        let alice = build_peer(vec![]).await?;
        let bob = build_peer(vec![]).await?;

        // Forward ICE candidates between peers locally.
        let _alice_to_bob = spawn_ice_relay(&alice, bob.pc.clone());
        let _bob_to_alice = spawn_ice_relay(&bob, alice.pc.clone());

        // Bob (answerer): when the data channel arrives, spawn a poll task
        // that echoes back any text it receives.
        let (bob_got_tx, mut bob_got_rx) = mpsc::channel::<String>(1);
        let mut bob_events = bob.subscribe();
        let bob_dc_task = tokio::spawn(async move {
            while let Ok(ev) = bob_events.recv().await {
                if let PeerEvent::DataChannel(dc) = ev {
                    let bob_got_tx = bob_got_tx.clone();
                    tokio::spawn(async move {
                        loop {
                            match dc.poll().await {
                                Some(DataChannelEvent::OnMessage(msg)) => {
                                    let s = String::from_utf8_lossy(&msg.data).into_owned();
                                    let _ = bob_got_tx.send(s).await;
                                    let _ = dc.send_text("pong").await;
                                }
                                Some(DataChannelEvent::OnClose) | None => break,
                                _ => continue,
                            }
                        }
                    });
                    break;
                }
            }
        });

        // Alice (offerer): open the channel, send ping when open, capture pong.
        let dc = open_a2a_channel(&alice).await?;
        let (alice_got_tx, mut alice_got_rx) = mpsc::channel::<String>(1);
        let dc_for_reader = dc.clone();
        let alice_reader = tokio::spawn(async move {
            loop {
                match dc_for_reader.poll().await {
                    Some(DataChannelEvent::OnOpen) => {
                        let _ = dc_for_reader.send_text("ping").await;
                    }
                    Some(DataChannelEvent::OnMessage(msg)) => {
                        let s = String::from_utf8_lossy(&msg.data).into_owned();
                        let _ = alice_got_tx.send(s).await;
                        break;
                    }
                    Some(DataChannelEvent::OnClose) | None => break,
                    _ => continue,
                }
            }
        });

        // Offer / answer.
        let offer = alice
            .pc
            .create_offer(None)
            .await
            .map_err(|e| anyhow!("alice.create_offer: {e}"))?;
        let offer_sdp = offer.sdp.clone();
        alice
            .pc
            .set_local_description(offer)
            .await
            .map_err(|e| anyhow!("alice.set_local_description: {e}"))?;
        bob.pc
            .set_remote_description(
                RTCSessionDescription::offer(offer_sdp).map_err(|e| anyhow!("offer: {e}"))?,
            )
            .await
            .map_err(|e| anyhow!("bob.set_remote_description(offer): {e}"))?;
        let answer = bob
            .pc
            .create_answer(None)
            .await
            .map_err(|e| anyhow!("bob.create_answer: {e}"))?;
        let answer_sdp = answer.sdp.clone();
        bob.pc
            .set_local_description(answer)
            .await
            .map_err(|e| anyhow!("bob.set_local_description: {e}"))?;
        alice
            .pc
            .set_remote_description(
                RTCSessionDescription::answer(answer_sdp).map_err(|e| anyhow!("answer: {e}"))?,
            )
            .await
            .map_err(|e| anyhow!("alice.set_remote_description(answer): {e}"))?;

        // Bob should receive "ping".
        let bob_got = tokio::time::timeout(Duration::from_secs(15), bob_got_rx.recv())
            .await
            .map_err(|_| anyhow!("timed out waiting for bob to receive ping"))?
            .ok_or_else(|| anyhow!("bob channel closed before receiving ping"))?;
        assert_eq!(bob_got, "ping");

        // Alice should receive "pong".
        let alice_got = tokio::time::timeout(Duration::from_secs(15), alice_got_rx.recv())
            .await
            .map_err(|_| anyhow!("timed out waiting for alice to receive pong"))?
            .ok_or_else(|| anyhow!("alice channel closed before receiving pong"))?;
        assert_eq!(alice_got, "pong");

        // Cleanup.
        let _ = dc.close().await;
        let _ = alice.pc.close().await;
        let _ = bob.pc.close().await;
        let _ = alice_reader.await;
        let _ = bob_dc_task.await;
        Ok(())
    }

    #[tokio::test]
    async fn build_peer_smoke() -> Result<()> {
        let p = build_peer(vec![]).await?;
        // Just confirm we can create a data channel with the canonical label.
        let dc = open_a2a_channel(&p).await?;
        assert_eq!(dc.label().await.unwrap_or_default(), A2A_CHANNEL_LABEL);
        let _ = p.pc.close().await;
        Ok(())
    }
}

