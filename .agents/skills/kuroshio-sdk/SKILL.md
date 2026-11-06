---
name: kuroshio-sdk
description: Guide and patterns for developing decentralized teams, spatial WebAssembly guest apps (LumeriaApp), 64-bit ECEF geodesy (Nautilus World), and Coral Escrow state channels using the Kuroshio SDK. Use when creating, extending, or integrating apps, teams, spatial overlays, or mini-apps with the Kuroshio SDK.
---

# Kuroshio SDK Agent Skill

This skill guides agents in developing, scaffolding, and verifying applications, spatial overlays, teams, and Wasm guest modules using the **Kuroshio SDK** (`kuroshio-sdk`).

## Core Architecture & Crates

The SDK provides clean-room, public developer abstractions for the Lemuria & Nautilus ecosystem:

```
                      +-----------------------------+
                      |     Your Community App      |
                      |   (Pure WASM / Rust Crate)  |
                      +--------------+--------------+
                                     |
        +-----------------------------+-----------------------------+
        |                             |                             |
        v                             v                             v
+---------------+            +---------------+            +------------------+
| nautilus-world|            | kuroshio-mesh |            | lumeria-app-sdk  |
| 64-bit ECEF   |            | Teams, Squads |            | WASM Guest FFI   |
| Planetary     |            | & Coral State |            | app.manifest.json|
| Geodesy       |            | Channels      |            | Capability Bounds|
+---------------+            +---------------+            +------------------+
```

1. **`nautilus-world`**: 64-bit WGS84 ECEF geodesy, `LocalTangentFrame` (East-North-Up), and `AnchoredSpatialOverlay` for jitter-free planetary anchoring.
2. **`kuroshio-mesh`**: Decentralized team & squad registration, member roles, and Coral state channel tipping.
3. **`lumeria-app-sdk`**: Sandboxed `LumeriaApp` guest trait, permission manifest builder (`AppManifest`), input events (`LumeriaInputEvent`), and host commands (`LumeriaHostCommand`).

---

## 1. Geodesy & Planetary Anchoring (`nautilus-world`)

### Geodetic Coordinate to ECEF
All local tactical gameplay, venues, and storefronts are pinned to Earth using 64-bit geodetic coordinates:

```rust
use nautilus_world::{Wgs84GeoCoord, AnchoredSpatialOverlay};

// WGS-84 coordinate (latitude, longitude, altitude in meters)
let origin = Wgs84GeoCoord::new(35.658580, 139.745433, 20.0);

// Convert to 64-bit ECEF vector
let ecef_dvec = origin.to_ecef();
```

### Local Tangent Frame (ENU)
Local physics and UI shaders MUST use East-North-Up (ENU) tangent frames to prevent vertex jitter and curvature distortion:

```rust
let arena = AnchoredSpatialOverlay::new(
    "overlay_tokyo_arena",
    "Tokyo Arena Tactical Zone",
    origin,
    1500.0, // 1.5km bounding radius
);

let enu_frame = arena.tangent_frame();
// Local delta (East, North, Up) in meters to ECEF:
let player_local = glam::Vec3::new(10.0, 5.0, 0.0);
let player_ecef = enu_frame.local_to_ecef(player_local);
```

---

## 2. Teams, Squads & Factions (`kuroshio-mesh`)

```rust
use kuroshio_mesh::{KuroshioTeam, MemberRole, SquadMember};
use nautilus_world::Wgs84GeoCoord;

let mut team = KuroshioTeam::new(
    "team_seraphim",
    "Seraphim Tactical Wing",
    "SERAPH",
    "did:atlantis:founder_pubkey",
    "Callsign_Vance",
);

team.set_headquarters(Wgs84GeoCoord::new(35.658580, 139.745433, 20.0));

// Add members
team.add_member(SquadMember {
    did: "did:atlantis:pilot_02".into(),
    callsign: "Aero_Zero".into(),
    role: MemberRole::Operator,
    reputation_score: 50,
    joined_timestamp: 1729000000,
});

// Coral treasury tipping
team.deposit_treasury(100.0); // 100 CYAN
```

---

## 3. The Seven Daughters 24/7 Siren Radio Network (`kuroshio-mesh`)

The global Siren Frequency Theater Radio (`siren:frequency:global_stage`, 432 Hz base) operates 24/7 across 7 harmonic time-slots hosted by **The Seven Daughters**:

| Host | Frequency | Air Time (UTC) | Theme / Genre |
| :--- | :--- | :--- | :--- |
| **Maia** | 432 Hz | 04:00 - 08:00 | Dawn Oceanic Ambient / Horizon Chill |
| **Alcyone** | 528 Hz | 08:00 - 12:00 | High-Speed Cyberpunk D&B / Breakbeats |
| **Asterope** | 639 Hz | 12:00 - 16:00 | Tactical Deep House & Frontline Reports |
| **Celaeno** | 741 Hz | 16:00 - 20:00 | Glitch / Industrial Bass / Storm Chaser |
| **Electra** | 852 Hz | 20:00 - 00:00 | Vocal Trance / Eurobeat / Club Atlantis |
| **Taygete** | 963 Hz | 00:00 - 02:00 | Darkwave / Minimal Techno / Abyssal Drone |
| **Merope** | 396 Hz | 02:00 - 04:00 | Lo-Fi / Starlight Harbor Acoustic |

### Tuning In & Micro-Tipping
```rust
use kuroshio_mesh::{SevenDaughtersHost, SirenRadioStream};

// 1. Resolve host by current UTC hour
let current_utc_hour = 14;
let stream = SirenRadioStream::new(current_utc_hour);
assert_eq!(stream.active_daughter, SevenDaughtersHost::Asterope);

// Unified stream originating directly from the Raspberry Pi
println!("Siren Pi Stream: {}", stream.stream_url); // http://10.0.0.237:8080/live.mp3

// 2. Micro-tip 0.1 CYAN via Coral state channel
let tip = stream.create_tip("did:atlantis:listener_wallet", 1, "Love this set!");
println!("Tipped host: {} with {} CYAN", tip.recipient_did, tip.amount_cyan);
```

---

## 4. Olympus Seed Node Offloading (`kuroshio-mesh`)

Custom autonomous daemons, mobile agents, and local tools running on the Raspberry Pi node can offload heavy AI model inference and root consensus verification to Olympus to protect local memory and thermal limits:

```rust
use kuroshio_mesh::OlympusSeedEndpoint;

let endpoint = OlympusSeedEndpoint::default();
let envelope = endpoint.create_offload_envelope(
    "did:atlantis:custom_pi_worker",
    "heavy_nlp_synthesis",
    b"Analyze radio feedback".to_vec(),
);
println!("Offload packet routed to: {}", endpoint.http_ingress_url);
```

---

## 5. Sandboxed Guest Apps & Manifests (`lumeria-app-sdk`)

Every guest Wasm module or spatial app requires an `AppManifest`:

```rust
use lumeria_app_sdk::{AppManifest, AppCapability, AppSecurityTier};

let manifest = AppManifest::new(
    "com.seraph.tactical",
    "Tactical Wing HUD",
    "0.1.0",
    "did:atlantis:publisher_pubkey"
)
.with_capability(AppCapability::LocationCoarse)
.with_capability(AppCapability::UiOverlay)
.with_capability(AppCapability::AudioSpatialStream);

// Validate permissions
manifest.validate_permissions().expect("Valid manifest");
```

### Capability Security Tiers
- **`AppCapability::LocationCoarse`**: Access to coarse geohash/sector only (never raw high-precision GPS).
- **`AppCapability::StoreInitiateCheckout`**: Requires `AppSecurityTier::ClubVerified` or higher.
- **`AppCapability::VehicleTelemetryRead`**: Telemetry hooks (speed, Pacejka slip, RPM).

### Implementing the `LumeriaApp` Guest Lifecycle
```rust
use lumeria_app_sdk::{LumeriaApp, LumeriaInputEvent, LumeriaHostCommand};

pub struct MyGuestApp;

impl LumeriaApp for MyGuestApp {
    fn on_start(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn on_tick(&mut self, _delta_time: f32) -> Vec<LumeriaHostCommand> {
        vec![]
    }

    fn on_input(&mut self, event: LumeriaInputEvent) -> Vec<LumeriaHostCommand> {
        match event {
            LumeriaInputEvent::ProximityTrigger { totem_id, distance_meters } => {
                if distance_meters < 2.0 {
                    vec![LumeriaHostCommand::ShowNotification {
                        title: "Storefront Entered".into(),
                        message: format!("Approached totem {}", totem_id),
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}
```

---

## 6. Tier 2 Cross-Platform Architecture & Hardening Pillars

When authoring or auditing Tier 2 mobile and desktop client applications:
1. **Platform-Native Secure Enclave Storage**:
   - iOS: Keychain Services API with device-lock access bounds.
   - Android: `AndroidKeyStore` StrongBox with biometrics/PIN protection.
   - Windows: Windows DPAPI (`CryptProtectData`).
   - In-memory representations must be strictly zeroized on drop.
2. **Cellular / Wi-Fi Network Handoff**:
   - Pre-buffer 5–10s of Opus audio in a lock-free ring buffer for uninterrupted Siren Radio listening.
   - Renegotiate ICE/STUN candidates gracefully without resetting audio tracks or UI frames.
   - Buffer uncommitted Coral micro-tips locally in an encrypted delay-tolerant queue (`DTNQueue`) during dead zones.
3. **Storage Quotas & Offline State**:
   - Enforce an LRU cache capped at **250 MB** for DEM elevation tiles and vector map data.
   - Cap temporary radio chunk cache at **50 MB**.
4. **Adaptive Resource Sharing**:
   - Only engage libp2p GossipSub relaying or stream chunk seeding when connected to unmetered Wi-Fi/Ethernet, plugged into AC (or battery > 40%), and thermal status is nominal.

---

## 7. Security Hardening Primitives (6-Vector Lockdown)

1. **Hardware Enclave Key Vault & `zeroize` (`kuroshio_mesh::security`)**:
   - Implement `HardwareKeyVault` (`AndroidKeystoreStrongBox`, `AppleSecureEnclave`, `LinuxTpm2`, `WindowsDpapi`) so private keys never enter raw Rust heap memory.
   - Wrap ephemeral session secrets in `EphemeralSessionSecret` (`ZeroizeOnDrop`).
2. **Domain-Separated Coral State Channels & Watchtowers (`coral_escrow` & `kuroshio_mesh::security`)**:
   - Bind every off-chain state update to `CoralDomainSeparator` (`chain_id`, `network_name`, `verifying_program_id`, `match_channel_id`) and enforce strictly monotonic nonces via `DomainSeparatedChannelState::verify_monotonic_transition`.
   - Use `submit_watchtower_fraud_proof` on Tier 3 Atlantis Watchtowers to penalize stale state broadcasts and slash 10% (`WATCHTOWER_SLASH_BPS = 1000`) of a cheater's escrow deposit.
3. **Noise/TLS 1.3 DID Binding, Mobile Handoff Tokens & GossipSub Anti-Eclipse (`kuroshio_mesh::transport`)**:
   - Issue and verify `HandoffResumptionToken` (HMAC-SHA256 bound to PeerID, DID, and monotonic `handoff_generation`) during Wi-Fi to 5G handoffs.
   - Enforce `GossipSubPeerScorer` (quadratic invalid message penalty, per-minute topic rate limits, and `/24` IPv4 subnet anti-eclipse limits).
4. **Zero-WASI Capability Sandbox & Memory Caps (`lumeria_host_runner`)**:
   - Audit guest modules with `WasiSandboxPolicy::strict_zero_wasi()` (`audit_wasm_imports`) to block `wasi_snapshot_preview1` filesystem/socket imports, enforce 500k per-frame instruction fuel limits, and cap linear memory at 256 pages (`16 MiB`) via `request_memory_grow()`.
5. **Olympus Tier Boundary Isolation (`olympus-core` Feature & `deny.toml`)**:
   - Compile Olympus targets with `--no-default-features --features olympus-core` and validate with `cargo-deny` (`deny.toml`) so Tier 2/3 networking dependencies never enter Tier 1.
6. **Geodetic Deserialization Sanitization (`nautilus_world`)**:
   - All `Wgs84GeoCoord`, `KinematicStateVector`, and `AnchoredSpatialOverlay` payloads enforce custom Serde deserializers rejecting `NaN`, `Infinity`, and out-of-bounds geodetic coordinates at the deserialization boundary.

## 8. Key Rules for Agents
1. **Zero Raw GPS Broadcasting**: Always enforce `LocationCoarse` or `ZK-Geo` coarse hashes. High-precision GPS is vaulted in the Club Home Node.
2. **Dual-Precision Geodesy**: Keep world-space coordinates in 64-bit `DVec3`/`f64` ECEF; downscale to camera-relative 32-bit `Vec3`/`f32` in tangent frames before passing to shaders.
3. **Pure Rust / Wasm**: Maintain pure Rust compatibility (`wasm32-unknown-unknown` friendly, zero non-standard C dependencies).
4. **Single Raspberry Pi Radio Stream**: The entire Siren 24/7 radio broadcast is served strictly as a single unified stream originating from the local Raspberry Pi node (`http://10.0.0.237:8080/live.mp3`).
5. **Two-Tier Compute Offload**: Edge and custom daemons run lightweight I/O and custom MCP tools on the Raspberry Pi node (`LocalPiNode`), offloading heavy model inference and root consensus batches to the Olympus Seed Node (`RemoteOlympusSeed`).
6. **Olympus Untouched**: Tier 2 and Tier 3 clients must never create hard runtime dependencies on Olympus; Olympus operates independently of all edge node availability.
