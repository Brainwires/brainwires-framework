# brainwires-channels

[![Crates.io](https://img.shields.io/crates/v/brainwires-channels.svg)](https://crates.io/crates/brainwires-channels)
[![Documentation](https://img.shields.io/docsrs/brainwires-channels)](https://docs.rs/brainwires-channels)
[![License](https://img.shields.io/crates/l/brainwires-channels.svg)](LICENSE)

Universal messaging channel contract for the Brainwires Agent Framework.

## Overview

`brainwires-channels` defines the traits and types that every messaging channel adapter (Discord, Telegram, Slack, WhatsApp, etc.) must implement. It provides a consistent interface between the gateway daemon and channel adapters.

```text
Channel Adapters                    Gateway
┌──────────┐                    ┌──────────────┐
│ Discord  │──┐                 │              │
├──────────┤  │  ChannelEvent   │  SessionMgr  │
│ Telegram │──┼────────────────►│  Router      │
├──────────┤  │  ChannelMessage │  Admin API   │
│ Slack    │──┘◄────────────────│              │
└──────────┘                    └──────────────┘
     All implement                Uses these types
     Channel trait                for routing
```

## Core Types

| Type | Description |
|------|-------------|
| `Channel` trait | 7 async methods: send/edit/delete messages, typing, reactions, history |
| `ChannelMessage` | Rich message with text, markdown, media, embeds, attachments |
| `ChannelEvent` | Event variants: message received/edited/deleted, reactions, typing, presence, WebRTC |
| `ChannelCapabilities` | 14 bitflags: rich text, media, threads, reactions, voice, video, data channels, etc. |
| `ChannelUser` | Platform-agnostic user identity |
| `ConversationId` | Platform + channel + optional server ID |
| `ChannelSession` | Maps a channel user to an agent session |
| `ChannelHandshake` | Protocol for channel adapters connecting to the gateway |

## Usage

```rust
use brainwires_channels::{Channel, ChannelMessage, ChannelEvent, ChannelCapabilities};

// Implement the Channel trait for your platform
struct MyChannel;

#[async_trait]
impl Channel for MyChannel {
    fn channel_type(&self) -> &str { "my-platform" }
    fn capabilities(&self) -> ChannelCapabilities {
        ChannelCapabilities::RICH_TEXT | ChannelCapabilities::REACTIONS
    }
    // ... implement remaining methods
}
```

## Conversion

The crate provides `From`/`TryFrom` conversions between `ChannelMessage` and `MessageEnvelope` from `brainwires-agent-network`, enabling seamless integration with the framework's networking layer.

## WebRTC Real-Time Media (`webrtc` feature)

Add real-time voice and video to any channel adapter using the Brainwires fork of `webrtc-rs`.

```toml
brainwires-channels = { version = "...", features = ["webrtc"] }
# or with congestion control + jitter buffering:
brainwires-channels = { version = "...", features = ["webrtc-advanced"] }
```

### Session lifecycle

```rust
use brainwires_channels::{
    WebRtcSession, WebRtcConfig, AudioCodec, SdpType,
    BroadcastSignaling, WebRtcSignaling, SignalingMessage,
};
use std::sync::Arc;

let session = Arc::new(WebRtcSession::new(WebRtcConfig::default(), conv.clone()));
session.open().await?;

// Add tracks before creating the offer
let audio = session.add_audio_track(AudioCodec::Opus).await?;

// Offer/answer exchange (transport the SDP via your signaling impl)
let sdp = session.create_offer().await?;
signaling.send_signaling(&conv, SignalingMessage::Offer {
    session_id: session.id.clone(), sdp
}).await?;

// Apply remote answer when received
session.set_remote_description(answer_sdp, SdpType::Answer).await?;

// ICE candidates flow automatically via ChannelEvent::IceCandidate broadcasts
let mut events = session.subscribe();
// Forward them to the remote peer and call session.add_ice_candidate(...)

// Write encoded audio frames once connected
audio.write_sample(&opus_frame).await?;

// Read incoming remote tracks
if let Some(track) = session.get_remote_track(&track_id).await {
    while let Some(event) = track.poll().await {
        // handle TrackRemoteEvent::OnRtpPacket(pkt), OnEnded, etc.
    }
}

// Query RTCP stats
let stats = session.get_stats().await?;
for stream in stats.inbound_rtp_streams() {
    println!("jitter: {:.1}ms  lost: {}", stream.jitter * 1000.0, stream.packets_lost);
}

session.close().await?;
```

### Configuration

```rust
use brainwires_channels::{
    WebRtcConfig, IceServer, IceTransportPolicy, DtlsRole,
    AudioCodec, VideoCodec, CodecPreferences, BandwidthConstraints,
};

let config = WebRtcConfig {
    ice_servers: vec![
        IceServer { urls: vec!["stun:stun.l.google.com:19302".into()], ..Default::default() },
        IceServer {
            urls: vec!["turn:turn.example.com:3478".into()],
            username: Some("user".into()),
            credential: Some("pass".into()),
        },
    ],
    ice_transport_policy: IceTransportPolicy::All,
    dtls_role: DtlsRole::Auto,
    mdns_enabled: false,
    tcp_candidates_enabled: true,
    bind_addresses: vec!["0.0.0.0:0".into()],
    codec_preferences: CodecPreferences {
        audio: vec![AudioCodec::Opus],
        video: vec![VideoCodec::Vp8, VideoCodec::H264],
    },
    bandwidth: BandwidthConstraints { min_bps: 30_000, start_bps: 500_000, max_bps: 3_000_000 },
};
```

### `webrtc-advanced` — Congestion control & jitter buffering

Enabling the `webrtc-advanced` feature adds three interceptors to every session:

| Interceptor | Purpose |
|---|---|
| **JitterBuffer** | Adaptive playout delay; reorders out-of-sequence packets |
| **TwccSender** | Adds transport-wide sequence numbers to outgoing RTP |
| **GCC** | Consumes TWCC feedback to estimate available bandwidth |

```rust
// Query the GCC target bitrate and adapt your encoder
if let Some(bps) = session.target_bitrate_bps() {
    encoder.set_bitrate(bps);
}
```

`BandwidthConstraints` in `WebRtcConfig` sets the GCC min/start/max bounds.

### Events emitted by `WebRtcSession`

| Event | Fired when |
|---|---|
| `IceCandidate` | A local ICE candidate is gathered |
| `IceGatheringComplete` | All local candidates have been gathered |
| `SdpOffer` / `SdpAnswer` | Received via `ChannelMessageSignaling` injection |
| `TrackAdded` | Remote peer adds a media track |
| `TrackRemoved` | Remote track ends |
| `WebRtcDataChannel` | Message arrives on a DataChannel |
| `PeerConnectionStateChanged` | Overall connection state changes |
| `IceConnectionStateChanged` | ICE connection state changes |
| `SignalingStateChanged` | Offer/answer signaling state changes |

### RTCP Stats

`session.get_stats()` returns an `RTCStatsReport` with:
- `inbound_rtp_streams()` — jitter, packets_lost, NACK/PLI/FIR counts, jitter buffer delay, audio level, frame stats
- `outbound_rtp_streams()` — bytes/packets sent, retransmitted packets, target bitrate
- `candidate_pairs()` — `current_round_trip_time`, available bandwidth estimates
- `transport()`, `peer_connection()`, `data_channels()`
