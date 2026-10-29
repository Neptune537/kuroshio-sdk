//! # Kuroshio Mock Home Node: State Channel Dispute Timers & Solana SPL/PDA Settlement
//!
//! A zero-cloud in-memory simulator of an Atlantis Club Home Node:
//! - **SIWS Authentication:** Sign-In With Solana challenge/nonce lifecycle.
//! - **Coral Escrow & State Channel Dispute Timers (`SignedChannelStateUpdate`):**
//!   Enforces cryptographic challenge windows and monotonic sequence-signed state arbitration.
//! - **Solana Program Library (SPL) / Anchor PDA Settlement (`SolanaPdaSettlementInstruction`):**
//!   Deterministically derives Escrow & Club Treasury PDAs and generates Anchor settlement payloads.

use kuroshio_mesh::ZkGeoProximityProof;
use lumeria_app_sdk::{AppManifest, AppSecurityTier, LumeriaHostCommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;

pub const CORAL_ESCROW_PROGRAM_ID: &str = "CoraL11111111111111111111111111111111111111";
pub const DEFAULT_DISPUTE_WINDOW_SECS: u64 = 3600; // 1-hour cryptographic challenge window

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
    #[error("Escrow `{0}` is not in Locked or Disputed status")]
    EscrowNotLocked(String),
    #[error("Dispute window still active until epoch {expires_at} (current: {current})")]
    DisputeWindowActive { current: u64, expires_at: u64 },
    #[error("State update sequence {submitted} is not strictly greater than current {current}")]
    StaleSequenceNumber { submitted: u64, current: u64 },
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
    InDisputeChallengeWindow,
    Released,
    Refunded,
}

/// Sequence-signed off-chain state channel update for dispute arbitration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedChannelStateUpdate {
    pub task_id: String,
    pub sequence_number: u64,
    pub worker_payout_cyan: f64,
    pub creator_refund_cyan: f64,
    pub winner_did: String,
    pub state_signature: String,
}

/// Anchor-compatible Solana Program Library (SPL) PDA settlement instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolanaPdaSettlementInstruction {
    pub program_id: String,
    pub escrow_vault_pda: String,
    pub club_treasury_pda: String,
    pub winner_token_account: String,
    pub winner_payout_lamports: u64,
    pub club_dividend_lamports: u64,
    pub final_sequence: u64,
    pub anchor_discriminator: [u8; 8],
}

/// An in-memory Coral Escrow contract tracking task deposits and dispute timers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockEscrow {
    pub task_id: String,
    pub creator_did: String,
    pub amount_cyan: f64,
    pub status: EscrowStatus,
    pub recipient_did: Option<String>,
    pub highest_sequence: u64,
    pub dispute_expires_at_epoch: Option<u64>,
}

/// Payout settlement receipt emitted upon task verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockEscrowSettlement {
    pub task_id: String,
    pub winner_did: String,
    pub amount_cyan: f64,
    pub club_dividend_cyan: f64,
    pub settled_at: u64,
    pub solana_pda_instruction: SolanaPdaSettlementInstruction,
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
            treasury_cyan: 10_000.0,
        }
    }

    /// Deterministically derives a Solana Program Derived Address (PDA) representation for `[seed_prefix, id]`.
    pub fn derive_solana_pda(seed_prefix: &str, identifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(CORAL_ESCROW_PROGRAM_ID.as_bytes());
        hasher.update(seed_prefix.as_bytes());
        hasher.update(identifier.as_bytes());
        let digest = hasher.finalize();
        format!("pda_{}", digest[..16].iter().map(|b| format!("{:02x}", b)).collect::<String>())
    }

    /// Emits a cryptographically formed SIWS challenge for a client wallet.
    pub fn issue_siws_challenge(&mut self, wallet_address: &str, current_epoch_sec: u64) -> SiwsChallenge {
        let nonce = format!("{:x}", (current_epoch_sec.wrapping_mul(6364136223846793005) ^ 0xDEADBEEF));
        let expires_at = current_epoch_sec + 300;
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

        self.nonces.remove(&challenge.nonce);

        let did = format!("did:atlantis:solana:{}", challenge.address);
        let session = MockSession {
            did: did.clone(),
            solana_address: challenge.address.clone(),
            staked_cyan: 50.0,
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
            highest_sequence: 0,
            dispute_expires_at_epoch: None,
        };

        self.escrows.insert(task_id.to_string(), escrow.clone());
        Ok(escrow)
    }

    /// Initiates or updates a cryptographic state channel dispute challenge window using a sequence-signed update.
    pub fn submit_state_channel_update(
        &mut self,
        update: &SignedChannelStateUpdate,
        current_epoch_sec: u64,
        dispute_window_secs: u64,
    ) -> Result<&MockEscrow, MockNodeError> {
        if update.state_signature.is_empty() {
            return Err(MockNodeError::InvalidSignature);
        }

        let escrow = self
            .escrows
            .get_mut(&update.task_id)
            .ok_or_else(|| MockNodeError::EscrowNotFound(update.task_id.clone()))?;

        if update.sequence_number <= escrow.highest_sequence {
            return Err(MockNodeError::StaleSequenceNumber {
                submitted: update.sequence_number,
                current: escrow.highest_sequence,
            });
        }

        escrow.highest_sequence = update.sequence_number;
        escrow.recipient_did = Some(update.winner_did.clone());
        escrow.status = EscrowStatus::InDisputeChallengeWindow;
        escrow.dispute_expires_at_epoch = Some(current_epoch_sec + dispute_window_secs);

        Ok(escrow)
    }

    /// Releases escrowed funds to a verified worker/winner with a 2% fractional club dividend
    /// and generates the Solana Anchor PDA settlement instruction.
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

        if escrow.status != EscrowStatus::Locked && escrow.status != EscrowStatus::InDisputeChallengeWindow {
            return Err(MockNodeError::EscrowNotLocked(task_id.to_string()));
        }

        if let Some(expires_at) = escrow.dispute_expires_at_epoch {
            if current_epoch_sec < expires_at {
                return Err(MockNodeError::DisputeWindowActive {
                    current: current_epoch_sec,
                    expires_at,
                });
            }
        }

        escrow.status = EscrowStatus::Released;
        escrow.recipient_did = Some(winner_did.to_string());

        let club_dividend = escrow.amount_cyan * 0.02;
        let winner_payout = escrow.amount_cyan - club_dividend;
        let final_seq = escrow.highest_sequence;
        self.treasury_cyan += club_dividend;

        let escrow_vault_pda = Self::derive_solana_pda("coral_escrow", task_id);
        let club_treasury_pda = Self::derive_solana_pda("club_treasury", &self.club_id);

        let solana_pda_instruction = SolanaPdaSettlementInstruction {
            program_id: CORAL_ESCROW_PROGRAM_ID.to_string(),
            escrow_vault_pda,
            club_treasury_pda,
            winner_token_account: winner_did.to_string(),
            winner_payout_lamports: (winner_payout * 1_000_000.0) as u64,
            club_dividend_lamports: (club_dividend * 1_000_000.0) as u64,
            final_sequence: final_seq,
            anchor_discriminator: [0x9a, 0x2b, 0x4c, 0x11, 0x8e, 0x05, 0x77, 0x3f],
        };

        Ok(MockEscrowSettlement {
            task_id: task_id.to_string(),
            winner_did: winner_did.to_string(),
            amount_cyan: winner_payout,
            club_dividend_cyan: club_dividend,
            settled_at: current_epoch_sec,
            solana_pda_instruction,
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
            commitment_hash: format!("comm_{}_{}", venue_id, timestamp),
            timestamp,
            club_signature: format!("mock_sig_club_{}_{}", self.club_id, timestamp),
            zk_circuit_proof: None,
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

        let session = node.verify_siws(&challenge, "valid_mock_sig", 1050).unwrap();
        assert_eq!(session.did, "did:atlantis:solana:7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU");
        assert!(session.staked_cyan >= 10.0);

        let retry = node.verify_siws(&challenge, "valid_mock_sig", 1055);
        assert!(retry.is_err());
    }

    #[test]
    fn test_coral_escrow_dispute_timer_and_solana_pda_settlement() {
        let mut node = MockHomeNode::new("okinawa_anchor");
        let task_id = "task_dem_elevation_001";
        let creator_did = "did:atlantis:solana:alice";
        let worker_did = "did:atlantis:solana:bob";

        let escrow = node.lock_coral_escrow(task_id, creator_did, 100.0).unwrap();
        assert_eq!(escrow.status, EscrowStatus::Locked);

        // Submit sequence-signed state update with a 600s challenge window
        let update_v1 = SignedChannelStateUpdate {
            task_id: task_id.into(),
            sequence_number: 1,
            worker_payout_cyan: 98.0,
            creator_refund_cyan: 0.0,
            winner_did: worker_did.into(),
            state_signature: "sig_seq_1".into(),
        };
        node.submit_state_channel_update(&update_v1, 1000, 600).unwrap();

        // Attempting to release before epoch 1600 must fail due to active dispute timer
        let early_release = node.release_coral_escrow(task_id, worker_did, 1200);
        assert!(matches!(early_release, Err(MockNodeError::DisputeWindowActive { .. })));

        // Higher sequence number supersedes previous state
        let update_v2 = SignedChannelStateUpdate {
            sequence_number: 2,
            state_signature: "sig_seq_2".into(),
            ..update_v1
        };
        node.submit_state_channel_update(&update_v2, 1100, 600).unwrap();

        // Settle after challenge window expires (epoch 1800 > 1700)
        let settlement = node.release_coral_escrow(task_id, worker_did, 1800).unwrap();
        assert_eq!(settlement.winner_did, worker_did);
        assert_eq!(settlement.amount_cyan, 98.0);
        assert_eq!(settlement.club_dividend_cyan, 2.0);
        assert_eq!(settlement.solana_pda_instruction.final_sequence, 2);
        assert!(settlement.solana_pda_instruction.escrow_vault_pda.starts_with("pda_"));
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

        manifest.security_tier = AppSecurityTier::SandboxedGuest;
        let cmd = LumeriaHostCommand::RequestEscrowCheckout {
            item_id: "chassis_part_v1".into(),
            price_cyan: 25.0,
        };
        let err = node.execute_app_command(&manifest, cmd.clone());
        assert!(err.is_err());

        manifest.security_tier = AppSecurityTier::ClubVerified;
        let success = node.execute_app_command(&manifest, cmd);
        assert!(success.is_ok());
    }
}
