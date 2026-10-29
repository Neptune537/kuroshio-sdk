//! # Kuroshio Mock Home Node: Local Developer Test Runner
//!
//! A lightweight, zero-cloud in-memory simulator of an Atlantis Club Home Node.
//! Allows external developers to test SIWS authentication, Coral Escrow state locks,
//! Bioluminescence PoH telemetry, and sandboxed app commands offline without
//! needing a physical Raspberry Pi or live Solana cluster.

use kuroshio_mesh::ZkGeoProximityProof;
use lumeria_app_sdk::{AppManifest, AppSecurityTier, LumeriaHostCommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum MockNodeError {
    #[error("Session nonce has expired or does not exist: `{0}`")]
    InvalidOrExpiredNonce(String),
    #[error("Invalid wallet address or signature format")]
    InvalidSignature,
    #[error("Escrow `{0}` already exists")]
    EscrowAlreadyExists(String),
    #[error("Escrow `{0}` not found")]
    EscrowNotFound(String),
    #[error("Escrow `{0}` is not in Locked status")]
    EscrowNotLocked(String),
    #[error("Insufficient funds in treasury: requested {requested}, available {available}")]
    InsufficientFunds { requested: f64, available: f64 },
    #[error("App capability `{0}` rejected for security tier `{1:?}`")]
    CapabilityRejected(String, AppSecurityTier),
}

/// Sign-In With Solana (SIWS) authentication challenge payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiwsChallenge {
    pub domain: String,
    pub address: String,
    pub statement: String,
    pub uri: String,
    pub version: String,
    pub chain_id: String,
    pub nonce: String,
    pub issued_at_epoch_sec: u64,
    pub expires_at_epoch_sec: u64,
}

/// Active verified developer/player session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockSession {
    pub did: String,
    pub solana_address: String,
    pub staked_cyan: f64,
    pub is_club_member: bool,
    pub created_at: u64,
}

/// Status of an on-chain/off-chain Coral Escrow record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EscrowStatus {
    Locked,
    Released,
    Refunded,
}

/// An in-memory Coral Escrow contract tracking task deposits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockEscrow {
    pub task_id: String,
    pub creator_did: String,
    pub amount_cyan: f64,
    pub status: EscrowStatus,
    pub recipient_did: Option<String>,
}

/// Payout settlement receipt emitted upon task verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockEscrowSettlement {
    pub task_id: String,
    pub winner_did: String,
    pub amount_cyan: f64,
    pub club_dividend_cyan: f64,
    pub settled_at: u64,
}

/// Recorded Proof-of-Health node telemetry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockTelemetry {
    pub node_did: String,
    pub cpu_percent: f32,
    pub vram_mb: u64,
    pub fps: f32,
    pub is_healthy: bool,
}

/// In-Memory Mock Home Node Test Runner.
#[derive(Debug, Default)]
pub struct MockHomeNode {
    pub club_id: String,
    pub node_did: String,
    pub nonces: HashMap<String, u64>,
    pub sessions: HashMap<String, MockSession>,
    pub escrows: HashMap<String, MockEscrow>,
    pub telemetry: HashMap<String, MockTelemetry>,
    pub treasury_cyan: f64,
}

impl MockHomeNode {
    /// Creates a new Mock Home Node for a given club namespace.
    pub fn new(club_id: &str) -> Self {
        Self {
            club_id: club_id.to_string(),
            node_did: format!("did:atlantis:club:{}", club_id),
            nonces: HashMap::new(),
            sessions: HashMap::new(),
            escrows: HashMap::new(),
            telemetry: HashMap::new(),
            treasury_cyan: 10_000.0, // Seed mock treasury with 10k CYAN
        }
    }

    /// Emits a cryptographically formed SIWS challenge for a client wallet.
    pub fn issue_siws_challenge(&mut self, wallet_address: &str, current_epoch_sec: u64) -> SiwsChallenge {
        let nonce = format!("{:x}", (current_epoch_sec.wrapping_mul(6364136223846793005) ^ 0xDEADBEEF));
        let expires_at = current_epoch_sec + 300; // 5-minute expiry
        self.nonces.insert(nonce.clone(), expires_at);

        SiwsChallenge {
            domain: format!("{}.atlantis.mesh", self.club_id),
            address: wallet_address.to_string(),
            statement: format!("Authorize {} Session via Mock Home Node", self.club_id),
            uri: format!("https://{}.atlantis.mesh/auth", self.club_id),
            version: "1".into(),
            chain_id: "solana:devnet".into(),
            nonce,
            issued_at_epoch_sec: current_epoch_sec,
            expires_at_epoch_sec: expires_at,
        }
    }

    /// Verifies the challenge and provisions a test session with a standard DID.
    pub fn verify_siws(
        &mut self,
        challenge: &SiwsChallenge,
        signature: &str,
        current_epoch_sec: u64,
    ) -> Result<MockSession, MockNodeError> {
        let expires_at = self
            .nonces
            .get(&challenge.nonce)
            .copied()
            .ok_or_else(|| MockNodeError::InvalidOrExpiredNonce(challenge.nonce.clone()))?;

        if current_epoch_sec > expires_at {
            self.nonces.remove(&challenge.nonce);
            return Err(MockNodeError::InvalidOrExpiredNonce(challenge.nonce.clone()));
        }

        if signature.is_empty() {
            return Err(MockNodeError::InvalidSignature);
        }

        // Clean up nonce (single use)
        self.nonces.remove(&challenge.nonce);

        let did = format!("did:atlantis:solana:{}", challenge.address);
        let session = MockSession {
            did: did.clone(),
            solana_address: challenge.address.clone(),
            staked_cyan: 50.0, // Mock $10+ qualifying stake
            is_club_member: true,
            created_at: current_epoch_sec,
        };

        self.sessions.insert(did, session.clone());
        Ok(session)
    }

    /// Locks funds into Coral Escrow for a task, bounty, or 1v1 match.
    pub fn lock_coral_escrow(
        &mut self,
        task_id: &str,
        creator_did: &str,
        amount_cyan: f64,
    ) -> Result<MockEscrow, MockNodeError> {
        if self.escrows.contains_key(task_id) {
            return Err(MockNodeError::EscrowAlreadyExists(task_id.to_string()));
        }

        let escrow = MockEscrow {
            task_id: task_id.to_string(),
            creator_did: creator_did.to_string(),
            amount_cyan,
            status: EscrowStatus::Locked,
            recipient_did: None,
        };

        self.escrows.insert(task_id.to_string(), escrow.clone());
        Ok(escrow)
    }

    /// Releases escrowed funds to a verified worker/winner with a fractional club dividend.
    pub fn release_coral_escrow(
        &mut self,
        task_id: &str,
        winner_did: &str,
        current_epoch_sec: u64,
    ) -> Result<MockEscrowSettlement, MockNodeError> {
        let escrow = self
            .escrows
            .get_mut(task_id)
            .ok_or_else(|| MockNodeError::EscrowNotFound(task_id.to_string()))?;

        if escrow.status != EscrowStatus::Locked {
            return Err(MockNodeError::EscrowNotLocked(task_id.to_string()));
        }

        escrow.status = EscrowStatus::Released;
        escrow.recipient_did = Some(winner_did.to_string());

        // 2% Club dividend
        let club_dividend = escrow.amount_cyan * 0.02;
        let winner_payout = escrow.amount_cyan - club_dividend;
        self.treasury_cyan += club_dividend;

        Ok(MockEscrowSettlement {
            task_id: task_id.to_string(),
            winner_did: winner_did.to_string(),
            amount_cyan: winner_payout,
            club_dividend_cyan: club_dividend,
            settled_at: current_epoch_sec,
        })
    }

    /// Records and validates Bioluminescence Proof-of-Health (PoH) metrics from a worker node.
    pub fn record_poh_telemetry(
        &mut self,
        node_did: &str,
        cpu_percent: f32,
        vram_mb: u64,
        fps: f32,
    ) -> Result<MockTelemetry, MockNodeError> {
        let is_healthy = fps >= 15.0 && cpu_percent < 95.0;
        let telemetry = MockTelemetry {
            node_did: node_did.to_string(),
            cpu_percent,
            vram_mb,
            fps,
            is_healthy,
        };
        self.telemetry.insert(node_did.to_string(), telemetry.clone());
        Ok(telemetry)
    }

    /// Generates a Zero-Knowledge geo-proximity proof that masks raw GPS.
    pub fn issue_zk_geo_proof(&self, venue_id: &str, coarse_geohash: &str, timestamp: u64) -> ZkGeoProximityProof {
        ZkGeoProximityProof {
            sector_geohash: coarse_geohash.to_string(),
            venue_id: Some(venue_id.to_string()),
            timestamp,
            club_signature: format!("mock_sig_club_{}_{}", self.club_id, timestamp),
        }
    }

    /// Executes host commands issued by a sandboxed Lumeria guest application.
    pub fn execute_app_command(
        &mut self,
        manifest: &AppManifest,
        command: LumeriaHostCommand,
    ) -> Result<String, MockNodeError> {
        match command {
            LumeriaHostCommand::ShowNotification { title, message } => {
                Ok(format!("Notification rendered: [{}] {}", title, message))
            }
            LumeriaHostCommand::PlaySpatialAudio { sound_id, volume } => {
                Ok(format!("Spatial audio played: {} at volume {}", sound_id, volume))
            }
            LumeriaHostCommand::RequestEscrowCheckout { item_id, price_cyan } => {
                manifest
                    .validate_permissions()
                    .map_err(|e| MockNodeError::CapabilityRejected(e, manifest.security_tier))?;
                let task_id = format!("checkout_{}_{}", manifest.app_id, item_id);
                self.lock_coral_escrow(&task_id, &manifest.publisher_did, price_cyan as f64)?;
                Ok(format!("Checkout escrow created: {}", task_id))
            }
            LumeriaHostCommand::SendMeshMessage { recipient_did, payload } => {
                Ok(format!("Mesh message routed to {}: {} bytes", recipient_did, payload.len()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumeria_app_sdk::AppCapability;

    #[test]
    fn test_siws_lifecycle() {
        let mut node = MockHomeNode::new("seraphim_club");
        let challenge = node.issue_siws_challenge("7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU", 1000);

        assert_eq!(challenge.chain_id, "solana:devnet");
        assert_eq!(challenge.domain, "seraphim_club.atlantis.mesh");

        // Verify with valid mock signature
        let session = node.verify_siws(&challenge, "valid_mock_sig", 1050).unwrap();
        assert_eq!(session.did, "did:atlantis:solana:7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU");
        assert!(session.staked_cyan >= 10.0);

        // Second verification with same challenge must fail (nonce burned)
        let retry = node.verify_siws(&challenge, "valid_mock_sig", 1055);
        assert!(retry.is_err());
    }

    #[test]
    fn test_coral_escrow_lifecycle() {
        let mut node = MockHomeNode::new("okinawa_anchor");
        let task_id = "task_dem_elevation_001";
        let creator_did = "did:atlantis:solana:alice";
        let worker_did = "did:atlantis:solana:bob";

        // 1. Lock funds in escrow
        let escrow = node.lock_coral_escrow(task_id, creator_did, 100.0).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Locked);

        // 2. Release to worker
        let settlement = node.release_coral_escrow(task_id, worker_did, 2000).unwrap();
        assert_eq!(settlement.winner_did, worker_did);
        assert_eq!(settlement.amount_cyan, 98.0); // 100 - 2% fee
        assert_eq!(settlement.club_dividend_cyan, 2.0); // 2% fee to club
        assert_eq!(node.treasury_cyan, 10_002.0);
    }

    #[test]
    fn test_zk_geo_proximity_masks_raw_coords() {
        let node = MockHomeNode::new("club_drift");
        let proof = node.issue_zk_geo_proof("venue_daikoku_pa", "xn77h", 5000);

        assert_eq!(proof.sector_geohash, "xn77h");
        assert_eq!(proof.venue_id.unwrap(), "venue_daikoku_pa");
        assert!(proof.club_signature.contains("club_drift"));
    }

    #[test]
    fn test_app_sandbox_capability_enforcement() {
        let mut node = MockHomeNode::new("test_club");
        let mut manifest = AppManifest::new("mini_store", "Mini Storefront", "1.0.0", "did:atlantis:dev1")
            .with_capability(AppCapability::StoreInitiateCheckout);

        // Sandboxed guest cannot checkout
        manifest.security_tier = AppSecurityTier::SandboxedGuest;
        let cmd = LumeriaHostCommand::RequestEscrowCheckout {
            item_id: "chassis_part_v1".into(),
            price_cyan: 25.0,
        };
        let err = node.execute_app_command(&manifest, cmd.clone());
        assert!(err.is_err());

        // Upgrading to ClubVerified allows checkout
        manifest.security_tier = AppSecurityTier::ClubVerified;
        let success = node.execute_app_command(&manifest, cmd);
        assert!(success.is_ok());
    }
}
