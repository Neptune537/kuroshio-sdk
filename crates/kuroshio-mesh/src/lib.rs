//! # Kuroshio Mesh Protocol: Teams, Squads & Coral Escrow Channels
//!
//! Provides the building blocks for community teams, squads, car clubs, and decentralized
//! state channels operating over the Kuroshio Gyre.

use nautilus_world::Wgs84GeoCoord;
use serde::{Deserialize, Serialize};

pub mod poh;
pub use poh::*;

pub mod zk_geo;
pub use zk_geo::*;

pub mod transport;
pub use transport::*;

/// Role within a Kuroshio squad or club.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemberRole {
    Founder,
    Officer,
    Operator,
    Recruit,
}

/// A member of a Kuroshio team or squad.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadMember {
    pub did: String,
    pub callsign: String,
    pub role: MemberRole,
    pub reputation_score: u32,
    pub joined_timestamp: u64,
}

/// A team, faction, or community club registered on the Kuroshio mesh.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KuroshioTeam {
    pub team_id: String,
    pub name: String,
    pub tag: String,
    pub headquarters_anchor: Option<Wgs84GeoCoord>,
    pub members: Vec<SquadMember>,
    pub treasury_cyan_balance: f64,
}

impl KuroshioTeam {
    pub fn new(team_id: &str, name: &str, tag: &str, founder_did: &str, founder_callsign: &str) -> Self {
        let founder = SquadMember {
            did: founder_did.to_string(),
            callsign: founder_callsign.to_string(),
            role: MemberRole::Founder,
            reputation_score: 100,
            joined_timestamp: 0,
        };

        Self {
            team_id: team_id.to_string(),
            name: name.to_string(),
            tag: tag.to_string(),
            headquarters_anchor: None,
            members: vec![founder],
            treasury_cyan_balance: 0.0,
        }
    }

    pub fn add_member(&mut self, member: SquadMember) {
        self.members.push(member);
    }

    pub fn set_headquarters(&mut self, coord: Wgs84GeoCoord) {
        self.headquarters_anchor = Some(coord);
    }

    pub fn deposit_treasury(&mut self, amount_cyan: f64) {
        self.treasury_cyan_balance += amount_cyan;
    }
}

/// Coral State Channel micro-transaction (0.1 CYAN tipping / peer payments).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoralTipPayload {
    pub sender_did: String,
    pub recipient_did: String,
    pub amount_cyan: f32,
    pub note: String,
    pub channel_sequence: u64,
}

/// The Seven Daughters 24/7 Siren Frequency Theater Radio Station broadcasters.
///
/// Anchored on the canonical 432 Hz base frequency across seven dedicated harmonic time-slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SevenDaughtersHost {
    /// Maia (432 Hz • Dawn Oceanic Ambient / Horizon Chill • 04:00-08:00)
    Maia,
    /// Alcyone (528 Hz • High-Speed Cyberpunk D&B / Breakbeats • 08:00-12:00)
    Alcyone,
    /// Asterope (639 Hz • Tactical Deep House & Frontline Reports • 12:00-16:00)
    Asterope,
    /// Celaeno (741 Hz • Glitch / Industrial Bass / Storm Chaser • 16:00-20:00)
    Celaeno,
    /// Electra (852 Hz • Vocal Trance / Eurobeat / Club Atlantis • 20:00-00:00)
    Electra,
    /// Taygete (963 Hz • Darkwave / Minimal Techno / Abyssal Drone • 00:00-02:00)
    Taygete,
    /// Merope (396 Hz • Lo-Fi / Starlight Harbor Acoustic • 02:00-04:00)
    Merope,
}

impl SevenDaughtersHost {
    /// Returns the canonical carrier / harmonic frequency in Hz.
    pub fn frequency_hz(&self) -> u32 {
        match self {
            Self::Merope => 396,
            Self::Maia => 432,
            Self::Alcyone => 528,
            Self::Asterope => 639,
            Self::Celaeno => 741,
            Self::Electra => 852,
            Self::Taygete => 963,
        }
    }

    /// Returns the musical genre and broadcast theme description.
    pub fn theme(&self) -> &'static str {
        match self {
            Self::Maia => "Dawn Oceanic Ambient / Horizon Chill",
            Self::Alcyone => "High-Speed Cyberpunk D&B / Breakbeats",
            Self::Asterope => "Tactical Deep House & Frontline Reports",
            Self::Celaeno => "Glitch / Industrial Bass / Storm Chaser",
            Self::Electra => "Vocal Trance / Eurobeat / Club Atlantis",
            Self::Taygete => "Darkwave / Minimal Techno / Abyssal Drone",
            Self::Merope => "Lo-Fi / Starlight Harbor Acoustic",
        }
    }

    /// Returns the scheduled broadcast hours range in UTC (start_hour, end_hour).
    pub fn air_time_utc(&self) -> (u8, u8) {
        match self {
            Self::Merope => (2, 4),
            Self::Maia => (4, 8),
            Self::Alcyone => (8, 12),
            Self::Asterope => (12, 16),
            Self::Celaeno => (16, 20),
            Self::Electra => (20, 24),
            Self::Taygete => (0, 2),
        }
    }

    /// Resolves which daughter is canonically on air given the UTC hour (0..24).
    pub fn from_utc_hour(hour: u8) -> Self {
        match hour % 24 {
            0..=1 => Self::Taygete,
            2..=3 => Self::Merope,
            4..=7 => Self::Maia,
            8..=11 => Self::Alcyone,
            12..=15 => Self::Asterope,
            16..=19 => Self::Celaeno,
            _ => Self::Electra,
        }
    }
}

/// Global 24/7 Siren Radio broadcast stream state and metadata.
///
/// Supports both P2P WebRTC DataChannel Opus Swarm Audio (`OpusAudioSwarmFrame` on channel 432)
/// and synchronized HTTP stream fallback from the Club Home Node (`10.0.0.60:8080`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SirenRadioStream {
    /// Canonical topic (e.g. `siren:frequency:global_stage`).
    pub topic: String,
    /// Currently active daughter hosting on air.
    pub active_daughter: SevenDaughtersHost,
    /// Authoritative stream URL originating from the Raspberry Pi node.
    pub stream_url: String,
    /// Current track title or live commentary segment.
    pub current_track: String,
    /// Coral tip address for sending 0.1 CYAN tips to the active daughter on air.
    pub tip_channel_did: String,
    /// Dedicated WebRTC DataChannel ID for P2P Opus Swarm Audio relay.
    pub webrtc_swarm_channel_id: u64,
}

impl SirenRadioStream {
    pub const CANONICAL_TOPIC: &'static str = "siren:frequency:global_stage";
    pub const DEFAULT_SWARM_CHANNEL_ID: u64 = 432;
    /// The authoritative stream URL from the Raspberry Pi Club Home Node (`zero@10.0.0.60`).
    pub const PI_STREAM_URL: &'static str = "http://10.0.0.60:8080/live.mp3";

    /// Creates a default Siren Radio Stream state for the given UTC hour.
    pub fn new(utc_hour: u8) -> Self {
        let active = SevenDaughtersHost::from_utc_hour(utc_hour);
        Self {
            topic: Self::CANONICAL_TOPIC.to_string(),
            active_daughter: active,
            stream_url: Self::PI_STREAM_URL.to_string(),
            current_track: format!("Siren 24/7 Live — Host: {:?}", active),
            tip_channel_did: format!("did:atlantis:radio:{:?}", active).to_lowercase(),
            webrtc_swarm_channel_id: Self::DEFAULT_SWARM_CHANNEL_ID,
        }
    }

    /// Encapsulates a 48kHz Opus audio frame into a P2P WebRTC `MeshDataPacket` for squad swarm relay.
    pub fn build_swarm_audio_packet(
        &self,
        sequence: u64,
        ttl_hops: u8,
        opus_frame_bytes: Vec<u8>,
    ) -> Result<MeshDataPacket, serde_json::Error> {
        let frame = OpusAudioSwarmFrame {
            stream_topic: self.topic.clone(),
            broadcaster_id: format!("{:?}", self.active_daughter),
            sequence,
            sample_rate_hz: 48000,
            channels: 2,
            ttl_hops,
            opus_frame_bytes,
        };
        MeshDataPacket::from_opus_swarm_frame(self.webrtc_swarm_channel_id, &frame)
    }

    /// Creates a 0.1 CYAN Coral tip payload for the active host on air.
    pub fn create_tip(&self, sender_did: &str, sequence: u64, message: &str) -> CoralTipPayload {
        CoralTipPayload {
            sender_did: sender_did.to_string(),
            recipient_did: self.tip_channel_did.clone(),
            amount_cyan: 0.1,
            note: message.to_string(),
            channel_sequence: sequence,
        }
    }
}

/// Seed node endpoint for offloading heavy compute and consensus tasks to Olympus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OlympusSeedEndpoint {
    pub node_id: String,
    pub http_ingress_url: String,
    pub status: String,
    pub max_batch_size: usize,
}

impl Default for OlympusSeedEndpoint {
    fn default() -> Self {
        Self {
            node_id: "olympus-seed-subpolar".to_string(),
            http_ingress_url: "http://10.0.0.60:8080/api/v1/olympus/offload".to_string(),
            status: "ONLINE • SUBPOLAR_APEX".to_string(),
            max_batch_size: 64,
        }
    }
}

impl OlympusSeedEndpoint {
    pub fn new(url: &str) -> Self {
        Self {
            node_id: "olympus-seed-subpolar".to_string(),
            http_ingress_url: url.to_string(),
            status: "ONLINE • SUBPOLAR_APEX".to_string(),
            max_batch_size: 64,
        }
    }

    /// Prepares an offload payload packet destined for the Olympus compute revolver.
    pub fn create_offload_envelope(&self, sender_did: &str, task_type: &str, payload_bytes: Vec<u8>) -> serde_json::Value {
        serde_json::json!({
            "target_seed": self.node_id,
            "sender_did": sender_did,
            "task_type": task_type,
            "payload_len": payload_bytes.len(),
            "offload_timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seven_daughters_schedule_and_frequency() {
        assert_eq!(SevenDaughtersHost::from_utc_hour(1), SevenDaughtersHost::Taygete);
        assert_eq!(SevenDaughtersHost::Taygete.frequency_hz(), 963);

        assert_eq!(SevenDaughtersHost::from_utc_hour(3), SevenDaughtersHost::Merope);
        assert_eq!(SevenDaughtersHost::Merope.frequency_hz(), 396);

        assert_eq!(SevenDaughtersHost::from_utc_hour(6), SevenDaughtersHost::Maia);
        assert_eq!(SevenDaughtersHost::Maia.frequency_hz(), 432);

        assert_eq!(SevenDaughtersHost::from_utc_hour(10), SevenDaughtersHost::Alcyone);
        assert_eq!(SevenDaughtersHost::Alcyone.frequency_hz(), 528);

        assert_eq!(SevenDaughtersHost::from_utc_hour(14), SevenDaughtersHost::Asterope);
        assert_eq!(SevenDaughtersHost::Asterope.frequency_hz(), 639);

        assert_eq!(SevenDaughtersHost::from_utc_hour(18), SevenDaughtersHost::Celaeno);
        assert_eq!(SevenDaughtersHost::Celaeno.frequency_hz(), 741);

        assert_eq!(SevenDaughtersHost::from_utc_hour(22), SevenDaughtersHost::Electra);
        assert_eq!(SevenDaughtersHost::Electra.frequency_hz(), 852);
    }

    #[test]
    fn test_siren_radio_stream_tipping() {
        let stream = SirenRadioStream::new(14); // Asterope
        assert_eq!(stream.active_daughter, SevenDaughtersHost::Asterope);
        assert_eq!(stream.stream_url, SirenRadioStream::PI_STREAM_URL);

        let tip = stream.create_tip("did:atlantis:listener_1", 1, "Awesome deep house set!");
        assert_eq!(tip.amount_cyan, 0.1);
        assert_eq!(tip.recipient_did, "did:atlantis:radio:asterope");
    }
}
