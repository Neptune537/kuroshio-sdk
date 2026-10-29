# Kuroshio SDK: Open World Geodesy, Mesh Teams & Lumeria Sandboxed App SDK

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

The official public open SDK for building **decentralized teams, social squads, and spatial WebAssembly applications** across the **Nautilus 3D Planetary World** and **Kuroshio Gyre** mesh network.

---

## 🏛️ Architecture Overview

The Kuroshio SDK provides clean-room, public developer abstractions while preserving and protecting internal engine IP (proprietary GPU shaders, direct-to-VRAM DRM decryption keys, and private hardware GPS enclaves remain sealed in private host runtimes).

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
| [`crates/nautilus-world`](crates/nautilus-world) | 64-bit WGS84 ECEF geodesy, EGM96/EGM2008 geoid undulation ($N$), Kármán line Coriolis kinematics, and `DMat4` high/low `f32` WebGPU splitters. | `0.1.0` |
| [`crates/kuroshio-mesh`](crates/kuroshio-mesh) | Team & squad registration, Groth16/Halo2 `ZK-Geo` polygon circuit proofs with hardware enclave attestation, PoH micro-VDF Sybil protection & 1-in-50 redundant Coral slashing, and WebRTC Opus Swarm Audio P2P relay. | `0.1.0` |
| [`crates/lumeria-app-sdk`](crates/lumeria-app-sdk) | Sandboxed `LumeriaApp` guest trait, Wasm Component Model `.wit` interface (`wit/lumeria-guest.wit`), 120Hz zero-copy `SharedInputRingBuffer`, and Wasmtime fuel budgeting (`WasmFuelMeter`). | `0.1.0` |
| [`crates/kuroshio-mock-node`](crates/kuroshio-mock-node) | Local mock Atlantis/Triton node with sequence-signed Coral state channel dispute timers and Solana SPL/Anchor PDA settlement instruction builders. | `0.1.0` |

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

## 🛡️ Security & Privacy Standard

1. **Zero GPS Leaks:** Apps interact with `LocationDisclosurePolicy` and `ZK-Geo` coarse sector proofs; high-precision GPS coordinates are permanently vaulted in the Club Home Node enclave.
2. **WASM Sandboxing:** 3D mini-apps execute in isolated WebAssembly instances with capability-based security.
3. **Double-Entry Escrow:** In-game purchases and team tips are secured off-ledger through Coral State Channels.

---

## 📜 License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
