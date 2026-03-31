# Brainwires Remote Communication Protocol Specification

**Version:** 1.0.0
**Status:** Draft
**Date:** December 2024
**Authors:** Brainwires Development Team

---

## Abstract

This document specifies the communication protocols used in the Brainwires distributed AI agent system. It covers agent-to-bridge IPC communication, bridge-to-backend WebSocket and HTTP protocols, authentication and encryption mechanisms, and proposes an extension for bridge-to-bridge mesh networking to enable multi-agent coordination across distributed systems.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [System Architecture](#2-system-architecture)
3. [Protocol Layers](#3-protocol-layers)
4. [Agent-to-Bridge IPC Protocol](#4-agent-to-bridge-ipc-protocol)
5. [Bridge-to-Backend Protocol](#5-bridge-to-backend-protocol)
6. [Supabase Realtime Protocol](#6-supabase-realtime-protocol)
7. [Authentication Mechanisms](#7-authentication-mechanisms)
8. [Encryption Specification](#8-encryption-specification)
9. [Message Formats](#9-message-formats)
10. [Connection Lifecycle](#10-connection-lifecycle)
11. [Error Handling](#11-error-handling)
12. [Proposed Improvements](#12-proposed-improvements)
13. [Bridge-to-Bridge Mesh Protocol (Proposed)](#13-bridge-to-bridge-mesh-protocol-proposed)
14. [Security Considerations](#14-security-considerations)
15. [References](#15-references)

---

## 1. Introduction

### 1.1 Purpose

The Brainwires Remote Communication Protocol enables:

- Real-time interaction with AI agents running on remote CLI instances
- Secure, encrypted communication between distributed components
- Multi-agent orchestration across heterogeneous computing environments
- Web-based monitoring and control of headless agent processes

### 1.2 Scope

This specification covers:

- **Layer 1:** Agent ↔ Bridge (Local IPC)
- **Layer 2:** Bridge ↔ Supabase Backend (WebSocket/HTTP)
- **Layer 3:** Frontend ↔ Supabase Realtime (WebSocket)
- **Layer 4:** Bridge ↔ Bridge Mesh (Proposed Extension)

### 1.3 Terminology

| Term | Definition |
|------|------------|
| **Agent** | An AI assistant process running in a TUI or headless mode |
| **Bridge** | A daemon process that connects local agents to the cloud backend |
| **Backend** | Supabase infrastructure (Database, Auth, Realtime, Edge Functions) |
| **Session** | A top-level agent without a parent (main conversation) |
| **Sub-agent** | A child agent spawned by another agent (has parent_id) |
| **Mesh** | A peer-to-peer network of interconnected bridges |

### 1.4 Design Goals

1. **Low Latency:** Sub-100ms message delivery for interactive use
2. **Security:** End-to-end encryption for sensitive data
3. **Resilience:** Automatic reconnection and graceful degradation
4. **Scalability:** Support for hundreds of concurrent agents per bridge
5. **Extensibility:** Protocol versioning and backward compatibility

---

## 2. System Architecture

### 2.1 Component Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           MACHINE A                                      │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                                 │
│  │ Agent 1 │  │ Agent 2 │  │ Agent 3 │     (AI Processes)              │
│  └────┬────┘  └────┬────┘  └────┬────┘                                 │
│       │            │            │                                       │
│       └────────────┼────────────┘                                       │
│                    │ Unix Domain Sockets (ChaCha20-Poly1305)           │
│              ┌─────┴─────┐                                              │
│              │  Bridge A │                                              │
│              └─────┬─────┘                                              │
└────────────────────┼────────────────────────────────────────────────────┘
                     │
                     │ WSS (TLS 1.3) + Supabase Realtime Protocol
                     │
┌────────────────────┼────────────────────────────────────────────────────┐
│                    │         SUPABASE BACKEND                           │
│              ┌─────┴─────┐                                              │
│              │ Realtime  │◄──────── WebSocket Multiplexer               │
│              │  Server   │                                              │
│              └─────┬─────┘                                              │
│                    │                                                    │
│    ┌───────────────┼───────────────┐                                   │
│    │               │               │                                    │
│ ┌──┴──┐      ┌─────┴─────┐   ┌─────┴─────┐                            │
│ │ Auth │      │ Database  │   │   Edge    │                            │
│ │(JWT) │      │(PostgreSQL│   │ Functions │                            │
│ └──────┘      └───────────┘   └───────────┘                            │
└─────────────────────────────────────────────────────────────────────────┘
                     │
                     │ WSS (TLS 1.3) + Supabase Realtime Protocol
                     │
┌────────────────────┼────────────────────────────────────────────────────┐
│              ┌─────┴─────┐         WEB FRONTEND                        │
│              │  Browser  │         (Next.js Application)               │
│              │  Client   │                                              │
│              └───────────┘                                              │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Data Flow Summary

| Flow | Transport | Encryption | Authentication |
|------|-----------|------------|----------------|
| Agent → Bridge | Unix Socket | ChaCha20-Poly1305 | Session Token |
| Bridge → Supabase | WSS | TLS 1.3 | JWT (Supabase) |
| Frontend → Supabase | WSS | TLS 1.3 | JWT (Supabase Auth) |
| Bridge ↔ Bridge (Proposed) | QUIC | TLS 1.3 + Noise | Mutual TLS + Ed25519 |

---

## 3. Protocol Layers

### 3.1 Layer Model

```
┌─────────────────────────────────────────────┐
│  Layer 4: Application Messages              │
│  (SendInput, SlashCommand, AgentStream)     │
├─────────────────────────────────────────────┤
│  Layer 3: Supabase Realtime Protocol        │
│  (Phoenix Channels, Broadcast, Presence)    │
├─────────────────────────────────────────────┤
│  Layer 2: WebSocket / HTTP                  │
│  (Connection Management, Heartbeat)         │
├─────────────────────────────────────────────┤
│  Layer 1: Transport Security                │
│  (TLS 1.3, ChaCha20-Poly1305)              │
├─────────────────────────────────────────────┤
│  Layer 0: Network Transport                 │
│  (TCP, Unix Sockets, QUIC)                  │
└─────────────────────────────────────────────┘
```

### 3.2 Message Encapsulation

Application messages are wrapped in multiple layers:

```
┌─────────────────────────────────────────────────────────────────┐
│ TLS Record                                                       │
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ WebSocket Frame                                              │ │
│ │ ┌─────────────────────────────────────────────────────────┐ │ │
│ │ │ Phoenix Channel Message                                  │ │ │
│ │ │ ┌─────────────────────────────────────────────────────┐ │ │ │
│ │ │ │ Supabase Broadcast Payload                          │ │ │ │
│ │ │ │ ┌─────────────────────────────────────────────────┐ │ │ │ │
│ │ │ │ │ Brainwires Application Message                  │ │ │ │ │
│ │ │ │ │ { type, id, payload, timestamp, userId }        │ │ │ │ │
│ │ │ │ └─────────────────────────────────────────────────┘ │ │ │ │
│ │ │ └─────────────────────────────────────────────────────┘ │ │ │
│ │ └─────────────────────────────────────────────────────────┘ │ │
│ └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Agent-to-Bridge IPC Protocol

### 4.1 Transport Layer

- **Socket Type:** Unix Domain Socket (Linux/macOS) or Named Pipe (Windows)
- **Location:** `$XDG_RUNTIME_DIR/brainwires/` or `/tmp/brainwires/`
- **File Pattern:** `agent-{session_id}.sock`
- **Permissions:** `0600` (owner read/write only)

### 4.2 Connection Handshake

```
Agent                                    Bridge
  │                                        │
  │──────── Connect to socket ────────────►│
  │                                        │
  │◄─────── Handshake Challenge ──────────│
  │         { nonce: [32 bytes] }          │
  │                                        │
  │──────── Handshake Response ───────────►│
  │         { session_token,               │
  │           signature: HMAC(nonce) }     │
  │                                        │
  │◄─────── Handshake Accept ─────────────│
  │         { status: "connected" }        │
  │                                        │
  │◄═══════ Encrypted Channel ════════════►│
```

### 4.3 Message Frame Format

All messages after handshake use the following frame format:

```
┌──────────────┬───────────────┬─────────────────────────────────┐
│ Nonce        │ Ciphertext    │ Authentication Tag              │
│ (12 bytes)   │ (variable)    │ (16 bytes, part of ciphertext)  │
└──────────────┴───────────────┴─────────────────────────────────┘
```

**Encryption Details:**
- **Algorithm:** ChaCha20-Poly1305 (RFC 8439)
- **Key Derivation:** `SHA256("brainwires-ipc-v1:" || session_token)`
- **Nonce:** Random 96-bit value, unique per message
- **Overhead:** 28 bytes per message (12 nonce + 16 auth tag)

### 4.4 IPC Message Types

#### 4.4.1 Bridge → Agent Messages (ViewerMessage)

```rust
enum ViewerMessage {
    /// User input from web interface
    UserInput {
        content: String,
        context_files: Vec<String>,
    },

    /// Slash command execution
    SlashCommand {
        command: String,
        args: Vec<String>,
    },

    /// Cancel current operation
    Cancel,

    /// Request conversation history sync
    SyncRequest,

    /// Ping for connection health
    Ping { timestamp: i64 },
}
```

#### 4.4.2 Agent → Bridge Messages (AgentMessage)

```rust
enum AgentMessage {
    /// Text output from assistant
    StreamChunk {
        content: String,
        chunk_type: StreamChunkType,
    },

    /// Stream completed
    StreamEnd {
        finish_reason: String,
    },

    /// Tool invocation
    ToolCall {
        tool_name: String,
        tool_id: String,
        input: serde_json::Value,
    },

    /// Tool execution result
    ToolResult {
        tool_id: String,
        output: String,
        is_error: bool,
    },

    /// State change notification
    StateChange {
        is_busy: bool,
        message_count: usize,
    },

    /// Conversation history (response to SyncRequest)
    History {
        messages: Vec<DisplayMessage>,
    },

    /// Pong response
    Pong { timestamp: i64 },

    /// Error notification
    Error {
        code: String,
        message: String,
    },
}
```

### 4.5 Stream Chunk Types

```rust
enum StreamChunkType {
    Text,        // Regular assistant text
    Thinking,    // Extended thinking/reasoning
    ToolCall,    // Tool invocation details
    ToolResult,  // Tool execution output
    Error,       // Error message
    System,      // System notification
    Complete,    // Stream finished marker
    History,     // Historical message (JSON array)
    UserInput,   // Echo of user input
}
```

---

## 5. Bridge-to-Backend Protocol

### 5.1 Connection Modes

The bridge supports two connection modes with automatic fallback:

| Mode | Transport | Latency | Use Case |
|------|-----------|---------|----------|
| **Realtime** | WebSocket (WSS) | ~50ms | Primary, bidirectional |
| **Polling** | HTTPS POST | ~500ms | Fallback, firewall-friendly |

### 5.2 Registration Flow

#### 5.2.1 Initial Registration (HTTP)

```http
POST /api/remote/connect HTTP/1.1
Host: brainwires.studio
Authorization: Bearer bw_prod_a1b2c3d4e5f6...
Content-Type: application/json

{
  "hostname": "developer-laptop",
  "os": "linux",
  "version": "0.5.0"
}
```

#### 5.2.2 Registration Response

```json
{
  "type": "authenticated",
  "session_token": "st_7f8e9d0c1b2a3456...",
  "user_id": "usr_abc123",
  "refresh_interval_secs": 30,
  "use_realtime": true,
  "realtime_token": "eyJhbGciOiJIUzI1NiIs...",
  "realtime_url": "wss://realtime.supabase.co/v1/websocket",
  "channel_name": "cli:usr_abc123",
  "supabase_anon_key": "eyJhbGciOiJIUzI1NiIs..."
}
```

### 5.3 Heartbeat Protocol (Polling Mode)

```http
POST /api/remote/heartbeat HTTP/1.1
Host: brainwires.studio
Content-Type: application/json

{
  "session_token": "st_7f8e9d0c1b2a3456...",
  "agents": [
    {
      "session_id": "session-abc123",
      "model": "claude-sonnet-4-20250514",
      "is_busy": false,
      "parent_id": null,
      "working_directory": "/home/user/project",
      "message_count": 42,
      "last_activity": 1703980800000,
      "status": "idle",
      "name": "Code Review Agent"
    }
  ],
  "system_load": 0.35,
  "messages": [
    {
      "type": "command_result",
      "command_id": "cmd_xyz789",
      "success": true,
      "result": { "message_sent": true }
    }
  ]
}
```

#### Heartbeat Response

```json
{
  "success": true,
  "commands": [
    {
      "type": "send_input",
      "command_id": "cmd_new123",
      "agent_id": "session-abc123",
      "content": "Explain this function"
    }
  ],
  "timestamp": 1703980805000
}
```

### 5.4 Backend Command Types

```typescript
type BackendCommand =
  | { type: "authenticated"; session_token: string; user_id: string; refresh_interval_secs: number }
  | { type: "send_input"; command_id: string; agent_id: string; content: string }
  | { type: "slash_command"; command_id: string; agent_id: string; command: string; args: string[] }
  | { type: "cancel_operation"; command_id: string; agent_id: string }
  | { type: "subscribe"; agent_id: string }
  | { type: "unsubscribe"; agent_id: string }
  | { type: "spawn_agent"; command_id: string; model?: string; working_directory?: string }
  | { type: "request_sync" }
  | { type: "ping"; timestamp: number }
  | { type: "disconnect"; reason: string }
  | { type: "authentication_failed"; error: string }
```

### 5.5 CLI Message Types

```typescript
type RemoteMessage =
  | { type: "register"; api_key: string; hostname: string; os: string; version: string }
  | { type: "heartbeat"; session_token: string; agents: RemoteAgentInfo[]; system_load: number }
  | { type: "command_result"; command_id: string; success: boolean; result?: any; error?: string }
  | { type: "agent_event"; event_type: AgentEventType; agent_id: string; data: any }
  | { type: "agent_stream"; agent_id: string; chunk_type: StreamChunkType; content: string }
  | { type: "pong"; timestamp: number }
```

---

## 6. Supabase Realtime Protocol

### 6.1 Protocol Overview

Supabase Realtime is built on Phoenix Channels, providing:

- **Broadcast:** Pub/sub messaging to channel subscribers
- **Presence:** Track online users and their state
- **Postgres Changes:** Real-time database change notifications

Brainwires uses the **Broadcast** feature for CLI ↔ Frontend communication.

### 6.2 WebSocket Connection

```
wss://realtime.supabase.co/v1/websocket?apikey={anon_key}&vsn=1.0.0
```

**Headers:**
```
Authorization: Bearer {jwt_token}
```

### 6.3 Phoenix Channel Protocol

#### 6.3.1 Join Channel

```json
{
  "topic": "realtime:cli:usr_abc123",
  "event": "phx_join",
  "payload": {
    "config": {
      "broadcast": { "self": false },
      "presence": { "key": "" }
    }
  },
  "ref": "1"
}
```

#### 6.3.2 Join Reply

```json
{
  "topic": "realtime:cli:usr_abc123",
  "event": "phx_reply",
  "payload": {
    "status": "ok",
    "response": {}
  },
  "ref": "1"
}
```

#### 6.3.3 Heartbeat (Phoenix)

Every 25 seconds to maintain connection:

```json
{
  "topic": "phoenix",
  "event": "heartbeat",
  "payload": {},
  "ref": "2"
}
```

### 6.4 Brainwires Message Wrapping

Application messages are wrapped in Supabase broadcast format:

```json
{
  "topic": "realtime:cli:usr_abc123",
  "event": "broadcast",
  "payload": {
    "type": "broadcast",
    "event": "remote",
    "payload": {
      "type": "remote.heartbeat",
      "id": "msg_uuid_123",
      "payload": {
        "agents": [...],
        "systemLoad": 0.35,
        "hostname": "developer-laptop"
      },
      "timestamp": 1703980800000,
      "userId": "usr_abc123"
    }
  },
  "ref": "3"
}
```

### 6.5 Realtime Message Types

| Type | Direction | Purpose |
|------|-----------|---------|
| `remote.register` | CLI → Backend | Initial registration with agent list |
| `remote.heartbeat` | CLI → Backend | Periodic status update |
| `remote.stream` | CLI → Frontend | Agent output streaming |
| `remote.command_result` | CLI → Backend | Command execution result |
| `remote.event` | CLI → Frontend | Agent lifecycle events |
| `remote.command` | Backend → CLI | Command to execute |
| `remote.ping` | Backend → CLI | Connection health check |
| `remote.pong` | CLI → Backend | Ping response |
| `remote.disconnect` | CLI → Backend | Graceful shutdown notification |

### 6.6 Payload Structures

#### RemoteRegisterPayload

```typescript
interface RemoteRegisterPayload {
  hostname: string
  os: string
  version: string
  sessionToken: string
  agents?: RemoteAgentInfo[]
  systemLoad?: number
}
```

#### RemoteHeartbeatPayload

```typescript
interface RemoteHeartbeatPayload {
  agents: RemoteAgentInfo[]
  systemLoad: number
  hostname?: string
  os?: string
  version?: string
}
```

#### RemoteStreamPayload

```typescript
interface RemoteStreamPayload {
  agentId: string
  chunkType: StreamChunkType
  content: string
}
```

#### RemoteCommandPayload

```typescript
interface RemoteCommandPayload {
  commandId: string
  commandType: string
  agentId?: string
  content?: string
  command?: string
  args?: string[]
  model?: string
  workingDirectory?: string
  reason?: string
}
```

---

## 7. Authentication Mechanisms

### 7.1 API Key System

#### 7.1.1 Key Format

```
bw_{environment}_{32_hex_chars}

Examples:
- bw_prod_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
- bw_dev_x9y8z7w6v5u4t3s2r1q0p9o8n7m6l5k4
```

**Environment Prefixes:**
| Prefix | Backend URL |
|--------|-------------|
| `bw_prod_` | https://brainwires.studio |
| `bw_dev_` | https://dev.brainwires.net |
| `bw_test_` | http://localhost:3000 |

#### 7.1.2 Key Storage

- **Primary:** System keyring (GNOME Keyring, macOS Keychain, Windows Credential Manager)
- **Fallback:** Encrypted file in `~/.config/brainwires-cli/`
- **Memory:** Zeroized immediately after use

#### 7.1.3 Key Verification

```
1. Extract key from Authorization header
2. Validate format regex: /^bw_(prod|dev|test)_[a-z0-9]{32}$/
3. Query cli_api_keys table for active keys by user
4. bcrypt.compare(api_key, stored_hash)
5. Check expiration: expires_at > now OR expires_at IS NULL
6. Update last_used_at timestamp
7. Return user_id for session binding
```

### 7.2 Session Token System

- **Generation:** UUID v4 on successful registration
- **Lifetime:** Valid until explicit disconnect or timeout (2 minutes no heartbeat)
- **Usage:** Included in every heartbeat for stateless authentication
- **Storage:** In-memory only (not persisted)

### 7.3 Supabase JWT

#### 7.3.1 Token Generation

```typescript
const token = jwt.sign(
  {
    iss: "brainwires-cli",
    sub: userId,
    aud: "authenticated",
    role: "authenticated",
    cli_session: sessionToken,
    is_cli: true,
    iat: Math.floor(Date.now() / 1000),
    exp: Math.floor(Date.now() / 1000) + 86400, // 24 hours
  },
  process.env.SUPABASE_JWT_SECRET,
  { algorithm: "HS256" }
)
```

#### 7.3.2 Token Claims

| Claim | Type | Purpose |
|-------|------|---------|
| `iss` | string | Issuer identifier ("brainwires-cli") |
| `sub` | string | User ID (maps to auth.uid() in RLS) |
| `aud` | string | Audience ("authenticated") |
| `role` | string | Supabase role ("authenticated") |
| `cli_session` | string | Session token for tracking |
| `is_cli` | boolean | Distinguish CLI from web clients |
| `iat` | number | Issued at timestamp |
| `exp` | number | Expiration timestamp (24h) |

### 7.4 Authentication Flow Diagram

```
┌─────────┐          ┌──────────┐          ┌───────────┐
│   CLI   │          │ Next.js  │          │ Supabase  │
│ Bridge  │          │   API    │          │           │
└────┬────┘          └────┬─────┘          └─────┬─────┘
     │                    │                      │
     │ POST /api/remote/connect                  │
     │ Authorization: Bearer bw_prod_xxx         │
     │───────────────────►│                      │
     │                    │                      │
     │                    │ SELECT * FROM cli_api_keys
     │                    │ WHERE is_active = true
     │                    │─────────────────────►│
     │                    │                      │
     │                    │◄─────────────────────│
     │                    │ [{ key_hash, user_id }]
     │                    │                      │
     │                    │ bcrypt.compare()     │
     │                    │ Generate JWT         │
     │                    │                      │
     │◄───────────────────│                      │
     │ { session_token,   │                      │
     │   realtime_token,  │                      │
     │   channel_name }   │                      │
     │                    │                      │
     │ WSS Connect        │                      │
     │ Authorization: Bearer {realtime_token}    │
     │──────────────────────────────────────────►│
     │                    │                      │
     │ phx_join cli:user_id                      │
     │──────────────────────────────────────────►│
     │                    │                      │
     │◄──────────────────────────────────────────│
     │ phx_reply { status: "ok" }                │
     │                    │                      │
```

---

## 8. Encryption Specification

### 8.1 Transport Layer Security

All network communication uses TLS 1.3:

| Connection | Protocol | Cipher Suites |
|------------|----------|---------------|
| HTTPS API | TLS 1.3 | TLS_AES_256_GCM_SHA384, TLS_CHACHA20_POLY1305_SHA256 |
| WSS Realtime | TLS 1.3 | TLS_AES_256_GCM_SHA384, TLS_CHACHA20_POLY1305_SHA256 |

### 8.2 IPC Encryption (Agent ↔ Bridge)

#### 8.2.1 Algorithm Selection

**ChaCha20-Poly1305** was chosen for IPC encryption because:

1. **Performance:** Faster than AES on systems without AES-NI
2. **Security:** 256-bit key, 96-bit nonce, resistant to timing attacks
3. **Simplicity:** Combined AEAD construction reduces implementation errors
4. **Compatibility:** Available in all major crypto libraries

#### 8.2.2 Key Derivation

```rust
fn derive_key_from_token(token: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"brainwires-ipc-v1:");  // Domain separator
    hasher.update(token.as_bytes());
    hasher.finalize().into()
}
```

**Domain Separator:** Prevents key reuse if same token used elsewhere.

#### 8.2.3 Message Encryption

```rust
fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = rand::random::<[u8; 12]>();  // 96-bit random nonce

    let ciphertext = cipher
        .encrypt(nonce.into(), plaintext)
        .expect("encryption failure");

    // Output: nonce || ciphertext || auth_tag
    [nonce.as_slice(), ciphertext.as_slice()].concat()
}
```

#### 8.2.4 Message Decryption

```rust
fn decrypt(encrypted: &[u8], key: &[u8; 32]) -> Result<Vec<u8>> {
    if encrypted.len() < 28 {  // 12 nonce + 16 tag minimum
        return Err(Error::InvalidMessage);
    }

    let cipher = ChaCha20Poly1305::new(key.into());
    let (nonce, ciphertext) = encrypted.split_at(12);

    cipher
        .decrypt(nonce.into(), ciphertext)
        .map_err(|_| Error::DecryptionFailed)
}
```

### 8.3 Data at Rest

| Data Type | Storage | Encryption |
|-----------|---------|------------|
| API Keys | Database | bcrypt hash (cost=12) |
| Session Tokens | Memory only | None (ephemeral) |
| Config Files | Filesystem | None (permissions-based) |
| Keyring Secrets | System Keyring | OS-provided encryption |

---

## 9. Message Formats

### 9.1 RemoteAgentInfo Structure

```typescript
interface RemoteAgentInfo {
  /** Unique session identifier */
  session_id: string

  /** AI model identifier (e.g., "claude-sonnet-4-20250514") */
  model: string

  /** Whether agent is currently processing */
  is_busy: boolean

  /** Parent session ID for sub-agents (null for main sessions) */
  parent_id: string | null

  /** Agent's working directory path */
  working_directory: string

  /** Number of messages in conversation */
  message_count: number

  /** Unix timestamp of last activity (milliseconds) */
  last_activity: number

  /** Human-readable status ("idle", "busy", "error") */
  status: string

  /** User-assigned name (optional) */
  name?: string
}
```

### 9.2 DisplayMessage Structure

```typescript
interface DisplayMessage {
  /** Message role ("user", "assistant", "system", "tool") */
  role: string

  /** Message content (text or JSON for tool messages) */
  content: string

  /** Creation timestamp (Unix milliseconds) */
  created_at: number
}
```

### 9.3 BridgeInfo Structure (Frontend)

```typescript
interface BridgeInfo {
  /** Unique bridge identifier (hostname-based) */
  id: string

  /** Machine hostname */
  hostname: string

  /** Operating system */
  os: string

  /** CLI version */
  version: string

  /** Normalized CPU load (0.0 - 1.0) */
  systemLoad: number

  /** Connection established timestamp */
  connectedAt: number

  /** Last heartbeat received timestamp */
  lastHeartbeat: number

  /** Current connection status */
  isConnected: boolean

  /** Agents running on this bridge */
  agents: RemoteAgentInfo[]
}
```

### 9.4 Agent Event Types

```typescript
type AgentEventType =
  | "spawned"           // New agent started
  | "exited"            // Agent process ended
  | "busy"              // Agent started processing
  | "idle"              // Agent finished processing
  | "state_changed"     // Generic state update
  | "viewer_connected"  // Web client subscribed
  | "viewer_disconnected" // Web client unsubscribed
```

---

## 10. Connection Lifecycle

### 10.1 Bridge Startup Sequence

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. Load Configuration                                            │
│    - Read config.json                                           │
│    - Retrieve API key from keyring                              │
│    - Determine backend URL from key prefix                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. Register with Backend                                         │
│    - POST /api/remote/connect                                   │
│    - Receive session_token and realtime credentials             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. Establish Realtime Connection                                 │
│    - Connect to WSS endpoint                                    │
│    - Join channel cli:{userId}                                  │
│    - Start Phoenix heartbeat (25s)                              │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. Send Initial Registration                                     │
│    - remote.register with current agent list                    │
│    - Includes system info (hostname, OS, version)               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. Enter Main Loop                                               │
│    - Collect agent status periodically (5s)                     │
│    - Send heartbeats via Realtime                               │
│    - Listen for commands                                        │
│    - Relay commands to agents via IPC                           │
│    - Stream agent output back                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 10.2 Disconnection Handling

#### 10.2.1 Graceful Shutdown

```
1. Bridge receives SIGTERM/SIGINT
2. Send remote.disconnect with reason
3. Close all agent IPC connections
4. Unsubscribe from Realtime channel
5. Close WebSocket connection
6. Exit process
```

#### 10.2.2 Unexpected Disconnection

```
1. WebSocket connection lost
2. Backend detects via missing heartbeat (30s timeout)
3. Frontend detects via missing heartbeat (30s timeout)
4. Bridge marks as disconnected in UI
5. Bridge attempts reconnection after reconnect_delay_secs (5s)
6. If reconnection fails, retry with exponential backoff
7. After max_reconnect_attempts (0=unlimited), give up
```

### 10.3 Timeout Configuration

| Timeout | Duration | Purpose |
|---------|----------|---------|
| Phoenix Heartbeat | 25s | WebSocket keep-alive |
| Brainwires Heartbeat | 5-30s | Agent status sync |
| Disconnect Detection | 30s | Mark bridge offline |
| Stale Bridge Cleanup | 5m | Remove from UI |
| Command Timeout | 30s | Fail pending commands |
| HTTP Request Timeout | 30s | API call limit |

---

## 11. Error Handling

### 11.1 Error Categories

| Category | Code Range | Recovery |
|----------|------------|----------|
| Authentication | 401-403 | Re-authenticate |
| Rate Limiting | 429 | Exponential backoff |
| Server Error | 500-503 | Retry with backoff |
| Network Error | - | Reconnect |
| Protocol Error | - | Log and continue |

### 11.2 Error Response Format

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Too many requests. Please wait before retrying.",
    "details": {
      "retry_after": 60,
      "limit": 100,
      "remaining": 0
    }
  }
}
```

### 11.3 Reconnection Strategy

```
attempt = 0
base_delay = 5 seconds
max_delay = 300 seconds

while should_reconnect:
    delay = min(base_delay * (2 ^ attempt), max_delay)
    delay += random(0, delay * 0.1)  # Jitter

    wait(delay)

    if connect_successful:
        attempt = 0
        break
    else:
        attempt += 1
```

---

## 12. Proposed Improvements

### 12.1 Protocol Versioning

**Current Issue:** No version negotiation in protocol.

**Proposed Solution:**

```typescript
interface ProtocolNegotiation {
  type: "protocol_hello"
  supported_versions: ["1.0", "1.1", "2.0"]
  preferred_version: "2.0"
  capabilities: ["streaming", "tools", "presence", "mesh"]
}

interface ProtocolAccept {
  type: "protocol_accept"
  selected_version: "1.1"
  enabled_capabilities: ["streaming", "tools"]
}
```

### 12.2 Message Compression

**Current Issue:** Large history syncs can be bandwidth-intensive.

**Proposed Solution:**

```typescript
interface CompressedMessage {
  type: "compressed"
  algorithm: "zstd" | "lz4" | "gzip"
  original_size: number
  compressed_data: string  // Base64 encoded
}
```

**Trigger:** Messages > 10KB automatically compressed.

### 12.3 Binary Protocol Option

**Current Issue:** JSON overhead for high-frequency streaming.

**Proposed Solution:** MessagePack or Protocol Buffers for stream chunks.

```
Header (4 bytes):
  [0-1]: Message type (uint16)
  [2-3]: Payload length (uint16)

Payload (variable):
  MessagePack-encoded data
```

**Estimated Savings:** 30-50% reduction in bandwidth for streaming.

### 12.4 End-to-End Encryption

**Current Issue:** Stream content visible to Supabase infrastructure.

**Proposed Solution:** Optional E2EE layer using Signal Protocol.

```typescript
interface E2EEEnvelope {
  type: "e2ee_message"
  sender_key_id: string
  ephemeral_public_key: string  // X25519
  ciphertext: string            // XChaCha20-Poly1305
  signature: string             // Ed25519
}
```

### 12.5 Presence Tracking

**Current Issue:** No visibility into who's viewing an agent.

**Proposed Solution:** Utilize Supabase Presence feature.

```typescript
// Join with presence
channel.track({
  user_id: userId,
  client_type: "web" | "cli",
  viewing_agent: agentId,
  online_at: new Date().toISOString()
})

// Sync presence state
channel.on("presence", { event: "sync" }, () => {
  const state = channel.presenceState()
  // { user_abc: [{ client_type: "web", viewing_agent: "agent_123" }] }
})
```

### 12.6 Command Queuing with Priority

**Current Issue:** All commands treated equally.

**Proposed Solution:**

```typescript
interface PrioritizedCommand extends BackendCommand {
  priority: "critical" | "high" | "normal" | "low"
  deadline_ms?: number  // Max time to wait for execution
  retry_policy?: {
    max_attempts: number
    backoff_multiplier: number
  }
}
```

### 12.7 Structured Logging and Telemetry

**Current Issue:** Limited observability into protocol issues.

**Proposed Solution:**

```typescript
interface ProtocolTelemetry {
  // Latency tracking
  message_latency_ms: Histogram
  command_roundtrip_ms: Histogram

  // Reliability metrics
  messages_sent: Counter
  messages_failed: Counter
  reconnection_count: Counter

  // Bandwidth tracking
  bytes_sent: Counter
  bytes_received: Counter
  compression_ratio: Gauge
}
```

---

## 13. Bridge-to-Bridge Mesh Protocol (Proposed)

### 13.1 Overview

The Bridge-to-Bridge Mesh Protocol extends Brainwires to support direct peer-to-peer communication between bridges, enabling:

- **Multi-Agent Coordination:** Agents on different machines collaborating on tasks
- **Resource Sharing:** Offload work to less-loaded bridges
- **Fault Tolerance:** Continue operation if backend temporarily unavailable
- **Local-First:** Reduced latency for co-located bridges

### 13.2 Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           MESH NETWORK                                   │
│                                                                         │
│    ┌──────────┐         QUIC/TLS 1.3           ┌──────────┐            │
│    │ Bridge A │◄───────────────────────────────►│ Bridge B │            │
│    │ (laptop) │                                 │ (server) │            │
│    └────┬─────┘                                 └────┬─────┘            │
│         │                                            │                  │
│         │              ┌──────────┐                  │                  │
│         └─────────────►│ Bridge C │◄─────────────────┘                  │
│                        │ (cloud)  │                                     │
│                        └────┬─────┘                                     │
│                             │                                           │
└─────────────────────────────┼───────────────────────────────────────────┘
                              │
                              │ WSS to Supabase (Coordination)
                              ▼
                     ┌────────────────┐
                     │   Supabase     │
                     │   Backend      │
                     └────────────────┘
```

### 13.3 Discovery Mechanisms

#### 13.3.1 Backend-Mediated Discovery

```typescript
// Bridge announces mesh capability to backend
interface MeshAnnouncement {
  type: "mesh.announce"
  bridge_id: string
  mesh_enabled: true
  public_key: string        // Ed25519 public key
  endpoints: MeshEndpoint[]
  capabilities: MeshCapability[]
}

interface MeshEndpoint {
  type: "direct" | "relay" | "stun"
  address: string           // IP:port or relay URL
  priority: number          // Lower = preferred
}

type MeshCapability =
  | "agent_relay"           // Can relay agent messages
  | "task_delegation"       // Can accept delegated tasks
  | "resource_sharing"      // Can share compute resources
  | "storage_sync"          // Can sync conversation storage
```

#### 13.3.2 Local Network Discovery (mDNS)

```
Service: _brainwires-mesh._udp.local
TXT Records:
  - version=1.0
  - pubkey=<base64 Ed25519 public key>
  - user_hash=<SHA256(user_id)[:16]>  // For same-user discovery
```

#### 13.3.3 DHT-Based Discovery (Optional)

For internet-wide peer discovery without backend:

- **Protocol:** Kademlia DHT
- **Bootstrap:** Well-known nodes or backend-provided list
- **Key:** SHA256(user_id || mesh_group_id)
- **Value:** Encrypted peer info (decryptable only by group members)

### 13.4 Connection Establishment

#### 13.4.1 Transport Selection

| Priority | Transport | Use Case |
|----------|-----------|----------|
| 1 | QUIC (UDP) | Primary, fast handshake |
| 2 | TCP | Fallback for UDP-blocked networks |
| 3 | WebSocket Relay | Through Supabase when direct fails |

#### 13.4.2 QUIC Configuration

```rust
struct MeshQuicConfig {
    // Connection settings
    idle_timeout: Duration::from_secs(30),
    keep_alive_interval: Duration::from_secs(10),
    max_streams: 100,

    // Security
    certificate: rustls::Certificate,  // Self-signed, verified via public key
    private_key: rustls::PrivateKey,

    // Performance
    initial_rtt: Duration::from_millis(100),
    max_udp_payload_size: 1350,
}
```

#### 13.4.3 Handshake Protocol

```
Bridge A                                Bridge B
    │                                       │
    │─────── QUIC ClientHello ─────────────►│
    │◄────── QUIC ServerHello ──────────────│
    │                                       │
    │ (TLS 1.3 Handshake with self-signed certs)
    │                                       │
    │─────── MeshHello ────────────────────►│
    │        { version, public_key,         │
    │          nonce, timestamp }           │
    │                                       │
    │◄────── MeshHelloReply ────────────────│
    │        { version, public_key,         │
    │          nonce, timestamp,            │
    │          signature }                  │
    │                                       │
    │─────── MeshAuth ─────────────────────►│
    │        { signature,                   │
    │          user_proof }                 │
    │                                       │
    │◄────── MeshAuthOk ────────────────────│
    │        { session_id }                 │
    │                                       │
    │◄═══════ Mesh Channel Open ═══════════►│
```

#### 13.4.4 Authentication

**Mutual Authentication:**
1. Both peers verify each other's Ed25519 public key
2. Signatures over (nonce_a || nonce_b || timestamp) prove key possession
3. User proof (signed by backend) proves authorization

**User Proof Token:**
```typescript
interface UserProofToken {
  user_id: string
  bridge_id: string
  public_key: string
  issued_at: number
  expires_at: number
  permissions: MeshPermission[]
  signature: string  // Signed by backend private key
}
```

### 13.5 Mesh Message Protocol

#### 13.5.1 Message Types

```rust
enum MeshMessage {
    // Discovery & Presence
    Ping { timestamp: u64 },
    Pong { timestamp: u64, load: f32 },
    PeerList { peers: Vec<PeerInfo> },

    // Agent Routing
    AgentQuery { agent_id: String },
    AgentLocation { agent_id: String, bridge_id: String },

    // Task Delegation
    TaskRequest {
        task_id: String,
        task_type: TaskType,
        payload: Vec<u8>,
        deadline: Option<u64>,
    },
    TaskAccept { task_id: String },
    TaskReject { task_id: String, reason: String },
    TaskProgress { task_id: String, progress: f32 },
    TaskComplete { task_id: String, result: Vec<u8> },
    TaskFailed { task_id: String, error: String },

    // Agent-to-Agent Communication
    AgentMessage {
        from_agent: String,
        to_agent: String,
        content: AgentMessageContent,
    },

    // Stream Forwarding
    StreamForward {
        agent_id: String,
        chunk: StreamChunk,
    },

    // Coordination
    LockRequest { resource: String, timeout: u64 },
    LockGrant { resource: String, token: String },
    LockRelease { resource: String, token: String },
}
```

#### 13.5.2 Agent-to-Agent Message Content

```rust
enum AgentMessageContent {
    // Information sharing
    ContextShare {
        files: Vec<FileRef>,
        summary: String,
    },

    // Task coordination
    SubtaskAssignment {
        description: String,
        context: String,
        expected_output: String,
    },
    SubtaskResult {
        success: bool,
        output: String,
        artifacts: Vec<Artifact>,
    },

    // Queries
    KnowledgeQuery {
        question: String,
        domain: Option<String>,
    },
    KnowledgeResponse {
        answer: String,
        confidence: f32,
        sources: Vec<String>,
    },

    // Status
    StatusUpdate {
        status: AgentStatus,
        current_task: Option<String>,
    },
}
```

### 13.6 Routing and Topology

#### 13.6.1 Routing Table

Each bridge maintains a routing table:

```rust
struct RoutingTable {
    // Direct connections
    direct_peers: HashMap<BridgeId, PeerConnection>,

    // Known bridges (may require hopping)
    known_bridges: HashMap<BridgeId, RouteInfo>,

    // Agent locations
    agent_locations: HashMap<AgentId, BridgeId>,
}

struct RouteInfo {
    next_hop: BridgeId,
    hop_count: u8,
    latency_ms: u32,
    last_updated: Instant,
}
```

#### 13.6.2 Routing Algorithm

```
1. Check if destination is directly connected
2. If not, lookup in routing table
3. If route exists and fresh (< 60s), use it
4. If stale or missing, broadcast AgentQuery to direct peers
5. Peers respond with AgentLocation if they know
6. Cache route for future use
7. If no route found, relay via backend
```

### 13.7 Security Model

#### 13.7.1 Threat Model

| Threat | Mitigation |
|--------|------------|
| Unauthorized peer connection | Ed25519 mutual authentication |
| Man-in-the-middle | TLS 1.3 + public key pinning |
| Replay attacks | Nonces + timestamps |
| Rogue bridge impersonation | User proof tokens signed by backend |
| Message tampering | AEAD encryption |
| Traffic analysis | Padding + dummy traffic (optional) |

#### 13.7.2 Trust Hierarchy

```
┌─────────────────────────────────────────────┐
│           Backend (Root of Trust)            │
│  - Issues user proof tokens                  │
│  - Publishes bridge public keys              │
│  - Provides revocation list                  │
└─────────────────────────────────────────────┘
                    │
                    │ Signs
                    ▼
┌─────────────────────────────────────────────┐
│           User Proof Token                   │
│  - Binds bridge_id to user_id               │
│  - Contains bridge public key                │
│  - Has expiration time                       │
│  - Lists granted permissions                 │
└─────────────────────────────────────────────┘
                    │
                    │ Authorizes
                    ▼
┌─────────────────────────────────────────────┐
│           Mesh Connection                    │
│  - Mutually authenticated                    │
│  - Encrypted channel                         │
│  - Permission-scoped operations              │
└─────────────────────────────────────────────┘
```

#### 13.7.3 Permission Model

```rust
enum MeshPermission {
    // Can communicate with specific user's bridges
    ConnectToUser { user_id: String },

    // Can accept delegated tasks
    AcceptTasks,

    // Can delegate tasks to this bridge
    DelegateTasks,

    // Can route messages through this bridge
    RelayMessages,

    // Can access shared storage
    AccessStorage { scope: StorageScope },

    // Can spawn agents on this bridge
    SpawnAgents,
}
```

### 13.8 Failure Handling

#### 13.8.1 Peer Failure Detection

```rust
struct PeerHealthChecker {
    ping_interval: Duration::from_secs(10),
    ping_timeout: Duration::from_secs(5),
    max_missed_pings: 3,
}

async fn check_peer_health(&mut self, peer: &mut PeerConnection) {
    match peer.ping().await {
        Ok(latency) => {
            peer.missed_pings = 0;
            peer.last_latency = latency;
        }
        Err(_) => {
            peer.missed_pings += 1;
            if peer.missed_pings >= self.max_missed_pings {
                self.handle_peer_failure(peer).await;
            }
        }
    }
}
```

#### 13.8.2 Graceful Degradation

```
1. Mesh peer unreachable
   → Route via alternative peer
   → Fall back to backend relay

2. Backend unreachable
   → Continue mesh operations locally
   → Queue backend-bound messages
   → Retry with exponential backoff

3. All peers unreachable
   → Operate in standalone mode
   → Attempt reconnection periodically
```

### 13.9 Use Cases

#### 13.9.1 Distributed Code Review

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Agent A    │     │  Agent B    │     │  Agent C    │
│ (Analysis)  │     │ (Security)  │     │ (Style)     │
│ Bridge 1    │     │ Bridge 2    │     │ Bridge 1    │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │
       │    Mesh Protocol  │                   │
       ├───────────────────┼───────────────────┤
       │                   │                   │
       ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────┐
│                 Coordinator Agent                    │
│  - Distributes code sections                         │
│  - Collects and merges reviews                       │
│  - Resolves conflicts                                │
└─────────────────────────────────────────────────────┘
```

#### 13.9.2 Parallel Task Execution

```typescript
// Coordinator agent spawns subtasks across mesh
const results = await Promise.all([
  mesh.delegateTask(bridge_a, { type: "analyze", data: chunk1 }),
  mesh.delegateTask(bridge_b, { type: "analyze", data: chunk2 }),
  mesh.delegateTask(bridge_c, { type: "analyze", data: chunk3 }),
]);

const merged = await agent.process("merge_results", results);
```

#### 13.9.3 Knowledge Sharing

```
Agent A (researching): "What authentication patterns does our codebase use?"

   → Broadcasts KnowledgeQuery to mesh

Agent B (different machine, indexed same codebase):
   → Responds with KnowledgeResponse from local RAG index

Agent A:
   → Uses response to inform its analysis
   → Avoids redundant indexing
```

---

## 14. Security Considerations

### 14.1 Current Security Properties

| Property | Implementation | Status |
|----------|---------------|--------|
| Authentication | API keys (bcrypt), JWT | ✓ Implemented |
| Transport Encryption | TLS 1.3 | ✓ Implemented |
| IPC Encryption | ChaCha20-Poly1305 | ✓ Implemented |
| Authorization | User-scoped sessions | ✓ Implemented |
| Rate Limiting | Per-user limits | ✓ Implemented |
| Audit Logging | Command logging | ✓ Implemented |

### 14.2 Known Limitations

1. **No E2EE for Stream Content:** Supabase can theoretically read stream data
2. **Single Point of Trust:** Backend compromise affects all users
3. **API Key Exposure:** Keys in memory could be extracted
4. **No Certificate Pinning:** Vulnerable to CA compromise

### 14.3 Recommendations

1. **Implement E2EE:** For sensitive workloads, add optional E2EE layer
2. **Key Rotation:** Implement automatic API key rotation
3. **HSM Support:** Allow API key storage in hardware security modules
4. **Anomaly Detection:** Monitor for unusual access patterns
5. **Mesh Security Audit:** Before enabling mesh, conduct security review

### 14.4 Compliance Considerations

| Requirement | Status | Notes |
|-------------|--------|-------|
| Data in Transit Encryption | ✓ | TLS 1.3 everywhere |
| Data at Rest Encryption | Partial | API keys hashed, conversations in plaintext |
| Access Logging | ✓ | Audit log for commands |
| User Data Isolation | ✓ | User-scoped channels and queries |
| Right to Deletion | Manual | No automated data purge |

---

## 15. References

### 15.1 Standards

- RFC 8439: ChaCha20 and Poly1305 for IETF Protocols
- RFC 8446: The Transport Layer Security (TLS) Protocol Version 1.3
- RFC 9000: QUIC: A UDP-Based Multiplexed and Secure Transport
- RFC 6455: The WebSocket Protocol

### 15.2 External Documentation

- [Supabase Realtime Documentation](https://supabase.com/docs/guides/realtime)
- [Phoenix Channels Protocol](https://hexdocs.pm/phoenix/channels.html)
- [Noise Protocol Framework](http://www.noiseprotocol.org/noise.html)

### 15.3 Internal References

- Source: `brainwires-cli/src/remote/` - Bridge implementation
- Source: `brainwires-cli/src/ipc/` - IPC encryption
- Source: `brainwires-studio/lib/remote/` - Protocol types
- Source: `brainwires-studio/lib/realtime/` - Realtime channels

---

## Appendix A: Protocol Message Quick Reference

### CLI → Backend

| Message Type | Realtime Event | Purpose |
|--------------|----------------|---------|
| Register | `remote.register` | Initial connection |
| Heartbeat | `remote.heartbeat` | Status update |
| CommandResult | `remote.command_result` | Command response |
| AgentEvent | `remote.event` | Lifecycle events |
| AgentStream | `remote.stream` | Output streaming |
| Pong | `remote.pong` | Health check response |

### Backend → CLI

| Command Type | Realtime Event | Purpose |
|--------------|----------------|---------|
| Authenticated | (HTTP response) | Auth success |
| SendInput | `remote.command` | User message |
| SlashCommand | `remote.command` | Slash command |
| CancelOperation | `remote.command` | Stop agent |
| Subscribe | `remote.command` | Start streaming |
| Unsubscribe | `remote.command` | Stop streaming |
| SpawnAgent | `remote.command` | Create agent |
| RequestSync | `remote.command` | Force heartbeat |
| Ping | `remote.ping` | Health check |
| Disconnect | `remote.command` | Graceful close |

---

## Appendix B: Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `AUTH_INVALID_KEY` | 401 | API key format invalid |
| `AUTH_KEY_NOT_FOUND` | 401 | API key not in database |
| `AUTH_KEY_EXPIRED` | 401 | API key past expiration |
| `AUTH_KEY_INACTIVE` | 401 | API key deactivated |
| `AUTH_SESSION_INVALID` | 401 | Session token invalid |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `AGENT_NOT_FOUND` | 404 | Agent ID unknown |
| `AGENT_DISCONNECTED` | 503 | Agent not connected |
| `COMMAND_TIMEOUT` | 504 | Command execution timeout |
| `INTERNAL_ERROR` | 500 | Server-side error |

---

## Appendix C: Configuration Schema

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "BrainwiresRemoteConfig",
  "type": "object",
  "properties": {
    "remote": {
      "type": "object",
      "properties": {
        "enabled": {
          "type": "boolean",
          "default": false
        },
        "backend_url": {
          "type": "string",
          "format": "uri",
          "default": "https://brainwires.studio"
        },
        "heartbeat_interval_secs": {
          "type": "integer",
          "minimum": 5,
          "maximum": 60,
          "default": 30
        },
        "reconnect_delay_secs": {
          "type": "integer",
          "minimum": 1,
          "maximum": 300,
          "default": 5
        },
        "max_reconnect_attempts": {
          "type": "integer",
          "minimum": 0,
          "default": 0,
          "description": "0 = unlimited"
        },
        "auto_start": {
          "type": "boolean",
          "default": true
        },
        "mesh": {
          "type": "object",
          "properties": {
            "enabled": {
              "type": "boolean",
              "default": false
            },
            "listen_port": {
              "type": "integer",
              "minimum": 1024,
              "maximum": 65535,
              "default": 7890
            },
            "discovery_methods": {
              "type": "array",
              "items": {
                "type": "string",
                "enum": ["backend", "mdns", "dht"]
              },
              "default": ["backend", "mdns"]
            }
          }
        }
      }
    }
  }
}
```

---

*End of Specification*
