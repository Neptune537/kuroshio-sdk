//! # Direct Browser WebRTC DataChannels & libp2p Gossipsub Mesh Transport
//!
//! Provides WebRTC DataChannel browser-mesh primitives and verifiable turn/dice synchronization
//! for serverless 2-player and squad co-op exploration.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Channel delivery reliability mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelReliability {
    /// Ordered, reliable stream (TCP-like over SCTP/WebRTC DataChannel).
    ReliableOrdered,
    /// Unordered, unreliable datagrams with max lifetime (UDP-like for 60Hz tick sync).
    UnreliableLossTolerant { max_packet_life_time_ms: u32 },
}

/// WebRTC Peer Connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

/// libp2p Gossipsub Topic subscription frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipsubTopic {
    pub name: String,
    pub max_transmit_size_bytes: usize,
}

/// Wire envelope for messages exchanged over the browser mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshEnvelope {
    pub envelope_id: String,
    pub sender_peer_id: String,
    pub target_peer_id: Option<String>,
    pub topic: String,
    pub timestamp_ms: u64,
    pub payload: MeshPayload,
}

/// Co-op Squad sync payload types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshPayload {
    /// Co-op movement synchronization.
    PlayerTransformSync {
        entity_id: u32,
        position_ecef: [f64; 3],
        velocity: [f32; 3],
        yaw_pitch: [f32; 2],
        motion_state: String,
    },
    /// Deterministic D&D style turn order sync.
    TurnCommitment {
        turn_id: u64,
        initiative_roll: u32,
        action_intent_hash: [u8; 32],
    },
    /// Synchronized dice roll with verifiable seed and cryptographic salt.
    VerifiableDiceRoll {
        roll_id: String,
        dice_notation: String, // e.g. "1d20+5", "2d6"
        sides: u32,
        count: u32,
        modifier: i32,
        result_sum: i32,
        individual_results: Vec<u32>,
        entropy_seed: [u8; 32],
    },
    /// Direct P2P tactic ping.
    SquadTacticPing {
        ping_type: String,
        target_ecef: [f64; 3],
        message: String,
    },
    /// Raw binary buffer.
    RawBinary(Vec<u8>),
}

/// Represents a single WebRTC DataChannel link between two browser peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRtcDataChannelPeer {
    pub peer_id: String,
    pub connection_state: PeerConnectionState,
    pub latency_rtt_ms: f32,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub reliability: ChannelReliability,
}

/// Manages active mesh peers, Gossipsub topic routing, and co-op squad states.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserMeshTransport {
    pub local_peer_id: String,
    pub active_squad_room_id: Option<String>,
    pub peers: HashMap<String, WebRtcDataChannelPeer>,
    pub subscribed_topics: HashSet<String>,
    pub outgoing_queue: Vec<MeshEnvelope>,
    pub incoming_queue: Vec<MeshEnvelope>,
    pub deterministic_turn_counter: u64,
}

impl BrowserMeshTransport {
    /// Creates a new browser mesh transport for a local peer.
    pub fn new(local_peer_id: String) -> Self {
        let mut transport = Self {
            local_peer_id,
            active_squad_room_id: None,
            peers: HashMap::new(),
            subscribed_topics: HashSet::new(),
            outgoing_queue: Vec::new(),
            incoming_queue: Vec::new(),
            deterministic_turn_counter: 0,
        };
        transport.subscribed_topics.insert("squad:coop:v1".to_string());
        transport.subscribed_topics.insert("squad:dice:v1".to_string());
        transport
    }

    /// Connects to a squad room.
    pub fn join_squad_room(&mut self, room_id: String) {
        self.active_squad_room_id = Some(room_id.clone());
        self.subscribed_topics.insert(format!("squad:{}:state", room_id));
        self.subscribed_topics.insert(format!("squad:{}:dice", room_id));
    }

    /// Registers a newly established peer connection (e.g. from WebRTC onopen).
    pub fn on_peer_connected(&mut self, peer_id: String, reliability: ChannelReliability) {
        self.peers.insert(
            peer_id.clone(),
            WebRtcDataChannelPeer {
                peer_id,
                connection_state: PeerConnectionState::Connected,
                latency_rtt_ms: 25.0,
                packets_sent: 0,
                packets_received: 0,
                reliability,
            },
        );
    }

    /// Marks a peer as disconnected.
    pub fn on_peer_disconnected(&mut self, peer_id: &str) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.connection_state = PeerConnectionState::Disconnected;
        }
    }

    /// Broadcasts player movement across all connected peers.
    pub fn broadcast_player_transform(
        &mut self,
        entity_id: u32,
        position_ecef: [f64; 3],
        velocity: [f32; 3],
        yaw_pitch: [f32; 2],
        motion_state: String,
        timestamp_ms: u64,
    ) {
        let envelope = MeshEnvelope {
            envelope_id: format!("env-{}", timestamp_ms),
            sender_peer_id: self.local_peer_id.clone(),
            target_peer_id: None,
            topic: "squad:coop:v1".to_string(),
            timestamp_ms,
            payload: MeshPayload::PlayerTransformSync {
                entity_id,
                position_ecef,
                velocity,
                yaw_pitch,
                motion_state,
            },
        };
        self.queue_outgoing(envelope);
    }

    /// Executes and broadcasts a verifiable synchronized dice roll for D&D / squad skill checks.
    pub fn broadcast_verifiable_dice_roll(
        &mut self,
        dice_notation: &str,
        count: u32,
        sides: u32,
        modifier: i32,
        entropy_seed: [u8; 32],
        timestamp_ms: u64,
    ) -> MeshEnvelope {
        use sha2::{Digest, Sha256};

        let mut individual_results = Vec::new();
        let mut sum = modifier;

        for i in 0..count {
            let mut hasher = Sha256::new();
            hasher.update(&entropy_seed);
            hasher.update(&i.to_le_bytes());
            hasher.update(&timestamp_ms.to_le_bytes());
            let hash = hasher.finalize();

            let raw_u32 = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
            let roll = (raw_u32 % sides) + 1;
            individual_results.push(roll);
            sum += roll as i32;
        }

        let envelope = MeshEnvelope {
            envelope_id: format!("roll-{}", timestamp_ms),
            sender_peer_id: self.local_peer_id.clone(),
            target_peer_id: None,
            topic: "squad:dice:v1".to_string(),
            timestamp_ms,
            payload: MeshPayload::VerifiableDiceRoll {
                roll_id: format!("roll-uuid-{}", timestamp_ms),
                dice_notation: dice_notation.to_string(),
                sides,
                count,
                modifier,
                result_sum: sum,
                individual_results,
                entropy_seed,
            },
        };

        self.queue_outgoing(envelope.clone());
        envelope
    }

    /// Queues an envelope for transmission.
    pub fn queue_outgoing(&mut self, env: MeshEnvelope) {
        for peer in self.peers.values_mut() {
            if peer.connection_state == PeerConnectionState::Connected {
                peer.packets_sent += 1;
            }
        }
        self.outgoing_queue.push(env);
    }

    /// Receives and buffers an incoming mesh envelope.
    pub fn receive_incoming(&mut self, env: MeshEnvelope) {
        if let Some(peer) = self.peers.get_mut(&env.sender_peer_id) {
            peer.packets_received += 1;
        }
        self.incoming_queue.push(env);
    }

    /// Drains incoming messages for the active gameplay frame.
    pub fn drain_incoming(&mut self) -> Vec<MeshEnvelope> {
        self.incoming_queue.drain(..).collect()
    }
}
