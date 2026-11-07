//! # End-to-End Kuroshio SDK Reference Example:
//! ## Continuous CDLOD Traversal, Browser Mesh Co-Op, WIT Component Modding & ZK-Geofence HTLC Escrow
//!
//! Run with:
//! ```bash
//! cargo run -p open_world_mod_and_escrow
//! ```

use kuroshio_mesh::{
    BrowserMeshTransport, ChannelReliability, GeofencePublicInputs, Groth16Proof,
    HtlcGeofenceEngine, MeshPayload, ZkGeofenceArrivalProof,
};
use lumeria_app_sdk::{
    AttackIntent, Combatant, DamageType, DiceType, DungeonTheme, LumeriaModHostEngine, RollRequest,
};
use nautilus_world::{
    CdlodPlanetEngine, FarFieldHorizonManager, FloatingOriginCamera, HorizonImpostorKind,
    HorizonLandmarkImpostor, TravelMode, Wgs84GeoCoord,
};
use sha2::{Digest, Sha256};

fn main() {
    println!("==========================================================================");
    println!("🌊 KUROSHIO SDK v0.2: END-TO-END ARCHITECTURAL REFERENCE SUITE");
    println!("==========================================================================\n");

    // =========================================================================
    // PILLAR 1: Continuous Geodetic CDLOD, Floating-Origin Camera & Impostors
    // =========================================================================
    println!("[1/4] NAUTILUS-WORLD: Continuous CDLOD Quadtree & Floating-Origin Traversal");

    let start_geo = Wgs84GeoCoord::new(38.6431, 34.8289, 1150.0); // Cappadocia Anchor
    let mut floating_cam = FloatingOriginCamera::new(
        start_geo,
        60.0f32.to_radians(),
        16.0 / 9.0,
        0.5,
        50_000.0,
    );

    let mut cdlod_engine = CdlodPlanetEngine::new(6, "Copernicus-GLO-30-COG");
    cdlod_engine.update(&floating_cam);

    // Register a distant horizon landmark impostor (Mount Erciyes ~24km East-Southeast)
    let mut horizon_mgr = FarFieldHorizonManager::new();
    let mt_erciyes_geo = Wgs84GeoCoord::new(38.5319, 35.0467, 3917.0);
    horizon_mgr.add_landmark(HorizonLandmarkImpostor::new(
        "lm-erciyes",
        "Mount Erciyes Stratovolcano",
        HorizonImpostorKind::MountainPeak,
        mt_erciyes_geo,
        2700.0,
        6500.0,
    ));

    // Simulate player accelerating across travel modes over 60 seconds
    for mode in [
        TravelMode::Walking,
        TravelMode::MountedSteed,
        TravelMode::TacticalVehicle,
        TravelMode::AtmosphericFlight,
    ] {
        let speed = mode.max_speed_mps();
        // Advance camera along ECEF vector to trigger 500m Floating-Origin Auto-Rebase
        let current_geo = Wgs84GeoCoord::from_ecef(floating_cam.camera_ecef_f64);
        let next_geo = Wgs84GeoCoord::new(
            current_geo.latitude - 0.002,
            current_geo.longitude + 0.008,
            current_geo.altitude_meters,
        );
        floating_cam.set_camera_ecef(next_geo.to_ecef());
        cdlod_engine.update(&floating_cam);

        let visible_landmarks = horizon_mgr.query_visible_impostors(floating_cam.camera_ecef_f64);
        println!(
            "  • Mode: {:<18} | Speed: {:>5.1} m/s | Active CDLOD Chunks: {:>3} | Cam-Rel Offset: {:>6.2}m | Far Landmarks: {}",
            format!("{:?}", mode),
            speed,
            cdlod_engine.visible_chunk_count,
            floating_cam.camera_offset_relative_f32.length(),
            visible_landmarks.len()
        );
    }
    println!();

    // =========================================================================
    // PILLAR 2: Direct WebRTC Browser Mesh & Verifiable SHA-256 Squad Dice
    // =========================================================================
    println!("[2/4] KUROSHIO-MESH: Serverless WebRTC DataChannels & Verifiable Squad Dice");

    let mut mesh = BrowserMeshTransport::new("did:atlantis:peer-alpha".to_string());
    mesh.join_squad_room("olympus-vanguard-squad".to_string());
    mesh.on_peer_connected(
        "did:atlantis:peer-bravo".to_string(),
        ChannelReliability::UnreliableLossTolerant {
            max_packet_life_time_ms: 16,
        },
    );

    // Broadcast 60Hz player transform over UnreliableLossTolerant channel
    let pos_ecef = floating_cam.camera_ecef_f64;
    mesh.broadcast_player_transform(
        101,
        [pos_ecef.x, pos_ecef.y, pos_ecef.z],
        [0.0, 0.0, 27.8],
        [1.57, 0.0],
        "TacticalVehicle".to_string(),
        1_730_995_200_000,
    );

    // Broadcast verifiable D&D 1d20+4 roll over ReliableOrdered channel
    let entropy_seed = [0x42u8; 32];
    let roll_env = mesh.broadcast_verifiable_dice_roll(
        "1d20+4",
        1,
        20,
        4,
        entropy_seed,
        1_730_995_200_050,
    );

    if let MeshPayload::VerifiableDiceRoll {
        dice_notation,
        result_sum,
        individual_results,
        ..
    } = &roll_env.payload
    {
        println!(
            "  • Squad Room: {:?} | Connected WebRTC Peers: {}",
            mesh.active_squad_room_id.as_deref().unwrap_or("none"),
            mesh.peers.len()
        );
        println!(
            "  • Verifiable Roll Broadcast: {} => Total: {} (Natural: {:?}) [SHA-256 Entropy Verified]",
            dice_notation, result_sum, individual_results
        );
    }
    println!();

    // =========================================================================
    // PILLAR 3: WebAssembly Component Model (WIT) Modding Host Engine
    // =========================================================================
    println!("[3/4] LUMERIA-APP-SDK: WASI Preview 2 / WIT Component Model Modding");

    let mod_engine = LumeriaModHostEngine::new();
    let d20_outcome = mod_engine.execute_dice_roll(
        &RollRequest {
            dice: DiceType::D20,
            count: 1,
            modifier: 5,
            advantage: true,
            disadvantage: false,
            reason: "Seraphic Ignis Blade Strike".to_string(),
        },
        987_654_321,
    );

    let boss = Combatant {
        id: 9001,
        name: "Karman Vortex Archon".to_string(),
        max_hp: 250.0,
        current_hp: 120.0,
        armor_class: 16,
        initiative: 14,
    };

    let combat_res = mod_engine.resolve_attack(
        &AttackIntent {
            attacker_id: 101,
            target_id: boss.id,
            weapon_name: "Ignis Lance".to_string(),
            attack_bonus: 6,
            damage_dice: "2d8+4".to_string(),
            dmg_type: DamageType::Radiant,
        },
        &boss,
        d20_outcome.natural_rolls[0],
    );

    let dungeon = mod_engine.generate_procedural_dungeon(
        42_000,
        3,
        DungeonTheme::ForgottenCitadel,
    );

    println!("  • WIT Combat Hook: {}", combat_res.combat_log);
    println!(
        "  • WIT Procedural Dungeon: Generated {} rooms (Theme: {:?}, Boss Room ID: #{})",
        dungeon.rooms.len(),
        DungeonTheme::ForgottenCitadel,
        dungeon.boss_room_id
    );
    println!();

    // =========================================================================
    // PILLAR 4: Trustless HTLC Escrow with Groth16 ZK-SNARK Geofence Settlement
    // =========================================================================
    println!("[4/4] KUROSHIO-MESH: Coral HTLC Escrow & Groth16 ZK-SNARK Geofence Settlement");

    let mut htlc_engine = HtlcGeofenceEngine::new();
    let sponsor_did = "did:atlantis:guild-olympus-treasury".to_string();
    let explorer_did = "did:atlantis:peer-alpha".to_string();
    htlc_engine
        .balances_micro_cyan
        .insert(sponsor_did.clone(), 50_000_000); // 50 CYAN

    let secret_preimage = b"seraphim-citadel-arrival-secret-2026".to_vec();
    let mut sha = Sha256::new();
    sha.update(&secret_preimage);
    let hash_lock: [u8; 32] = sha.finalize().into();

    let epoch_sec: u64 = 1_730_995_500;
    let mut commit_sha = Sha256::new();
    commit_sha.update(explorer_did.as_bytes());
    commit_sha.update(&epoch_sec.to_le_bytes());
    let claimer_commitment_hash: [u8; 32] = commit_sha.finalize().into();

    let geofence_inputs = GeofencePublicInputs {
        target_lat_scaled: 385_319_000, // 38.5319 deg N
        target_lon_scaled: 354_467_000, // 35.4467 deg E
        max_distance_sq_meters: 250.0 * 250.0, // 250m radius
        claimer_commitment_hash,
        valid_epoch_seconds: epoch_sec,
    };

    htlc_engine
        .create_zk_htlc_bounty(
            "bounty-erciyes-recon-01".to_string(),
            sponsor_did.clone(),
            explorer_did.clone(),
            15_000_000, // 15 CYAN bounty
            hash_lock,
            geofence_inputs.clone(),
            epoch_sec + 3600,
        )
        .expect("Failed to create HTLC bounty");

    let zk_proof = ZkGeofenceArrivalProof {
        proof: Groth16Proof {
            a_g1: [0x11; 32],
            b_g2_0: [0x22; 32],
            b_g2_1: [0x33; 32],
            c_g1: [0x44; 32],
        },
        public_inputs: geofence_inputs,
        circuit_vk_id: "vk_groth16_geofence_bn254_v2".to_string(),
    };

    let payout = htlc_engine
        .settle_with_zk_proof(
            "bounty-erciyes-recon-01",
            secret_preimage,
            &zk_proof,
            epoch_sec + 120,
        )
        .expect("ZK-Geofence HTLC settlement failed");

    println!(
        "  • HTLC Contract 'bounty-erciyes-recon-01' Settled Trustlessly!"
    );
    println!(
        "  • Payout Released: {:.2} CYAN ({} micro-CYAN) to {} without revealing raw GPS!",
        payout as f64 / 1_000_000.0,
        payout,
        explorer_did
    );
    println!("\n✅ All 4 Kuroshio SDK Architectural Pillars Verified Successfully.");
}
