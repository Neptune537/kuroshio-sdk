//! # WebRTC DataChannel Signaling, Reticulum Packet Framing & P2P Opus Swarm Audio
//!
//! Provides signaling payloads for establishing direct peer-to-peer WebRTC
//! DataChannels, packetizing offloaded graphics frames, and relaying 48kHz Opus
//! audio frames across squad nodes without single-point unicast bottlenecks.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const PAYLOAD_TYPE_HZB_OCCLUSION: u8 = 0x01;
pub const PAYLOAD_TYPE_PHYSICS_STATE: u8 = 0x02;
pub const PAYLOAD_TYPE_TELEMETRY: u8 = 0x03;
pub const PAYLOAD_TYPE_OPUS_SWARM_AUDIO: u8 = 0x04;
/// Coral Escrow state transition & ledger settlement packet (Highest QoS priority)
pub const PAYLOAD_TYPE_CORAL_LEDGER_SETTLEMENT: u8 = 0x05;

/// Maximum frame size permitted on Atlantis/Lumeria framing codecs (64 KiB)
/// Blocks malicious length-prefixed OOM allocation bombs on headless Raspberry Pi edge nodes.
pub const MAX_NETWORK_FRAME_BYTES: usize = 65_536;
/// Maximum circuit bytes allowed on Atlantis libp2p Circuit Relay v2 before throttling
pub const DEFAULT_MAX_CIRCUIT_BYTES: u64 = 1_048_576; // 1 MiB per circuit
/// Maximum active circuits allowed per mobile peer on Atlantis relay
pub const DEFAULT_MAX_CIRCUITS_PER_PEER: usize = 2;

/// WebRTC Session Description Protocol (SDP) exchange for P2P Gyre mesh links.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SdpSignalEnvelope {
    pub session_id: String,
    pub sender_node_id: String,
    pub target_node_id: String,
    pub signal_type: SdpSignalType,
    pub sdp_payload: String,
    pub candidates: Vec<IceCandidatePayload>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SdpSignalType {
    Offer,
    Answer,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IceCandidatePayload {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_m_line_index: Option<u32>,
}

/// A compact packet frame for Reticulum / WebRTC DataChannel binary transport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshDataPacket {
    /// 8-byte channel identifier
    pub channel_id: u64,
    /// Packet sequence number
    pub sequence: u64,
    /// Payload type (`0x01` = HZB Occlusion, `0x02` = Physics, `0x03` = Telemetry, `0x04` = Opus Swarm Audio)
    pub payload_type: u8,
    /// Binary frame payload
    pub payload: Vec<u8>,
}

impl MeshDataPacket {
    pub fn new(channel_id: u64, sequence: u64, payload_type: u8, payload: Vec<u8>) -> Self {
        Self {
            channel_id,
            sequence,
            payload_type,
            payload,
        }
    }

    /// Wraps a P2P `OpusAudioSwarmFrame` into a `MeshDataPacket` (`0x04`).
    pub fn from_opus_swarm_frame(channel_id: u64, frame: &OpusAudioSwarmFrame) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_vec(frame)?;
        Ok(Self::new(
            channel_id,
            frame.sequence,
            PAYLOAD_TYPE_OPUS_SWARM_AUDIO,
            payload,
        ))
    }

    /// Extracts an `OpusAudioSwarmFrame` if `payload_type == PAYLOAD_TYPE_OPUS_SWARM_AUDIO`.
    pub fn to_opus_swarm_frame(&self) -> Result<Option<OpusAudioSwarmFrame>, serde_json::Error> {
        if self.payload_type != PAYLOAD_TYPE_OPUS_SWARM_AUDIO {
            return Ok(None);
        }
        let frame: OpusAudioSwarmFrame = serde_json::from_slice(&self.payload)?;
        Ok(Some(frame))
    }

    /// Serializes packet to compact binary bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserializes packet from binary bytes with strict frame-length limits.
    /// Rejects any packet exceeding `MAX_NETWORK_FRAME_BYTES` to protect edge nodes from OOM attacks.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        if bytes.len() > MAX_NETWORK_FRAME_BYTES {
            return Err(serde::de::Error::custom(format!(
                "Frame size {} exceeds maximum permitted limit {}",
                bytes.len(),
                MAX_NETWORK_FRAME_BYTES
            )));
        }
        serde_json::from_slice(bytes)
    }

    /// Serializes packet to binary using bounded Bincode options.
    pub fn to_bincode_bounded(&self) -> Result<Vec<u8>, bincode::Error> {
        use bincode::Options;
        bincode::DefaultOptions::new()
            .with_limit(MAX_NETWORK_FRAME_BYTES as u64)
            .serialize(self)
    }

    /// Deserializes packet from binary using strictly bounded Bincode options.
    /// Pre-allocation sizes are capped by the buffer size, preventing $2^{30}$ length prefix OOM attacks.
    pub fn from_bincode_bounded(bytes: &[u8]) -> Result<Self, bincode::Error> {
        use bincode::Options;
        if bytes.len() > MAX_NETWORK_FRAME_BYTES {
            return Err(bincode::ErrorKind::SizeLimit.into());
        }
        bincode::DefaultOptions::new()
            .with_limit(MAX_NETWORK_FRAME_BYTES as u64)
            .deserialize(bytes)
    }
}

/// 48kHz Opus-encoded audio frame relayed peer-to-peer over WebRTC DataChannels
/// for the Seven Daughters Siren Radio & Squad Voice Mesh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpusAudioSwarmFrame {
    /// Broadcast topic (e.g., `siren:frequency:global_stage`)
    pub stream_topic: String,
    /// Active Seven Daughters broadcaster callsign or squad speaker DID
    pub broadcaster_id: String,
    /// Monotonic audio frame sequence number (20ms Opus frames = 50 frames/sec)
    pub sequence: u64,
    /// Sample rate in Hz (canonically 48000)
    pub sample_rate_hz: u32,
    /// Number of channels (2 = stereo)
    pub channels: u8,
    /// Remaining mesh relay hops (TTL) to prevent broadcast storms
    pub ttl_hops: u8,
    /// Raw Opus-encoded audio frame bytes
    pub opus_frame_bytes: Vec<u8>,
}

/// Peer-to-peer WebRTC DataChannel audio swarm router with sequence deduplication.
#[derive(Debug, Default)]
pub struct SwarmAudioRouter {
    seen_sequences: HashSet<(String, u64)>,
    pub peer_node_ids: Vec<String>,
}

impl SwarmAudioRouter {
    pub fn new(peer_node_ids: Vec<String>) -> Self {
        Self {
            seen_sequences: HashSet::new(),
            peer_node_ids,
        }
    }

    /// Ingests an incoming `MeshDataPacket`. If it contains an unseen `OpusAudioSwarmFrame`
    /// with `ttl_hops > 0`, returns `(decoded_frame, relay_packet_with_decremented_ttl)`.
    pub fn receive_and_relay(
        &mut self,
        packet: &MeshDataPacket,
    ) -> Result<Option<(OpusAudioSwarmFrame, Option<MeshDataPacket>)>, serde_json::Error> {
        let Some(mut frame) = packet.to_opus_swarm_frame()? else {
            return Ok(None);
        };

        let key = (frame.stream_topic.clone(), frame.sequence);
        if !self.seen_sequences.insert(key) {
            // Already processed this audio frame in the P2P mesh
            return Ok(None);
        }

        let relay_packet = if frame.ttl_hops > 0 && !self.peer_node_ids.is_empty() {
            frame.ttl_hops -= 1;
            Some(MeshDataPacket::from_opus_swarm_frame(packet.channel_id, &frame)?)
        } else {
            None
        };

        Ok(Some((frame, relay_packet)))
    }
}

// =========================================================================
// 4. P2P NAT TRAVERSAL, STUN/TURN HOLE PUNCHING & LIBP2P PEER DISCOVERY
// =========================================================================

/// Circuit Relay v2 resource allocation and quota manager for Atlantis Tier 3 nodes.
/// Prevents mobile Lumeria clients behind CGNAT from exhausting Atlantis network bandwidth and file descriptors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CircuitRelayQuotaConfig {
    pub max_circuit_bytes: u64,
    pub max_circuit_duration_secs: u32,
    pub max_circuits_per_peer: usize,
    pub max_total_circuits: usize,
}

impl Default for CircuitRelayQuotaConfig {
    fn default() -> Self {
        Self {
            max_circuit_bytes: DEFAULT_MAX_CIRCUIT_BYTES,
            max_circuit_duration_secs: 120,
            max_circuits_per_peer: DEFAULT_MAX_CIRCUITS_PER_PEER,
            max_total_circuits: 32,
        }
    }
}

/// Priority-based QoS packet dispatcher ensuring Coral Escrow ledger traffic takes precedence over spatial/voice gossip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MeshQosPriority {
    GossipAudio = 0,
    GossipSpatial = 1,
    Telemetry = 2,
    CoralLedgerSettlement = 3,
}

impl MeshQosPriority {
    pub fn from_payload_type(payload_type: u8) -> Self {
        match payload_type {
            PAYLOAD_TYPE_CORAL_LEDGER_SETTLEMENT => Self::CoralLedgerSettlement,
            PAYLOAD_TYPE_PHYSICS_STATE => Self::GossipSpatial,
            PAYLOAD_TYPE_TELEMETRY => Self::Telemetry,
            _ => Self::GossipAudio,
        }
    }
}

/// State tracker for an active Circuit Relay session on an Atlantis node.
#[derive(Debug, Clone)]
pub struct ActiveCircuit {
    pub circuit_id: String,
    pub peer_id: String,
    pub bytes_transferred: u64,
    pub opened_at_epoch_sec: u64,
}

/// Manages Circuit Relay v2 quotas and QoS dispatching on Atlantis nodes.
pub struct AtlantisRelayManager {
    pub config: CircuitRelayQuotaConfig,
    pub active_circuits: std::collections::HashMap<String, ActiveCircuit>,
}

impl AtlantisRelayManager {
    pub fn new(config: CircuitRelayQuotaConfig) -> Self {
        Self {
            config,
            active_circuits: std::collections::HashMap::new(),
        }
    }

    /// Attempts to open a new circuit for a mobile peer behind CGNAT.
    pub fn try_open_circuit(&mut self, circuit_id: &str, peer_id: &str, current_epoch_sec: u64) -> Result<(), &'static str> {
        if self.active_circuits.len() >= self.config.max_total_circuits {
            return Err("Relay capacity exhausted: total circuits limit reached");
        }

        let peer_circuit_count = self.active_circuits.values().filter(|c| c.peer_id == peer_id).count();
        if peer_circuit_count >= self.config.max_circuits_per_peer {
            return Err("Peer relay quota exceeded: max circuits per peer reached");
        }

        self.active_circuits.insert(
            circuit_id.to_string(),
            ActiveCircuit {
                circuit_id: circuit_id.to_string(),
                peer_id: peer_id.to_string(),
                bytes_transferred: 0,
                opened_at_epoch_sec: current_epoch_sec,
            },
        );
        Ok(())
    }

    /// Records relayed bandwidth and checks for quota or duration exhaustion.
    pub fn record_transfer(
        &mut self,
        circuit_id: &str,
        bytes: usize,
        current_epoch_sec: u64,
    ) -> Result<(), &'static str> {
        let circuit = self.active_circuits.get_mut(circuit_id).ok_or("Circuit not found")?;

        if current_epoch_sec - circuit.opened_at_epoch_sec > self.config.max_circuit_duration_secs as u64 {
            self.active_circuits.remove(circuit_id);
            return Err("Relay circuit expired by duration limit");
        }

        circuit.bytes_transferred += bytes as u64;
        if circuit.bytes_transferred > self.config.max_circuit_bytes {
            self.active_circuits.remove(circuit_id);
            return Err("Relay circuit terminated: bandwidth quota exceeded");
        }

        Ok(())
    }

    /// Closes an active circuit.
    pub fn close_circuit(&mut self, circuit_id: &str) {
        self.active_circuits.remove(circuit_id);
    }
}

/// Transport protocols supported by Kuroshio P2P nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum P2pTransportProtocol {
    WebRtcDataChannel,
    Libp2pGossipsub,
    Libp2pNoise,
    DirectUdpHolePunch,
}

/// Dynamic STUN/TURN NAT traversal candidate type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatCandidateType {
    /// Direct host LAN interface (e.g., 192.168.1.x, 10.0.0.x).
    Host,
    /// Server Reflexive candidate discovered via STUN binding request.
    ServerReflexive { public_ip: String, public_port: u16 },
    /// Relayed candidate routed via TURN allocation when symmetric NAT prevents direct traversal.
    RelayedTurn { relay_endpoint: String, token: String },
}

/// Decentralized DHT Rendezvous descriptor for peer discovery without central servers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DhtRendezvousAnnouncement {
    pub squad_topic: String,
    pub peer_did: String,
    pub rendezvous_key_sha256: String,
    pub multiaddrs: Vec<String>,
    pub expires_at_epoch_sec: u64,
}

impl DhtRendezvousAnnouncement {
    pub fn new(squad_topic: &str, peer_did: &str, multiaddrs: Vec<String>, ttl_seconds: u64) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(squad_topic.as_bytes());
        hasher.update(peer_did.as_bytes());
        let key_hex = format!("{:x}", hasher.finalize());

        Self {
            squad_topic: squad_topic.to_string(),
            peer_did: peer_did.to_string(),
            rendezvous_key_sha256: key_hex,
            multiaddrs,
            expires_at_epoch_sec: 1793316000 + ttl_seconds,
        }
    }

    pub fn matches_topic(&self, target_topic: &str) -> bool {
        self.squad_topic == target_topic
    }
}

/// Coarse geohash / H3 cell representation for squad discovery and peer matchmaking.
/// Prevents physical GPS or high-precision Nautilus coordinate leakage over public DHT and GossipSub topics.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CoarseSpatialDiscoveryKey {
    /// Truncated geohash (e.g. 5-character string, ~4.9km x 4.9km city-neighborhood resolution)
    pub geohash_prefix: String,
    /// Squad activity tag / game mode
    pub activity_tag: String,
}

impl CoarseSpatialDiscoveryKey {
    pub const COARSE_GEOHASH_PRECISION: usize = 5;

    pub fn from_coordinates(lat: f64, lon: f64, activity: &str) -> Self {
        let full_geohash = encode_simple_geohash(lat, lon, Self::COARSE_GEOHASH_PRECISION);
        Self {
            geohash_prefix: full_geohash,
            activity_tag: activity.to_string(),
        }
    }

    /// Derives the public libp2p GossipSub topic string.
    pub fn to_discovery_topic(&self) -> String {
        format!("kuroshio:discovery:{}:{}", self.geohash_prefix, self.activity_tag)
    }
}

/// Simple deterministic geohash encoder without external C libraries.
pub fn encode_simple_geohash(latitude: f64, longitude: f64, precision: usize) -> String {
    const BASE32: &[u8] = b"0123456789bcdefghjkmnpqrstuvwxyz";
    let mut lat_interval = (-90.0, 90.0);
    let mut lon_interval = (-180.0, 180.0);
    let mut is_even = true;
    let mut bit = 0;
    let mut ch = 0;
    let mut geohash = String::with_capacity(precision);

    while geohash.len() < precision {
        if is_even {
            let mid = (lon_interval.0 + lon_interval.1) / 2.0;
            if longitude >= mid {
                ch |= 1 << (4 - bit);
                lon_interval.0 = mid;
            } else {
                lon_interval.1 = mid;
            }
        } else {
            let mid = (lat_interval.0 + lat_interval.1) / 2.0;
            if latitude >= mid {
                ch |= 1 << (4 - bit);
                lat_interval.0 = mid;
            } else {
                lat_interval.1 = mid;
            }
        }

        is_even = !is_even;
        if bit < 4 {
            bit += 1;
        } else {
            geohash.push(BASE32[ch as usize] as char);
            bit = 0;
            ch = 0;
        }
    }
    geohash
}

/// Encrypted mutually authenticated squad tunnel payload for high-precision coordinate exchange.
/// High-precision coordinates are strictly forbidden in public topics and can only be transmitted here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedSquadTunnelPayload {
    pub sender_did: String,
    pub recipient_did: String,
    pub session_id: String,
    pub encrypted_ciphertext_hex: String,
    pub auth_tag_hex: String,
}

/// Automated STUN/TURN hole punch coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatTraversalCoordinator {
    pub local_peer_id: String,
    pub discovered_candidates: Vec<NatCandidateType>,
    pub active_gossip_topics: HashSet<String>,
}

impl NatTraversalCoordinator {
    pub fn new(peer_id: &str) -> Self {
        Self {
            local_peer_id: peer_id.to_string(),
            discovered_candidates: vec![NatCandidateType::Host],
            active_gossip_topics: HashSet::new(),
        }
    }

    pub fn add_stun_candidate(&mut self, public_ip: &str, public_port: u16) {
        self.discovered_candidates.push(NatCandidateType::ServerReflexive {
            public_ip: public_ip.to_string(),
            public_port,
        });
    }

    pub fn add_turn_fallback(&mut self, relay_endpoint: &str, token: &str) {
        self.discovered_candidates.push(NatCandidateType::RelayedTurn {
            relay_endpoint: relay_endpoint.to_string(),
            token: token.to_string(),
        });
    }

    pub fn subscribe_gossip_topic(&mut self, topic: &str) {
        self.active_gossip_topics.insert(topic.to_string());
    }

    /// Generates a complete WebRTC / libp2p multiaddress string vector for peer connection.
    pub fn export_multiaddrs(&self) -> Vec<String> {
        let mut addrs = Vec::new();
        for candidate in &self.discovered_candidates {
            match candidate {
                NatCandidateType::Host => {
                    addrs.push(format!("/ip4/127.0.0.1/udp/0/webrtc-direct/p2p/{}", self.local_peer_id));
                }
                NatCandidateType::ServerReflexive { public_ip, public_port } => {
                    addrs.push(format!("/ip4/{}/udp/{}/webrtc-direct/p2p/{}", public_ip, public_port, self.local_peer_id));
                }
                NatCandidateType::RelayedTurn { relay_endpoint, .. } => {
                    addrs.push(format!("/dns4/{}/tcp/443/wss/p2p-circuit/p2p/{}", relay_endpoint, self.local_peer_id));
                }
            }
        }
        addrs
    }

    /// Sanitizes and filters multiaddresses before broadcasting to external DHT or GossipSub networks.
    /// Completely strips RFC 1918 private subnets (192.168.x.x, 10.x.x.x, 172.16-31.x.x), loopback (127.x.x.x),
    /// Docker bridge interfaces, and link-local ranges, preventing home-lab topology leakage unless on local LAN mDNS.
    pub fn sanitize_multiaddrs_for_external_broadcast(&self, is_local_lan_mdns: bool) -> Vec<String> {
        let addrs = self.export_multiaddrs();
        if is_local_lan_mdns {
            return addrs;
        }

        addrs
            .into_iter()
            .filter(|addr| {
                if let Some(ip_str) = extract_ip4_from_multiaddr(addr) {
                    if let Ok(ip) = ip_str.parse::<std::net::Ipv4Addr>() {
                        // Strip private, loopback, link-local, and broadcast
                        return !ip.is_private() && !ip.is_loopback() && !ip.is_link_local() && !ip.is_broadcast();
                    }
                }
                // Allow public DNS4 relay endpoints
                addr.contains("/dns4/")
            })
            .collect()
    }
}

/// Helper parsing IPv4 segment from libp2p multiaddr string.
fn extract_ip4_from_multiaddr(addr: &str) -> Option<&str> {
    if let Some(rest) = addr.strip_prefix("/ip4/") {
        rest.split('/').next()
    } else {
        None
    }
}

// =========================================================================
// 4. HARDENED LIBP2P NOISE / TLS 1.3 HANDOFF RESUMPTION & GOSSIPSUB v1.1 SCORING
// =========================================================================

/// Cryptographic transport protocol enforced on all `kuroshio-mesh` libp2p connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeshTransportSecurity {
    /// Noise XX handshake pattern (`Noise_XX_25519_ChaChaPoly_SHA256`) bound to Ed25519 PeerID + DID.
    NoiseXxEd25519,
    /// TLS 1.3 mutual authentication (`libp2p-tls`) with self-signed peer certificate extension.
    Tls13MutualPeerId,
}

/// Cryptographically bound session resumption ticket allowing a Tier 2 mobile device
/// (switching between Wi-Fi and 5G/LTE cellular IPs) to migrate its active P2P session
/// without dropping state or allowing IP-spoofing hijacks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffResumptionToken {
    pub session_id: String,
    pub peer_id: String,
    pub bound_did: String,
    pub handoff_generation: u32,
    pub previous_ip_hash: String,
    pub new_ip_hash: String,
    pub expires_at_epoch_sec: u64,
    /// Keyed SHA-256 MAC computed over `(session_id || peer_id || bound_did || handoff_generation || new_ip_hash)`
    pub session_mac_hex: String,
}

impl HandoffResumptionToken {
    /// Issues a signed mobile network handoff resumption token using the session's `ZeroizeOnDrop` ephemeral secret.
    pub fn issue(
        session_id: &str,
        peer_id: &str,
        bound_did: &str,
        handoff_generation: u32,
        previous_ip: &str,
        new_ip: &str,
        expires_at_epoch_sec: u64,
        session_secret: &crate::security::EphemeralSessionSecret,
    ) -> Self {
        use sha2::{Digest, Sha256};
        let prev_hash: String = Sha256::digest(previous_ip.as_bytes()).iter().map(|b| format!("{:02x}", b)).collect();
        let new_hash: String = Sha256::digest(new_ip.as_bytes()).iter().map(|b| format!("{:02x}", b)).collect();

        let msg = format!(
            "HANDOFF_V1:{}:{}:{}:{}:{}:{}",
            session_id, peer_id, bound_did, handoff_generation, new_hash, expires_at_epoch_sec
        );
        let mac = session_secret.sign_digest(msg.as_bytes());
        let session_mac_hex = mac.iter().map(|b| format!("{:02x}", b)).collect();

        Self {
            session_id: session_id.to_string(),
            peer_id: peer_id.to_string(),
            bound_did: bound_did.to_string(),
            handoff_generation,
            previous_ip_hash: prev_hash,
            new_ip_hash: new_hash,
            expires_at_epoch_sec,
            session_mac_hex,
        }
    }

    /// Verifies that the mobile handoff token is authentic, unexpired, and strictly monotonic in `handoff_generation`.
    pub fn verify_handoff(
        &self,
        expected_peer_id: &str,
        current_generation: u32,
        current_epoch_sec: u64,
        session_secret: &crate::security::EphemeralSessionSecret,
    ) -> Result<(), &'static str> {
        if self.peer_id != expected_peer_id {
            return Err("PeerID mismatch on mobile handoff");
        }
        if self.handoff_generation <= current_generation {
            return Err("Stale handoff generation (replay attempt blocked)");
        }
        if current_epoch_sec > self.expires_at_epoch_sec {
            return Err("Handoff resumption token expired");
        }

        let msg = format!(
            "HANDOFF_V1:{}:{}:{}:{}:{}:{}",
            self.session_id,
            self.peer_id,
            self.bound_did,
            self.handoff_generation,
            self.new_ip_hash,
            self.expires_at_epoch_sec
        );
        let expected_mac = session_secret.sign_digest(msg.as_bytes());
        let expected_hex: String = expected_mac.iter().map(|b| format!("{:02x}", b)).collect();

        if self.session_mac_hex != expected_hex {
            return Err("Invalid handoff resumption MAC signature");
        }
        Ok(())
    }
}

/// Hardened GossipSub v1.1 peer scoring, rate-limiting, and anti-eclipse configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GossipSubScoringConfig {
    /// Score threshold below which a peer is detached from gossip meshes (`-50.0`)
    pub gossip_threshold: f64,
    /// Score threshold below which all incoming messages from the peer are dropped (`-100.0`)
    pub publish_threshold: f64,
    /// Score threshold below which the peer is disconnected and blacklisted (`-200.0`)
    pub graylist_threshold: f64,
    /// Maximum messages allowed per peer per second per topic before rate-limit penalty triggers
    pub max_messages_per_sec: u32,
    /// Maximum peers allowed from the same `/24` IPv4 subnet to prevent Eclipse attacks
    pub max_peers_per_ip_subnet: usize,
}

impl Default for GossipSubScoringConfig {
    fn default() -> Self {
        Self {
            gossip_threshold: -50.0,
            publish_threshold: -100.0,
            graylist_threshold: -200.0,
            max_messages_per_sec: 25,
            max_peers_per_ip_subnet: 2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PeerScoreState {
    pub score: f64,
    pub messages_in_window: u32,
    pub invalid_messages_count: u32,
    pub ip_subnet_prefix: String,
}

/// Enforces GossipSub v1.1 peer scoring, rate-limiting, and `/24` subnet diversity (Anti-Eclipse).
pub struct GossipSubPeerScorer {
    pub config: GossipSubScoringConfig,
    pub peers: std::collections::HashMap<String, PeerScoreState>,
}

impl GossipSubPeerScorer {
    pub fn new(config: GossipSubScoringConfig) -> Self {
        Self {
            config,
            peers: std::collections::HashMap::new(),
        }
    }

    /// Registers a peer while enforcing `/24` subnet diversity to block Eclipse attacks on Atlantis/Lumeria nodes.
    pub fn register_peer(&mut self, peer_id: &str, ipv4_subnet_24: &str) -> Result<(), &'static str> {
        let same_subnet_count = self
            .peers
            .values()
            .filter(|p| p.ip_subnet_prefix == ipv4_subnet_24)
            .count();

        if same_subnet_count >= self.config.max_peers_per_ip_subnet {
            return Err("Eclipse protection triggered: maximum peers per /24 IP subnet exceeded");
        }

        self.peers.insert(
            peer_id.to_string(),
            PeerScoreState {
                score: 0.0,
                messages_in_window: 0,
                invalid_messages_count: 0,
                ip_subnet_prefix: ipv4_subnet_24.to_string(),
            },
        );
        Ok(())
    }

    /// Evaluates an incoming GossipSub message from `peer_id`, applying rate-limiting and validity scoring.
    pub fn validate_and_score_message(
        &mut self,
        peer_id: &str,
        is_signature_and_schema_valid: bool,
    ) -> Result<f64, &'static str> {
        let state = self.peers.get_mut(peer_id).ok_or("Unregistered peer")?;

        if state.score <= self.config.graylist_threshold {
            return Err("Peer is graylisted due to low GossipSub v1.1 score");
        }

        state.messages_in_window += 1;
        if state.messages_in_window > self.config.max_messages_per_sec {
            state.score -= 35.0; // P7 rate-limit excess penalty
            return Err("GossipSub rate limit exceeded; penalty applied");
        }

        if !is_signature_and_schema_valid {
            state.invalid_messages_count += 1;
            // P4 invalid message delivery penalty (quadratic decay)
            let penalty = 50.0 * (state.invalid_messages_count as f64).powi(2);
            state.score -= penalty;
            return Err("Invalid GossipSub message rejected; P4 score penalty applied");
        }

        // Reward valid first-delivery mesh participation
        state.score = (state.score + 1.5).min(100.0);
        Ok(state.score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdp_signaling_envelope() {
        let envelope = SdpSignalEnvelope {
            session_id: "sess_p2p_01".to_string(),
            sender_node_id: "node_alpha".to_string(),
            target_node_id: "node_beta".to_string(),
            signal_type: SdpSignalType::Offer,
            sdp_payload: "v=0\r\no=- 12345 2 IN IP4 127.0.0.1\r\n".to_string(),
            candidates: vec![IceCandidatePayload {
                candidate: "candidate:1 1 UDP 2130706431 192.168.1.100 50000 typ host".to_string(),
                sdp_mid: Some("0".to_string()),
                sdp_m_line_index: Some(0),
            }],
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: SdpSignalEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(envelope, decoded);
    }

    #[test]
    fn test_mesh_data_packet_roundtrip() {
        let packet = MeshDataPacket::new(101, 1, 0x01, vec![10, 20, 30, 40]);
        let bytes = packet.to_bytes().unwrap();
        let unpacked = MeshDataPacket::from_bytes(&bytes).unwrap();
        assert_eq!(packet, unpacked);
    }

    #[test]
    fn test_webrtc_opus_swarm_audio_relay_and_dedup() {
        let opus_frame = OpusAudioSwarmFrame {
            stream_topic: "siren:frequency:global_stage".into(),
            broadcaster_id: "SIREN-03-ASTEROPE".into(),
            sequence: 5001,
            sample_rate_hz: 48000,
            channels: 2,
            ttl_hops: 3,
            opus_frame_bytes: vec![0xF8, 0xFF, 0xFE, 0x10, 0x20],
        };

        let packet = MeshDataPacket::from_opus_swarm_frame(432, &opus_frame).unwrap();
        let mut router = SwarmAudioRouter::new(vec!["node_peer_02".into(), "node_peer_03".into()]);

        // First receive decodes audio and emits a decremented-TTL relay packet
        let (decoded, relay) = router.receive_and_relay(&packet).unwrap().unwrap();
        assert_eq!(decoded.sequence, 5001);
        let relay_pkt = relay.expect("Should emit relay packet when TTL > 0");
        let relayed_frame = relay_pkt.to_opus_swarm_frame().unwrap().unwrap();
        assert_eq!(relayed_frame.ttl_hops, 2);

        // Duplicate packet arrival is suppressed by deduplication filter
        let dup = router.receive_and_relay(&packet).unwrap();
        assert!(dup.is_none());
    }

    #[test]
    fn test_nat_traversal_coordinator_and_dht_announcements() {
        let mut nat = NatTraversalCoordinator::new("12D3KooWTestPeerId");
        nat.add_stun_candidate("203.0.113.45", 54321);
        nat.add_turn_fallback("turn.kuroshio.atlantis.market", "ephemeral_token_abc");
        nat.subscribe_gossip_topic("squad:tactical_seraphim");

        let addrs = nat.export_multiaddrs();
        assert_eq!(addrs.len(), 3);
        assert!(addrs[1].contains("203.0.113.45"));
        assert!(addrs[2].contains("turn.kuroshio.atlantis.market"));

        let ann = DhtRendezvousAnnouncement::new(
            "squad:tactical_seraphim",
            "did:atlantis:player1",
            addrs,
            3600,
        );
        assert!(ann.matches_topic("squad:tactical_seraphim"));
        assert_eq!(ann.rendezvous_key_sha256.len(), 64);
    }

    #[test]
    fn test_mobile_handoff_resumption_token_and_gossipsub_anti_eclipse() {
        let secret = crate::security::EphemeralSessionSecret::from_bytes([0x42u8; 32]);
        let token = HandoffResumptionToken::issue(
            "sess_mobile_99",
            "12D3KooWMobilePeer",
            "did:atlantis:pilot_7",
            1,
            "192.168.1.44",
            "172.56.21.89", // 5G cellular handoff IP
            2_000_000_000,
            &secret,
        );

        // Valid handoff generation 1 > 0 succeeds
        assert!(token.verify_handoff("12D3KooWMobilePeer", 0, 1_900_000_000, &secret).is_ok());

        // Replay of generation 1 <= 1 fails
        assert!(token.verify_handoff("12D3KooWMobilePeer", 1, 1_900_000_000, &secret).is_err());

        // Spoofed PeerID fails
        assert!(token.verify_handoff("12D3KooWAttacker", 0, 1_900_000_000, &secret).is_err());

        // Anti-Eclipse /24 subnet cap & GossipSub v1.1 invalid message scoring
        let mut scorer = GossipSubPeerScorer::new(GossipSubScoringConfig::default());
        assert!(scorer.register_peer("peer_a", "203.0.113.0/24").is_ok());
        assert!(scorer.register_peer("peer_b", "203.0.113.0/24").is_ok());
        // 3rd peer from the exact same /24 subnet is blocked by Eclipse protection
        assert!(scorer.register_peer("peer_c_sybil", "203.0.113.0/24").is_err());

        // Valid message increases peer score
        let s1 = scorer.validate_and_score_message("peer_a", true).unwrap();
        assert!(s1 > 0.0);

        // Invalid messages trigger quadratic P4 penalty and drop peer below graylist threshold
        let _ = scorer.validate_and_score_message("peer_b", false);
        let _ = scorer.validate_and_score_message("peer_b", false);
        assert!(scorer.validate_and_score_message("peer_b", true).is_err());
    }

    #[test]
    fn test_atlantis_relay_manager_quotas_and_qos_priority() {
        let mut relay = AtlantisRelayManager::new(CircuitRelayQuotaConfig {
            max_circuit_bytes: 1000,
            max_circuit_duration_secs: 10,
            max_circuits_per_peer: 1,
            max_total_circuits: 2,
        });

        // 1. Open circuit succeeds
        assert!(relay.try_open_circuit("circ_1", "peer_mobile_1", 100).is_ok());

        // 2. Second circuit from same peer exceeds quota
        assert!(relay.try_open_circuit("circ_2", "peer_mobile_1", 100).is_err());

        // 3. Bandwidth transfer tracking and limit enforcement
        assert!(relay.record_transfer("circ_1", 500, 102).is_ok());
        // Exceeding 1000 bytes terminates circuit
        assert!(relay.record_transfer("circ_1", 600, 103).is_err());
        assert!(!relay.active_circuits.contains_key("circ_1"));

        // 4. QoS priority levels
        assert!(MeshQosPriority::from_payload_type(PAYLOAD_TYPE_CORAL_LEDGER_SETTLEMENT) > MeshQosPriority::from_payload_type(PAYLOAD_TYPE_OPUS_SWARM_AUDIO));
    }

    #[test]
    fn test_bounded_bincode_deserialization_and_coarse_geohash() {
        let packet = MeshDataPacket::new(999, 1, PAYLOAD_TYPE_CORAL_LEDGER_SETTLEMENT, vec![1, 2, 3, 4]);

        // Bounded bincode roundtrip
        let bytes = packet.to_bincode_bounded().unwrap();
        let decoded = MeshDataPacket::from_bincode_bounded(&bytes).unwrap();
        assert_eq!(packet, decoded);

        // Frame exceeding MAX_NETWORK_FRAME_BYTES is rejected
        let oversized = vec![0u8; MAX_NETWORK_FRAME_BYTES + 10];
        assert!(MeshDataPacket::from_bytes(&oversized).is_err());
        assert!(MeshDataPacket::from_bincode_bounded(&oversized).is_err());

        // Coarse spatial discovery truncates coordinates into privacy-preserving topic
        let key = CoarseSpatialDiscoveryKey::from_coordinates(35.6586, 139.7454, "tactical_wing");
        assert_eq!(key.geohash_prefix.len(), 5);
        let topic = key.to_discovery_topic();
        assert!(topic.starts_with("kuroshio:discovery:xn7"));
    }

    #[test]
    fn test_multiaddr_sanitization_blocks_private_ip_leakage() {
        let mut nat = NatTraversalCoordinator::new("12D3KooWSanitizeTest");
        // Add private LAN, public STUN, and relayed TURN
        nat.add_stun_candidate("192.168.1.150", 4001); // Private RFC 1918 (should be filtered)
        nat.add_stun_candidate("203.0.113.88", 54321);  // Public IP (should remain)
        nat.add_turn_fallback("turn.relay.atlantis.market", "token_xyz");

        // 1. External broadcast strips 127.0.0.1 and 192.168.1.150
        let external_addrs = nat.sanitize_multiaddrs_for_external_broadcast(false);
        assert_eq!(external_addrs.len(), 2);
        assert!(!external_addrs.iter().any(|a| a.contains("127.0.0.1")));
        assert!(!external_addrs.iter().any(|a| a.contains("192.168.1.150")));
        assert!(external_addrs.iter().any(|a| a.contains("203.0.113.88")));
        assert!(external_addrs.iter().any(|a| a.contains("turn.relay.atlantis.market")));

        // 2. Local LAN mDNS keeps all discovered candidates
        let local_addrs = nat.sanitize_multiaddrs_for_external_broadcast(true);
        assert_eq!(local_addrs.len(), 4);
    }
}
