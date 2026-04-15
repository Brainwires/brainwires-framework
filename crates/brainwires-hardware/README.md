# brainwires-hardware

Hardware I/O for the [Brainwires Agent Framework](https://github.com/Brainwires/brainwires-framework).

Provides a unified hardware abstraction layer covering audio, GPIO, Bluetooth, network hardware, and home automation protocols — all behind opt-in feature flags so you only compile what you need.

## Modules

| Module | Feature | Description |
|--------|---------|-------------|
| `audio` | `audio` | Audio capture/playback, STT, TTS (16 cloud providers + local Whisper) |
| `audio/vad` | *(always)* / `vad` | Voice activity detection — `EnergyVad` (always) + `WebRtcVad` (`vad`) |
| `audio/wake_word` | `wake-word` | Wake word detection — `EnergyTriggerDetector` + optional ML backends |
| `audio/assistant` | `voice-assistant` | End-to-end voice assistant pipeline |
| `gpio` | `gpio` | GPIO pin management with safety allow-lists and PWM (Linux) |
| `bluetooth` | `bluetooth` | BLE advertisement scanning and adapter enumeration |
| `network` | `network` | NIC enumeration, IP config, ARP host discovery, port scanning |
| `camera` | `camera` | Webcam/camera frame capture (V4L2/AVFoundation/MSMF) |
| `usb` | `usb` | Raw USB device enumeration and transfers (no libusb) |
| `homeauto/zigbee` | `zigbee` | Zigbee 3.0 coordinator — EZSP (Silicon Labs) + ZNP (TI Z-Stack) backends |
| `homeauto/zwave` | `zwave` | Z-Wave Plus v2 Serial API — node incl/excl, command class send/recv |
| `homeauto/thread` | `thread` | OpenThread Border Router REST API client (Thread 1.3.0) |
| `homeauto/matter` | `matter` | Matter 1.3 controller + device server (mDNS, UDP, TLV, QR commissioning) |

## Getting started

```toml
[dependencies]
# Pick only what you need:
brainwires-hardware = { version = "0.8", features = ["audio"] }
brainwires-hardware = { version = "0.8", features = ["gpio"] }
brainwires-hardware = { version = "0.8", features = ["bluetooth"] }
brainwires-hardware = { version = "0.8", features = ["network"] }

# Or enable everything:
brainwires-hardware = { version = "0.8", features = ["full"] }
```

## Feature flags

| Feature | Description |
|---------|-------------|
| `audio` | Hardware audio I/O via CPAL + 16 cloud STT/TTS providers |
| `flac` | FLAC encode/decode |
| `local-stt` | Local Whisper STT inference via whisper-rs (heavy dep, opt-in) |
| `vad` | WebRTC VAD algorithm (`EnergyVad` is always available with `audio`) |
| `wake-word` | Wake word detection — `EnergyTriggerDetector` (zero deps) |
| `wake-word-rustpotter` | `RustpotterDetector` — pure-Rust ML wake word (opt-in, see notes) |
| `wake-word-porcupine` | `PorcupineDetector` — Picovoice Porcupine (requires AccessKey + git dep) |
| `voice-assistant` | Full pipeline: capture → wake word → VAD → STT → handler → TTS |
| `gpio` | GPIO pin control via Linux character device API (`gpio-cdev`) |
| `bluetooth` | BLE scanning and adapter enumeration via `btleplug` |
| `network` | NIC enumeration, IP config, ARP discovery, port scanning |
| `camera` | Webcam/camera capture via nokhwa (V4L2/AVFoundation/MSMF) |
| `usb` | Raw USB device access and transfers via nusb (no libusb) |
| `zigbee` | Zigbee 3.0 via EZSP (Silicon Labs EFR32) or ZNP (TI Z-Stack 3.x) |
| `zwave` | Z-Wave Plus v2 (ZAPI2) over USB serial stick |
| `thread` | OpenThread Border Router (OTBR) REST API client |
| `matter` | Matter 1.3 controller + device server (pure-Rust stack, no rs-matter) |
| `matter-ble` | BLE commissioning window (btleplug peripheral, Linux/macOS) |
| `homeauto` | All four home automation protocols (`zigbee` + `zwave` + `thread` + `matter`) |
| `homeauto-full` | All home automation including BLE (`homeauto` + `matter-ble`) |
| `full` | All features (except `local-stt`, `wake-word-rustpotter`, `wake-word-porcupine`) |

## Audio

Supports hardware capture and playback via [CPAL](https://crates.io/crates/cpal), plus cloud STT/TTS integrations:

**STT:** OpenAI, Azure, Deepgram, ElevenLabs, Fish Audio
**TTS:** OpenAI, Azure, Deepgram, ElevenLabs, Fish Audio, Google, Murf, Cartesia

```rust
use brainwires_hardware::{TextToSpeech, TtsOptions, OutputFormat};

let tts = OpenAiTts::new(api_key);
let audio = tts.synthesize("Hello, world!", &TtsOptions::default()).await?;
```

## GPIO (Linux)

Safe GPIO access with explicit allow-lists — no pin can be used unless it appears in the configured policy.

```rust
use brainwires_hardware::{GpioPinManager, GpioSafetyPolicy};
use brainwires_hardware::gpio::device::GpioDirection;

let mut manager = GpioPinManager::from_config(&config);
let pin = manager.acquire(0, 17, GpioDirection::Output, "my-agent")?;
```

## Bluetooth

Cross-platform BLE scanning using [btleplug](https://crates.io/crates/btleplug):

```rust
use brainwires_hardware::bluetooth;
use std::time::Duration;

let devices = bluetooth::scan_ble(Duration::from_secs(5)).await;
for d in &devices {
    println!("{} — {:?}", d.address, d.name);
}
```

## Network

```rust
use brainwires_hardware::network;
use std::time::Duration;

// List interfaces
for iface in network::list_interfaces() {
    println!("{} ({:?})", iface.name, iface.kind);
}

// IP config with gateways
for cfg in network::get_ip_configs() {
    println!("{}: gateway={:?}", cfg.interface, cfg.gateway);
}

// Port scan
let results = network::scan_common_ports(
    "192.168.1.1".parse().unwrap(),
    Duration::from_millis(500),
).await;

// ARP host discovery (requires CAP_NET_RAW)
let hosts = network::arp_scan("192.168.1.0/24".parse().unwrap()).await;
```

## Voice Activity Detection

`EnergyVad` is always available (no extra feature needed beyond `audio`). `WebRtcVad` requires the `vad` feature.

```rust
use brainwires_hardware::{EnergyVad, VoiceActivityDetector};

let vad = EnergyVad::default(); // -40 dBFS threshold
if vad.is_speech(&audio_buffer) {
    println!("Speech detected!");
}
```

## Wake Word Detection

```rust
use brainwires_hardware::{EnergyTriggerDetector, WakeWordDetector};

let mut detector = EnergyTriggerDetector::new(-20.0, 3, 16_000);
// Feed 30 ms i16 frames from the mic:
if let Some(detection) = detector.process_frame(&frame) {
    println!("Wake trigger! score={:.2}", detection.score);
}
```

## Voice Assistant Pipeline

```rust
use brainwires_hardware::{VoiceAssistant, VoiceAssistantHandler, VoiceAssistantConfig};
use brainwires_hardware::audio::types::Transcript;
use async_trait::async_trait;

struct MyHandler;

#[async_trait]
impl VoiceAssistantHandler for MyHandler {
    async fn on_speech(&self, transcript: &Transcript) -> Option<String> {
        println!("You said: {}", transcript.text);
        Some("I heard you!".to_string())
    }
}

// Build and run
let mut assistant = VoiceAssistant::builder(capture, stt)
    .with_playback(playback)
    .with_tts(tts)
    .with_wake_word(Box::new(EnergyTriggerDetector::default()))
    .build();

assistant.run(&MyHandler).await?;
```

## Home Automation

All four protocols are behind the `homeauto` feature (or enable each individually).

```toml
brainwires-hardware = { version = "0.8", features = ["homeauto"] }
```

### Zigbee (`zigbee`)

Two serial backends sharing the `ZigbeeCoordinator` trait:

```rust
use brainwires_hardware::homeauto::{EzspCoordinator, ZigbeeCoordinator};

// Silicon Labs EFR32 stick (EZSP v8 over ASH)
let coord = EzspCoordinator::open("/dev/ttyUSB0", 115_200).await?;
coord.start().await?;
coord.permit_join(60).await?;

for dev in coord.devices().await? {
    println!("{} — {:?}", dev.addr.ieee, dev.kind);
}
```

### Z-Wave (`zwave`)

```rust
use brainwires_hardware::homeauto::{ZWaveSerialController, ZWaveController};
use brainwires_hardware::homeauto::CommandClass;

let ctrl = ZWaveSerialController::open("/dev/ttyUSB0", 115_200).await?;
ctrl.start().await?;

for node in ctrl.nodes().await? {
    println!("Node {}: {:?}", node.node_id, node.kind);
}

// Toggle a binary switch on node 3
ctrl.send_cc(3, CommandClass::SwitchBinary, &CommandClass::switch_binary_set(true)).await?;
```

### Thread (`thread`)

```rust
use brainwires_hardware::homeauto::ThreadBorderRouter;

let otbr = ThreadBorderRouter::new("http://192.168.1.100:8081").await?;
let info = otbr.node_info().await?;
println!("Network: {} ({:?})", info.network_name, info.role);

for neighbor in otbr.neighbors().await? {
    println!("  neighbor {} rssi={}", neighbor.ext_address, neighbor.rssi);
}
```

### Matter (`matter`)

**Controller — commission and control a device:**

```rust
use brainwires_hardware::homeauto::{MatterController};
use std::path::Path;

let ctrl = MatterController::new("MyFabric", Path::new("/tmp/matter-fabric")).await?;
let device = ctrl.commission_qr("MT:Y.K9042C00KA0648G00", 1).await?;
ctrl.on_off(&device, 1, true).await?;
```

**Device server — expose an agent as a Matter device:**

```rust
use brainwires_hardware::homeauto::{MatterDeviceConfig, MatterDeviceServer};

let config = MatterDeviceConfig::builder()
    .device_name("Brainwires Light")
    .vendor_id(0xFFF1)
    .product_id(0x8001)
    .discriminator(3840)
    .passcode(20202021)
    .build();

let server = MatterDeviceServer::new(config).await?;
server.set_on_off_handler(|on| println!("On/Off: {on}"));
println!("QR: {}", server.qr_code());
server.start().await?;  // blocks; scan QR code with your Matter controller
```

#### Matter Stack — What's Implemented

The `matter` feature ships a **complete Matter 1.3 protocol stack** written entirely in pure Rust (no `rs-matter` or `embassy-time` dependency):

| Layer | Status | Notes |
|-------|--------|-------|
| SPAKE2+ (RFC 9383) | Complete | RustCrypto p256, PBKDF2-HMAC-SHA256, cA/cB confirmation |
| PASE commissioning | Complete | Full PBKDFParam/Pake1/2/3 handshake, session key derivation |
| CASE operational | Complete | SIGMA Sigma1/2/3, P-256 ECDH, AES-CCM-128, NOC verification |
| Matter TLV certs | Complete | NOC/ICAC/RCAC encode/decode, P-256 ECDSA-SHA256 |
| Fabric management | Complete | Root CA generation, NOC issuance, JSON persistence |
| Message transport | Complete | Matter §4.4 header, MRP retry/backoff, AES-CCM-128 UDP |
| Interaction Model | Complete | Read/Write/Invoke/Subscribe, wildcard paths, TLV codec |
| Commissioning clusters | Complete | BasicInformation, GeneralCommissioning, OperationalCredentials, NetworkCommissioning |
| mDNS advertisement | Complete | `_matterc._udp` commissionable + `_matter._tcp` operational |
| BLE commissioning | Complete (`matter-ble`) | BTP handshake, segmentation/reassembly, btleplug peripheral |
| CASE session resumption | Not yet implemented | Sigma2Resume path |
| Multi-fabric | Not yet implemented | Single fabric per controller instance |
| BLE on Windows | Not implemented | btleplug WinRT BLE requires additional work |

Run the ready-made examples to get started:

```bash
# Expose this machine as a Matter on/off light — scan the printed QR code
cargo run --example matter_server --features matter

# Commission a real Matter device and toggle it
cargo run --example matter_on_off --features matter -- commission "MT:YOUR_QR_CODE"
```

## Migration from brainwires-audio

```toml
# Before
brainwires-audio = "0.8"

# After
brainwires-hardware = { version = "0.8", features = ["audio"] }
```

All public types and traits are re-exported from the crate root — existing code using
`brainwires_audio::*` can switch to `brainwires_hardware::*` with no further changes.

## Examples

```bash
cargo run -p brainwires-hardware --example text_to_speech --features audio
cargo run -p brainwires-hardware --example bluetooth_scan --features bluetooth
cargo run -p brainwires-hardware --example network_interfaces --features network
cargo run -p brainwires-hardware --example port_scan --features network -- 192.168.1.1
sudo cargo run -p brainwires-hardware --example host_discovery --features network -- 192.168.1.0/24

# Wake word demo (prints detections from mic)
cargo run -p brainwires-hardware --example wake_word_demo --features wake-word

# Full voice assistant demo (requires OPENAI_API_KEY)
cargo run -p brainwires-hardware --example voice_assistant --features voice-assistant

# Standalone voice assistant binary
cargo run -p voice-assistant -- --list-devices
cargo run -p voice-assistant -- --verbose

# Home automation examples (requires physical hardware for full operation)
cargo run -p brainwires-hardware --example zigbee_scan --features zigbee
cargo run -p brainwires-hardware --example zwave_nodes --features zwave
cargo run -p brainwires-hardware --example thread_info --features thread -- http://192.168.1.100:8081
cargo run -p brainwires-hardware --example matter_on_off --features matter -- serve
cargo run -p brainwires-hardware --example matter_on_off --features matter -- commission "MT:Y.K9042C00KA0648G00"
```
