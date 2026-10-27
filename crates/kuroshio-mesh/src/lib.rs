//! # Kuroshio Mesh Protocol: Teams, Squads & Coral Escrow Channels
//!
//! Provides the building blocks for community teams, squads, car clubs, and decentralized
//! state channels operating over the Kuroshio Gyre.

use nautilus_world::Wgs84GeoCoord;
use serde::{Deserialize, Serialize};

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
}

/// Zero-Knowledge location disclosure token (prevents raw GPS leaks).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkGeoProximityProof {
    pub sector_geohash: String,
    pub venue_id: Option<String>,
    pub timestamp: u64,
    pub club_signature: String,
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
