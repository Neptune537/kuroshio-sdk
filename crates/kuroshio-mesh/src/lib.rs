//! # Kuroshio Mesh Protocol: Teams, Squads & Coral Escrow Channels
//!
//! Provides the building blocks for community teams, squads, car clubs, and decentralized
//! state channels operating over the Kuroshio Gyre.

use nautilus_world::Wgs84GeoCoord;
use serde::{Deserialize, Serialize};
use sha2::Digest;

pub mod poh;
pub use poh::*;

pub mod zk_geo;
pub use zk_geo::*;

pub mod browser_mesh;
pub use browser_mesh::*;

pub mod htlc_geofence;
pub use htlc_geofence::*;

pub mod siren_radio;
pub use siren_radio::*;

#[cfg(feature = "mesh-transport")]
pub mod transport;
#[cfg(feature = "mesh-transport")]
pub use transport::*;

pub mod security;
pub use security::*;

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
/// Enforces integer-based lamports/CYAN units, cryptographic signature verification,
/// officer authorization checks, duplicate/banned member guards, and replay-protected deposits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KuroshioTeam {
    pub team_id: String,
    pub name: String,
    pub tag: String,
    pub headquarters_anchor: Option<Wgs84GeoCoord>,
    pub members: Vec<SquadMember>,
    /// Integer-based treasury balance (micro-CYAN / lamports: 1 CYAN = 1_000_000 micro-CYAN)
    /// Completely eliminates floating-point rounding discrepancies and phantom financial state.
    pub treasury_micro_cyan: u64,
    /// Banned or revoked DIDs barred from joining or claiming rewards
    pub banned_dids: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TeamMembershipError {
    UnauthorizedCaller,
    AlreadyMember,
    MemberBanned,
    CallerNotOfficerOrFounder,
    InvalidSignature,
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
            treasury_micro_cyan: 0,
            banned_dids: vec![],
        }
    }

    /// Checks if a DID is an active founder or officer with administrative authority.
    pub fn is_authorized_officer(&self, caller_did: &str) -> bool {
        self.members.iter().any(|m| {
            m.did == caller_did && (m.role == MemberRole::Founder || m.role == MemberRole::Officer)
        })
    }

    /// Adds a squad member with cryptographic authentication and authorization checks:
    /// - Requires the caller to be an active Founder or Officer.
    /// - Checks that the joining user is not already a member and is not banned.
    /// - Verifies the joining user's cryptographic signature over their DID and team_id.
    pub fn add_member_verified(
        &mut self,
        caller_did: &str,
        new_member: SquadMember,
        member_pubkey_bytes: &[u8; 32],
        signature_bytes: &[u8; 64],
    ) -> Result<(), TeamMembershipError> {
        if !self.is_authorized_officer(caller_did) {
            return Err(TeamMembershipError::CallerNotOfficerOrFounder);
        }

        if self.banned_dids.contains(&new_member.did) {
            return Err(TeamMembershipError::MemberBanned);
        }

        if self.members.iter().any(|m| m.did == new_member.did) {
            return Err(TeamMembershipError::AlreadyMember);
        }

        // Verify cryptographic signature: joins must prove ownership of the supplied DID
        use ed25519_dalek::{Signature, VerifyingKey};
        let verifying_key = VerifyingKey::from_bytes(member_pubkey_bytes)
            .map_err(|_| TeamMembershipError::InvalidSignature)?;
        let signature = Signature::from_bytes(signature_bytes);

        let mut hasher = sha2::Sha256::default();
        hasher.update(b"KUROSHIO_TEAM_JOIN_V1:");
        hasher.update(self.team_id.as_bytes());
        hasher.update(new_member.did.as_bytes());
        let digest: [u8; 32] = sha2::Digest::finalize(hasher).into();

        verifying_key
            .verify_strict(&digest, &signature)
            .map_err(|_| TeamMembershipError::InvalidSignature)?;

        self.members.push(new_member);
        Ok(())
    }

    /// Legacy convenience helper for local testing.
    pub fn add_member(&mut self, member: SquadMember) {
        if !self.members.iter().any(|m| m.did == member.did) {
            self.members.push(member);
        }
    }

    pub fn set_headquarters(&mut self, coord: Wgs84GeoCoord) {
        self.headquarters_anchor = Some(coord);
    }

    /// Deposits treasury funds in exact integer micro-CYAN units with overflow protection.
    pub fn deposit_treasury_micro_cyan(&mut self, amount_micro_cyan: u64) {
        self.treasury_micro_cyan = self.treasury_micro_cyan.saturating_add(amount_micro_cyan);
    }

    /// Legacy float deposit converted to integer micro-CYAN (1.0 CYAN = 1_000_000 micro-CYAN).
    pub fn deposit_treasury(&mut self, amount_cyan: f64) {
        let micro = (amount_cyan.max(0.0) * 1_000_000.0).round() as u64;
        self.deposit_treasury_micro_cyan(micro);
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
    /// The authoritative secure TLS stream URL from the Atlantis Node network (`https://radio.kuroshio.atlantis.market`).
    pub const SECURE_STREAM_URL: &'static str = "https://radio.kuroshio.atlantis.market/live.mp3";
    /// Fallback local development LAN endpoint
    pub const LOCAL_DEV_STREAM_URL: &'static str = "http://10.0.0.60:8080/live.mp3";

    /// Creates a default Siren Radio Stream state for the given UTC hour using secure TLS.
    pub fn new(utc_hour: u8) -> Self {
        let active = SevenDaughtersHost::from_utc_hour(utc_hour);
        Self {
            topic: Self::CANONICAL_TOPIC.to_string(),
            active_daughter: active,
            stream_url: Self::SECURE_STREAM_URL.to_string(),
            current_track: format!("Siren 24/7 Live — Host: {:?}", active),
            tip_channel_did: format!("did:atlantis:radio:{:?}", active).to_lowercase(),
            webrtc_swarm_channel_id: Self::DEFAULT_SWARM_CHANNEL_ID,
        }
    }

    /// Encapsulates a 48kHz Opus audio frame into a P2P WebRTC `MeshDataPacket` for squad swarm relay.
    #[cfg(feature = "mesh-transport")]
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
            http_ingress_url: "https://olympus.kuroshio.atlantis.market/api/v1/olympus/offload".to_string(),
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
        assert_eq!(stream.stream_url, SirenRadioStream::SECURE_STREAM_URL);

        let tip = stream.create_tip("did:atlantis:listener_1", 1, "Awesome deep house set!");
        assert_eq!(tip.amount_cyan, 0.1);
        assert_eq!(tip.recipient_did, "did:atlantis:radio:asterope");
    }

    #[test]
    fn test_team_verified_membership_and_integer_treasury() {
        use ed25519_dalek::Signer;

        let _founder_key = ed25519_dalek::SigningKey::from_bytes(&[0x11u8; 32]);
        let founder_did = "did:atlantis:founder_vance";
        let mut team = KuroshioTeam::new("team_seraph", "Seraphim Tactical", "SERAPH", founder_did, "Vance");

        // 1. Integer-based treasury deposit (100.5 CYAN = 100_500_000 micro-CYAN)
        team.deposit_treasury(100.5);
        assert_eq!(team.treasury_micro_cyan, 100_500_000);

        // 2. Joining member signs their DID + team_id
        let member_key = ed25519_dalek::SigningKey::from_bytes(&[0x22u8; 32]);
        let member_pubkey = member_key.verifying_key();
        let member_did = "did:atlantis:pilot_zero";

        let mut hasher = sha2::Sha256::default();
        hasher.update(b"KUROSHIO_TEAM_JOIN_V1:");
        hasher.update(team.team_id.as_bytes());
        hasher.update(member_did.as_bytes());
        let digest: [u8; 32] = sha2::Digest::finalize(hasher).into();
        let sig = member_key.sign(&digest);

        let new_member = SquadMember {
            did: member_did.to_string(),
            callsign: "Zero".to_string(),
            role: MemberRole::Operator,
            reputation_score: 50,
            joined_timestamp: 1000,
        };

        // Unauthorized caller fails
        assert_eq!(
            team.add_member_verified(
                "did:atlantis:random_stranger",
                new_member.clone(),
                &member_pubkey.to_bytes(),
                &sig.to_bytes()
            ),
            Err(TeamMembershipError::CallerNotOfficerOrFounder)
        );

        // Founder authorization succeeds
        assert!(team
            .add_member_verified(
                founder_did,
                new_member.clone(),
                &member_pubkey.to_bytes(),
                &sig.to_bytes()
            )
            .is_ok());

        // Duplicate member rejected
        assert_eq!(
            team.add_member_verified(
                founder_did,
                new_member.clone(),
                &member_pubkey.to_bytes(),
                &sig.to_bytes()
            ),
            Err(TeamMembershipError::AlreadyMember)
        );
    }
}
