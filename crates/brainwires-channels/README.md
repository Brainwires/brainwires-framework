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
| `ChannelEvent` | 8 event variants: message received/edited/deleted, reactions, typing, presence |
| `ChannelCapabilities` | 12 bitflags: rich text, media, threads, reactions, voice, video, etc. |
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
