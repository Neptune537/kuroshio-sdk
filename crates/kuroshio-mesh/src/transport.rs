//! # WebRTC DataChannel Signaling & Reticulum Packet Framing
//!
//! Provides signaling payloads for establishing direct peer-to-peer WebRTC
//! DataChannels and packetizing offloaded graphics frames over low-overhead channels.

use serde::{Deserialize, Serialize};

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
    /// Payload type (e.g., 0x01 = HZB Occlusion, 0x02 = Physics State, 0x03 = Telemetry)
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

    /// Serializes packet to compact binary bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserializes packet from binary bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
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
}
