# Kuroshio SDK: Open World Geodesy, Mesh Teams & Lumeria Sandboxed App SDK

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

The official public open SDK for building **decentralized teams, social squads, and spatial WebAssembly applications** across the **Nautilus 3D Planetary World** and **Kuroshio Gyre** mesh network.

---

## 🏛️ Architecture Overview: Nautilus IS the Playing Field

> [!IMPORTANT]
> **Nautilus is not an abstract mini-map or a 2D GIS widget.** Nautilus **is the actual 3D playing field**.
> The game engine renders the real, continuous Earth globe from **"God's Eye View" space orbits and continental heatmaps** all the way down into **tactical boots-on-the-ground first-person shooter (FPS) combat arenas, vehicle dynamics, and oceanic exploration** with zero loading screens.

```
       [ 🛰️ God's Eye View: Orbit & Satellites (Alt > 100km, SGP4, Global Heatmaps) ]
                                          │
                                          ▼  (Continuous Quadtree CDLOD Zoom)
       [ 🌍 Continental & Metropolitan Sector Airspace (Alt 5km - 25km, H3 / SIM) ]
                                          │
                                          ▼  (Seamless Atmospheric Entry / Descent)
       [ 🎯 Boots-on-the-Ground Tactical Battlefield (Alt < 500m, DEM Heights, 64-bit ECEF) ]
         • First-Person ADS Scope Aiming & Ballistics (Verdansk / Tactical Arenas)
         • Rapier3D Terrain Collision Mesh (LocalTangentFrame Centimeter Physics)
         • Pacejka Vehicle Dynamics & Street-Level Co-op Squad Navigation
```

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
       |                             |                             |
       +-----------------------------+-----------------------------+
                                     |
                                     v
                 +---------------------------------------+
                 |    Pre-Compiled Lemuria Host Runtime   |
                 | (HeavenlyBound / Lite / WebGPU Canvas)|
                 +---------------------------------------+
```

---

## 📦 Crates in this Workspace

| Crate | Purpose | Crates.io |
| :--- | :--- | :--- |
| [`crates/nautilus-world`](crates/nautilus-world) | 64-bit WGS84 ECEF geodesy, DGGS H3/S2 spatial indexing, SIM packet filtering, DEM terrain heightfield baking, and WebGPU float splitters. | `0.1.0` |
| [`crates/kuroshio-mesh`](crates/kuroshio-mesh) | Team registration, Groth16 ZK-Geo proofs, PoH micro-VDF Sybil protection, libp2p GossipSub & STUN/TURN NAT hole punching, and WebRTC Opus swarm audio. | `0.1.0` |
| [`crates/lumeria-app-sdk`](crates/lumeria-app-sdk) | Sandboxed `LumeriaApp` guest trait, Wasm Component Model `.wit` interface (`wit/lumeria-guest.wit`), 120Hz zero-copy `SharedInputRingBuffer`, and Wasmtime fuel budgeting (`WasmFuelMeter`). | `0.1.0` |
| [`crates/lumeria-host-runner`](crates/lumeria-host-runner) | Host-side Wasmtime execution engine with shared linear memory mapping and 500k instruction per-frame safety interruption handlers. | `0.1.0` |
| [`crates/kuroshio-mock-node`](crates/kuroshio-mock-node) | Local mock Atlantis/Triton node with sequence-signed Coral state channel dispute timers and Solana SPL/Anchor PDA settlement instruction builders. | `0.1.0` |
| [`programs/coral-escrow`](programs/coral-escrow) | On-chain Anchor-compatible Solana smart contract for Coral state channels, 1-hour challenge dispute windows, and 2% club treasury dividend payouts. | `0.1.0` |

---

## 🚀 Quickstart: Building a Team on the Nautilus World

Add the crates to your `Cargo.toml`:

```toml
[dependencies]
nautilus-world = { git = "https://github.com/Neptune537/kuroshio-sdk.git", package = "nautilus-world" }
kuroshio-mesh = { git = "https://github.com/Neptune537/kuroshio-sdk.git", package = "kuroshio-mesh" }
lumeria-app-sdk = { git = "https://github.com/Neptune537/kuroshio-sdk.git", package = "lumeria-app-sdk" }
```

### 1. Register a Squad & Spatial Arena

```rust
use nautilus_world::{Wgs84GeoCoord, AnchoredSpatialOverlay};
use kuroshio_mesh::KuroshioTeam;
use lumeria_app_sdk::{AppManifest, AppCapability};

fn main() {
    // 1. Authoritative Earth coordinate (e.g. Tokyo Tower)
    let origin = Wgs84GeoCoord::new(35.658580, 139.745433, 20.0);

    // 2. Establish team headquarters on Kuroshio Mesh
    let mut team = KuroshioTeam::new(
        "team_seraphim",
        "Seraphim Tactical Wing",
        "SERAPH",
        "did:atlantis:founder_pubkey",
        "Callsign_Vance",
    );
    team.set_headquarters(origin);

    // 3. Anchor a 3D game arena / storefront to Earth
    let arena = AnchoredSpatialOverlay::new(
        "overlay_tokyo_arena",
        "Tokyo Arena Tactical Zone",
        origin,
        1500.0, // 1.5km bounding radius
    );

    let enu_frame = arena.tangent_frame();
    println!("Anchored ECEF Position: {:?}", enu_frame.origin_ecef);

    // 4. Declare capabilities for the sandboxed Lemuria runtime
    let manifest = AppManifest::new("com.seraph.tactical", "Tactical Wing", "0.1.0", "did:atlantis:pubkey")
        .with_capability(AppCapability::LocationCoarse)
        .with_capability(AppCapability::UiOverlay);

    manifest.validate_permissions().expect("Manifest valid");
}
```

---

## 📱 Cross-Platform Tier 2 Architecture & Client Guidelines (PC, Android, iOS)

For developers building Tier 2 community and client applications across **Windows PCs (low-spec/integrated GPU laptops)**, **Android (API 26+)**, and **Apple (iOS 16+ / macOS)**:

### 1. System Topology & Zero-Trust Invariants
- **Platform Invariant (Dashboard & Companion In-App Menu Standard):** The **Dashboard** (formerly lite) is established as the official sovereign standard across the entire platform. It serves two unified roles:
  1. **Standalone Companion App:** The primary desktop (Windows/macOS/Linux) and mobile (Android/iOS) hub for community task bounties, IRL spatial navigation, guild rosters, 24/7 Siren Radio, and wallet management.
  2. **Standard In-App Menu Component:** The universal overlay HUD embedded inside all platform applications, games, and sub-apps (e.g., Lemuria AAA, Tactical Wings, and community WASM modules).
- **Architecture Replacement:** Completely replaces legacy centralized app architectures (Flutter runtimes, Firebase/Firestore databases, and legacy tithX contracts) with **pure Rust, local SQLite caching, Coral State Channels, and Lumeria WASM sandboxing**.
- **Platform Invariant (The Replacement for Lite):** All apps on the Kuroshio Gyre platform follow this universal sovereign standard: replacing legacy centralized app architectures (Flutter runtimes, Firebase/Firestore databases, and legacy `tithX` contracts) with **pure Rust, local SQLite caching, Coral State Channels, and Lumeria WASM sandboxing**.
- **Tier 2 (Client Layer):** Pelagic client edge handling UI rendering (Slint/WebGPU), IRL spatial task & gig discovery, local Siren Radio playback, spatial mapping, and opt-in idle mesh relaying.
- **Tier 3 (Headless Edge / Atlantis):** Headless workers (e.g. Raspberry Pi) dedicated to proof-of-health, DTN buffers, and state channel routing.
- **Olympus (Apex Keystone):** Remains 100% untouched and unencumbered. The network MUST NOT depend on Tier 2 client sessions.

### 2. The Four Hardening Pillars
1. **Secure Enclave Key Storage (`SecureKeyVault`):**
   - **iOS / macOS:** Apple Keychain (`kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`).
   - **Android:** Hardware-backed `AndroidKeyStore` (StrongBox Keymint when available) with zeroized memory buffers.
   - **Windows:** Windows DPAPI (`CryptProtectData`) / Credential Manager.
2. **Mobile Network Handoff & DTN Reconnection:**
   - 5–10 second pre-buffered Opus audio ring buffer for Siren Radio to eliminate pops/clicks during 5G/Wi-Fi handoffs.
   - Dynamic ICE candidate renegotiation for WebRTC swarm audio.
   - Offline uncommitted Coral micro-tips buffered locally in an encrypted delay-tolerant queue (`DTNQueue`).
3. **Bounded Storage & Offline Caching:**
   - Hard **250 MB ceiling** (configurable up to 1 GB) enforced via LRU eviction for DEM heightfields and vector map tiles.
   - Radio chunk temporary buffer capped at **50 MB**.
4. **Adaptive Resource Sharing & Thermal Safety:**
   - User toggle: `[x] Contribute Idle Compute & Mesh Relay`.
   - Mesh relaying and chunk seeding automatically suspend if on cellular, battery < 40%, or CPU thermal state exceeds `Fair`.

### 3. Multi-Target CI/CD Build Toolchains
- **Windows:** `x86_64-pc-windows-msvc` (native standalone executable + Slint DirectX/Vulkan/software backend).
- **Android:** `cargo-ndk` targeting `aarch64-linux-android` & `armv7-linux-androideabi` (generating JNI `.aar` libraries).
- **Apple iOS / macOS:** `cargo-lipo` / `cargo-xcode` targeting `aarch64-apple-ios` & `aarch64-apple-darwin` (generating universal XCFrameworks).
- **Wasm Sandbox:** `wasm32-unknown-unknown` validating `LumeriaApp` guest compliance.

---

## 🛡️ Security & Privacy Hardening Architecture (6-Vector Lockdown)

The Kuroshio SDK enforces strict cryptographic, spatial, and capability boundaries across all three network tiers (Olympus Tier 1, Lumeria Tier 2, and Atlantis Tier 3):

1. **Hardware-Backed Enclave Key Vault & `zeroize` Memory Wiping (`kuroshio-mesh::security`):**
   - Private keys for Coral Escrow state channels and DID wallets never enter raw Rust heap memory (`HardwareKeyVault` trait, `EnclaveKeyHandle`, and `EnclaveBackend`: Android Keystore StrongBox, Apple Secure Enclave, Linux/Pi TPM 2.0, and Windows DPAPI).
   - Ephemeral session secrets (`EphemeralSessionSecret`) derive `Zeroize` and `ZeroizeOnDrop` so secrets are overwritten with zeros immediately on scope exit.
2. **EIP-712 Domain-Separated Coral State Channels & Watchtower Fraud Proofs (`coral-escrow` & `kuroshio-mesh`):**
   - Every off-chain Coral state payload includes a `CoralDomainSeparator` (`chain_id`, `network_name`, `verifying_program_id`, and `match_channel_id`) and a strictly monotonic `state_nonce` (`DomainSeparatedChannelState::verify_monotonic_transition`).
   - Disconnected Lumeria clients are protected during the 1-hour dispute challenge window (`DISPUTE_WINDOW_SECONDS = 3600`) by Tier 3 Atlantis Watchtowers via `submit_watchtower_fraud_proof`, which verifies higher-nonce counter-proofs and slashes 10% of the cheater's deposit (`WATCHTOWER_SLASH_BPS = 1000`).
3. **Noise/TLS 1.3 DID Binding, Mobile Handoff Tokens & GossipSub v1.1 Anti-Eclipse (`kuroshio-mesh::transport`):**
   - `MeshTransportSecurity` mandates Noise XX (`Noise_XX_25519_ChaChaPoly_BLAKE2s`) or TLS 1.3 strictly bound to verified DIDs/PeerIDs.
   - `HandoffResumptionToken` provides HMAC-SHA256 authenticated session resumption tied to PeerID, DID, and a strictly increasing `handoff_generation` during Wi-Fi to 5G mobile handoffs.
   - `GossipSubPeerScorer` enforces GossipSub v1.1 quadratic invalid-message penalties (`P4`), per-minute topic rate limits, and a `/24` IPv4 subnet peer cap (`max_peers_per_ipv4_subnet = 2`) to defeat Sybil and Eclipse attacks.
4. **Zero-WASI Capability Sandbox, Fuel Metering & Memory Caps (`lumeria-host-runner`):**
   - `WasiSandboxPolicy::strict_zero_wasi()` strips all ambient host filesystem (`path_open`, `fd_read`, `fd_write`), socket (`sock_*`), and environment (`environ_get`) imports via `audit_wasm_imports()`.
   - Execution is bounded by a 500,000-instruction per-frame fuel budget (`WasmFuelMeter`) and a hard 256-page (`16 MiB`) linear memory cap (`request_memory_grow()`).
5. **Tier Boundary Isolation (`olympus-core` Feature Gate & `deny.toml`):**
   - `kuroshio-mesh` separates pure data structures (`--no-default-features --features olympus-core`) from Tier 2/3 networking and FFI (`mesh-transport`).
   - Enforced by `deny.toml` (`cargo-deny`) and `.github/workflows/security_audit.yml` CI checks to guarantee zero Tier 2/3 dependency contamination in Olympus.
6. **Geodetic Deserialization Sanitization (`nautilus-world`):**
   - Custom Serde validators on `Wgs84GeoCoord`, `KinematicStateVector`, and `AnchoredSpatialOverlay` reject `NaN`, `Infinity`, out-of-bounds latitudes, longitudes, altitudes, and relativistic velocity spoofing at the deserialization boundary before engine physics or shaders execute.
7. **Zero GPS Leaks:** Apps interact with `LocationDisclosurePolicy` and `ZK-Geo` coarse sector proofs; high-precision GPS coordinates are permanently vaulted in the Club Home Node enclave.

---

## 📜 License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
