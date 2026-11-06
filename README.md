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



## 🛡️ Systems, Cryptographic & Operational Security Hardening (19-Pillar Production Lockdown)

The Kuroshio SDK enforces strict mathematical determinism, cryptographic canonicalization, and resource-bounded isolation across **Tier 1 (Olympus)**, **Tier 2 (Lumeria)**, and **Tier 3 (Atlantis)**:

### Part A: Cryptographic & Spatial Primitives
1. **Deterministic Fixed-Point Geodesy for State Channels (`nautilus-world`):**
   - Implements `DeterministicWgs84Coord` (micro-degree latitude/longitude, millimeter altitude) and `DeterministicVec3Fixed` using the `fixed` crate (`I32F32`) paired with an integer square root (`integer_sqrt_u128`).
   - Completely eliminates cross-architecture FMA and transcendental float variance between x86_64, ARM64, and Raspberry Pi Cortex-A72/A76 nodes so honest peers never compute divergent state hashes or trigger false Coral Escrow disputes.
2. **Canonical BCS Serialization & Strict Ed25519 Signatures (`kuroshio-mesh::security`):**
   - All Coral Escrow state channel signing transcripts use `CanonicalCoralStateTranscript` serialized strictly with **BCS** (Binary Canonical Serialization), eliminating non-canonical JSON key reordering, whitespace, and padding ambiguities.
   - Enforces `verify_strict` via `ed25519-dalek` to reject malleable scalar S values and small-order points.
3. **Hardware-Backed Enclave Key Vault & `zeroize` (`kuroshio-mesh::security`):**
   - Private keys never enter raw Rust heap memory (`HardwareKeyVault`: Android Keystore StrongBox, Apple Secure Enclave, Linux TPM 2.0, Windows DPAPI).
   - Ephemeral session secrets (`EphemeralSessionSecret`) derive `Zeroize` and `ZeroizeOnDrop` for immediate zeroization on scope exit.
4. **EIP-712 Domain Separation & Tier 3 Watchtowers (`coral-escrow` & `kuroshio-mesh`):**
   - Monotonic state channel nonces bound to `CoralDomainSeparator` (`chain_id`, `program_id`, `channel_id`).
   - Tier 3 Atlantis Watchtowers (`submit_watchtower_fraud_proof`) penalize stale broadcast attempts during the dispute window, slashing 10% of the cheater's deposit.
5. **Slot-Height Anchored Dispute Windows & Anti-Clock Drift (`coral-escrow`):**
   - Dispute challenge timers (`DISPUTE_CHALLENGE_WINDOW_SLOTS = 9000`) and payouts are anchored strictly to Solana ledger block/slot heights rather than wall-clock time (`SystemTime::now()`).
   - Eliminates premature timeout exploits and protects edge nodes lacking battery-backed RTC chips against clock drift.
6. **Ephemeral Session Certificates & CRDT Revocation Lists (`kuroshio-mesh::security`):**
   - Master root DID keys sign short-lived slot-bounded `EphemeralSessionCertificate` instances; state channels and mesh sessions never sign with root master keys directly.
   - `CrdtRevocationList` broadcasts compact revocation entries over Gossipsub control channels so compromised or physically seized edge nodes are dropped immediately across Atlantis and Lumeria peers without waiting for an upstream ledger sync.
7. **Monotonic Event Ordering via Hybrid Logical Clocks (`kuroshio-mesh::security`):**
   - Implements `HybridLogicalClock` combining ledger slot heights and Lamport logical event counters, guaranteeing strictly causal event ordering without relying on synchronized wall-clock time.

### Part B: Network, Relay & Privacy Hardening
8. **Bounded Deserialization & Anti-OOM Protections on Atlantis Nodes (`kuroshio-mesh::transport`):**
   - Enforces `MAX_NETWORK_FRAME_BYTES` (64 KiB) framing limits before buffers reach the deserializer.
   - Enforces `bincode::DefaultOptions::new().with_limit(MAX_NETWORK_FRAME_BYTES)` (`from_bincode_bounded`), preventing malicious length-prefix attacks from triggering OOM crashes on headless Raspberry Pi Atlantis nodes.
9. **Circuit Relay v2 Quotas & Priority QoS Dispatching (`kuroshio-mesh::transport`):**
   - `AtlantisRelayManager` limits active relay circuits (`DEFAULT_MAX_CIRCUITS_PER_PEER = 2`, `DEFAULT_MAX_CIRCUIT_BYTES = 1 MiB`, `120s` duration limit) preventing mobile CGNAT hole-punching fallbacks from starving edge nodes.
   - `MeshQosPriority` ensures Coral Escrow ledger settlement packets (`PAYLOAD_TYPE_CORAL_LEDGER_SETTLEMENT`) take strict dispatch precedence over high-frequency spatial gossip and Opus swarm audio.
10. **Zero Location & Telemetry Leakage in Squad Discovery (`kuroshio-mesh::transport`):**
    - Public DHT and GossipSub topics use coarse 5-character Geohash grids (`CoarseSpatialDiscoveryKey`, ~4.9km neighborhood resolution).
    - High-precision 64-bit Nautilus coordinates are strictly restricted to end-to-end encrypted tunnels (`EncryptedSquadTunnelPayload`) after mutual DID verification.
11. **Anti-Leak Multiaddr Filtering & Topology Protection (`kuroshio-mesh::transport`):**
    - `sanitize_multiaddrs_for_external_broadcast` strips RFC 1918 private subnets (10.x.x.x, 192.168.x.x, 172.16-31.x.x), loopback, link-local, and Docker bridge IPs from external DHT announcements, completely preventing home lab and edge subnet layout leakage unless verified on local LAN mDNS.
12. **Noise/TLS 1.3 DID Binding, Mobile Handoff Tokens & GossipSub Anti-Eclipse (`kuroshio-mesh::transport`):**
    - HMAC-SHA256 authenticated `HandoffResumptionToken` tied to PeerID, DID, and monotonic `handoff_generation` for seamless Wi-Fi to 5G switching.
    - `GossipSubPeerScorer` enforces quadratic P4 penalties and `/24` IPv4 subnet anti-eclipse caps.

### Part C: Runtime Safety, FFI & Storage
13. **Host-Function TOCTOU Elimination via Atomic Copy-In (`lumeria-host-runner`):**
    - `copy_in_guest_buffer` copies guest linear memory slices into owned host `Vec<u8>` buffers before running validation or executing host operations, completely defeating Time-of-Check to Time-of-Use attacks from secondary guest threads.
14. **FFI Panic Boundaries & Poison-Free State Concurrency (`lumeria-app-sdk`):**
    - All external FFI functions are wrapped in `catch_ffi_unwind`, translating panics into structured `FfiResultCode` and preventing Undefined Behavior (UB) unwinding crashes into host runtimes.
    - `PoisonFreeStateCell` uses `parking_lot::Mutex` to prevent lock poisoning cascading across threads.
    - `AtomicChannelNonceState` isolates state channel transitions from `tokio::select!` future cancellation desync.
15. **Crash-Resilient ACID Storage Engine (`kuroshio-mesh::security` via `redb`):**
    - Implements `AtlantisCrashResilientStorage` powered by `redb` (pure-Rust copy-on-write B-trees with synchronous WAL flushing), preventing filesystem corruption on Raspberry Pi edge nodes during sudden power loss.
    - Ephemeral peer scoring and mesh gossip remain in RAM (tmpfs) to eliminate microSD flash wear.
16. **Seam-Free 3D ECEF Spatial Bounding & Antimeridian Protection (`nautilus-world`):**
    - `EcefSpatialBoundingSphere` and `is_in_ecef_interest_range` calculate all distances and area-of-interest (AoI) inclusion in 3D Cartesian ECEF space $(X, Y, Z)$, completely eliminating polar singularities and the $\pm 180^\circ$ antimeridian wrap seam.
17. **Zero-WASI Capability Sandbox & Instruction Metering (`lumeria-host-runner`):**
    - `WasiSandboxPolicy::strict_zero_wasi()` completely strips ambient filesystem, raw socket, and environment syscalls.
    - Enforces 500k instruction per-frame fuel limits and a 256-page (16 MiB) linear memory ceiling (`request_memory_grow()`).
18. **Olympus Keystone Isolation (`olympus-core` Feature Flag):**
    - `kuroshio-mesh` isolates core data structures under `--no-default-features --features olympus-core`, guaranteeing Olympus never pulls in Tier 2/3 transport or FFI dependencies.
19. **Supply-Chain & Pure-Rust Cross-Compilation Hardening (`deny.toml`):**
    - Drops `deny.toml` (`cargo-deny`) banning `openssl`, `openssl-sys`, and `native-tls` in favor of pure-Rust `rustls` and `ed25519-dalek` for seamless cross-compilation to `aarch64-linux-android` (Android) and `aarch64-unknown-linux-gnu` (Atlantis Pi).

20. **Verified Team Membership & Role Authorization (`kuroshio-mesh`):**
    - `add_member_verified` enforces caller authority (Founder or Officer) and validates cryptographic Ed25519 signatures over `KUROSHIO_TEAM_JOIN_V1:<team_id>:<did>` to prevent unauthorized member spoofing. Banned DIDs (`banned_dids`) are strictly rejected.
21. **Integer Token Units for Treasury Accounting (`kuroshio-mesh`):**
    - Treasury balance represented as `treasury_micro_cyan: u64` (1 CYAN = 1,000,000 micro-CYAN) with saturating arithmetic (`saturating_add`), completely replacing floating-point representation (`f64`) to eliminate financial rounding drift.
22. **Default Secure TLS Endpoints (`kuroshio-mesh`):**
    - Production radio streams (`SirenRadioStream`) and seed offload endpoints (`OlympusSeedEndpoint`) default to TLS-encrypted HTTPS endpoints (`https://radio.kuroshio.atlantis.market` and `https://olympus.kuroshio.atlantis.market`), preventing plaintext transmission of media streams and offload packets. Local dev IP addresses are segregated under `LOCAL_DEV_STREAM_URL`.
23. **Cryptographically Verified Escrow Disputes & Watchtower Proofs (`coral-escrow`):**
    - `submit_dispute_verified` and `submit_watchtower_fraud_proof_verified` perform strict Ed25519 signature checks using `ed25519-dalek` over domain-separated challenge envelopes (`CORAL_STATE_DISPUTE_V1` and `WATCHTOWER_FRAUD_PROOF_V1`), preventing forged dispute claims or invalid watchtower slashing.

---

## 📜 License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
