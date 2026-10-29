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

    /// Deserializes packet from binary bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
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
}
