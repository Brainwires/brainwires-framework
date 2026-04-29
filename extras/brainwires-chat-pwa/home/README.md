# brainwires-home

Dial-home daemon for the [Brainwires chat PWA](../web/). Runs on the user's
own hardware and exposes a WebRTC peer + A2A JSON-RPC bridge into
[`brainwires-agents::TaskAgent`](../../../crates/brainwires-agents/), so the
PWA in a browser can talk to a powerful agent on the user's home machine
without paying anyone a per-token markup.

The PWA reaches this daemon via WebRTC behind a Cloudflare Tunnel (or
equivalent). The tunnel only ever forwards HTTPS signaling (`/signal/*`,
`/pair/*`, `/.well-known/agent-card.json`) — once the WebRTC peer is
negotiated, A2A traffic flows peer-to-peer over an SCTP DataChannel.

## Status

Phase 2 of the chat-PWA pivot. Milestones land directly on the version
branch.

| Milestone | What lands                                                              | Status     |
|-----------|--------------------------------------------------------------------------|------------|
| **M1**    | Crate scaffold, `webrtc-rs` dep, in-process two-peer ping/pong test     | this commit |
| M2        | Axum `/signal/*` endpoints, in-memory session map, agent-card JSON      | next       |
| M3        | Wire the WebRTC peer into the axum routes; JSON-RPC `ping` echo         | —          |
| M4        | A2A bridge — route inbound JSON-RPC into a real `TaskAgent`             | —          |
| M5–M6     | Browser-side dial-home + point PWA at the existing tunnel               | —          |
| M7        | Cloudflare Calls TURN credential minting (cellular symmetric-NAT path)  | —          |
| M8        | Pairing flow (`/pair/claim`, `/pair/confirm`, QR + 6-digit confirm)     | —          |
| M9–M12    | `home-provider.js`, reconnect/resume, multimodal chunking, polish       | —          |

## Architecture

```
+-----------------+       HTTPS (signaling)        +----------------------+
|  PWA (browser)  | ---> POST /signal/offer  --->  |  Cloudflare Tunnel   |
|  vanilla JS     | <--- GET  /signal/answer ----- |  cloudflared         |
|  RTCPeer...     | <--- GET  /signal/ice    ----- |  (trycloudflare or   |
+--------+--------+                                |   user-owned domain) |
         |                                         +----------+-----------+
         |  WebRTC SCTP DataChannel ("a2a")                   |
         |  (Cloudflare Calls TURN if needed)                 v
         |                                          +-------------------+
         +----------- ICE/DTLS/SCTP --------------> | brainwires-home   |
                                                    | axum :7878        |
                                                    | signaling+WebRTC  |
                                                    +---------+---------+
                                                              | A2A JSON-RPC
                                                              v
                                                  TaskAgent / AgentPool /
                                                  Providers / MCP / files
```

## Quickstart

```sh
cargo run -p brainwires-home -- --help
cargo run -p brainwires-home -- --bind 127.0.0.1:7878
```

The default bind is `127.0.0.1:7878` — the tunnel sits in front of it. The
daemon never needs to listen on a public interface; if it does, you've
mis-configured the tunnel.

## Endpoints (target shape — wired up in M2/M3/M8)

### Signaling

| Method | Path                        | Body / response                                     |
|--------|-----------------------------|-----------------------------------------------------|
| POST   | `/signal/session`           | → `{ session_id, ice_servers: [...] }`              |
| POST   | `/signal/offer/{session}`   | `{ sdp, type }` → `204`                             |
| GET    | `/signal/answer/{session}`  | long-poll 25 s → `{ sdp, type }`                    |
| POST   | `/signal/ice/{session}`     | `{ candidate }` → `204`                             |
| GET    | `/signal/ice/{session}?since=N` | long-poll → `{ candidates: [...], cursor }`     |
| DELETE | `/signal/{session}`         | → `204`                                             |

`ice_servers` includes a Cloudflare Calls TURN credential the daemon mints
server-side (~10 minute lifetime, refreshed on next session). The PWA never
holds the CF Calls API key.

### Well-known

| Method | Path                              | Description                          |
|--------|-----------------------------------|--------------------------------------|
| GET    | `/.well-known/agent-card.json`    | A2A agent card for discovery         |

### Pairing (M8)

| Method | Path              | Body / response                                     |
|--------|-------------------|-----------------------------------------------------|
| POST   | `/pair/claim`     | `{ one_time_token, device_pubkey, device_name }` → `204` |
| POST   | `/pair/confirm`   | `{ code }` → `{ cf_client_id, cf_client_secret, device_token, peer_pubkey }` |

## Data-channel protocol — A2A JSON-RPC over SCTP

Frame: `[u32 LE length][JSON bytes]`. SCTP is ordered + reliable; we do not
reinvent retransmit/sequence. Payloads are
[`brainwires-a2a`](../../../crates/brainwires-a2a/) `JsonRpcRequest` /
`JsonRpcResponse` / `message/stream` partial-result envelopes verbatim —
zero new schema. Streaming tokens are one frame per datachannel send.

Multimodal payloads >256 KB chunk via `bin/begin`, `bin/chunk`, `bin/end`
JSON-RPC pairs (transport concern, lives in `webrtc.rs`).

## Reconnect

15 s app-level ping. On `iceconnectionstate === "disconnected"` for >5 s,
the PWA initiates an ICE restart on the same signaling endpoints. A new
`session_id` is only minted on a second restart failure.

The home daemon keeps a bounded ring buffer of the last 64 messages by
JSON-RPC `id`. On reconnect the PWA sends `resume { last_seen_id }`; the
home replays. **Not** a durable queue — bounded only.

## Dev workflow

Run the daemon and a unit-test sweep:

```sh
cargo run -p brainwires-home -- --bind 127.0.0.1:7878
cargo test -p brainwires-home
```

The M1 unit test in `src/webrtc.rs` spins up two in-process WebRTC peers,
runs the offer/answer dance manually, opens the canonical `"a2a"` data
channel, and round-trips a ping/pong frame. Passing this test is the
gate to wiring the same peer into the axum signaling routes in M3.

For end-to-end PWA → home dev (M5+): point `web/src/home-signaling.js` at
`http://127.0.0.1:7878` and flip the dev toggle in the PWA Settings panel.

## Production

The home daemon expects a Cloudflare Tunnel (or any reverse tunnel that
lands on `127.0.0.1:7878`) to be running on the same host. The user
provides:

- `cloudflared` binary + tunnel credentials (one-time `cloudflared tunnel
  login` + `cloudflared tunnel create brainwires-home`),
- A hostname that maps to `http://127.0.0.1:7878` in the tunnel config,
- Cloudflare Access service-token pair, bound at pairing time, sent on
  every signaling request as `CF-Access-Client-Id` / `CF-Access-Client-Secret`,
- Optional Cloudflare Calls API token for TURN credential minting (M7).

The daemon also enforces its own `Authorization: Bearer <device_token>`
on every signaling request — defence in depth, so a leaked CF service
token alone cannot reach the agent.

## What this crate is not

- **Not** a relay or mesh node. One PWA ↔ one home daemon.
- **Not** a STUN/TURN server. The daemon mints CF Calls credentials and
  hands them to the PWA; it does not proxy media.
- **Not** a multi-tenant gateway. For that, see `brainclaw-gateway`. This
  daemon serves a single user's PWA(s).
- **Not** a durable agent host. Agents run in-process; if the daemon
  restarts the conversation reconnects (see Reconnect above), but in-flight
  requests beyond the bounded ring buffer are dropped.

## License

MIT OR Apache-2.0 — same as the workspace.
