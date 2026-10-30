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

## 🛡️ Security & Privacy Standard

1. **Zero GPS Leaks:** Apps interact with `LocationDisclosurePolicy` and `ZK-Geo` coarse sector proofs; high-precision GPS coordinates are permanently vaulted in the Club Home Node enclave.
2. **WASM Sandboxing:** 3D mini-apps execute in isolated WebAssembly instances with capability-based security.
3. **Double-Entry Escrow:** In-game purchases and team tips are secured off-ledger through Coral State Channels.

---

## 📜 License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
