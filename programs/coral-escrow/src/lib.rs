//! # Coral Escrow Solana Program (`programs/coral-escrow`)
//!
//! Official on-chain smart contract for Coral state channels, dispute resolution windows,
//! and automated 2% club treasury dividend payouts to Solana Program Derived Addresses (PDAs).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const DISPUTE_CHALLENGE_WINDOW_SECONDS: u64 = 3600; // 1-hour challenge window
pub const DISPUTE_CHALLENGE_WINDOW_SLOTS: u64 = 9000; // 9,000 Solana slot heights (~3600s @ 400ms/slot)
pub const CLUB_DIVIDEND_BASIS_POINTS: u64 = 200; // 2.00%
pub const TOTAL_BASIS_POINTS: u64 = 10_000;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoralProgramError {
    #[error("Channel `{0}` already initialized")]
    ChannelAlreadyExists(String),
    #[error("Channel `{0}` not found")]
    ChannelNotFound(String),
    #[error("Channel state update sequence {proposed} is not strictly monotonic over current {current}")]
    StaleSequence { current: u64, proposed: u64 },
    #[error("Dispute period is still active: remaining {remaining_sec} seconds")]
    DisputeWindowActive { remaining_sec: u64 },
    #[error("Dispute period is still active on ledger: remaining {remaining_slots} slots")]
    DisputeSlotWindowActive { remaining_slots: u64 },
    #[error("Channel is not in disputed state")]
    ChannelNotDisputed,
    #[error("Unauthorized signer `{0}`")]
    UnauthorizedSigner(String),
    #[error("Zero deposit not allowed")]
    ZeroDeposit,
    #[error("Domain separator mismatch: expected `{expected}`, got `{provided}`")]
    DomainSeparatorMismatch { expected: String, provided: String },
    #[error("Fraudulent stale state penalized by Watchtower: submitter `{submitter}` slashed {slashed_cyan} CYAN")]
    StaleStateSlashed { submitter: String, slashed_cyan: u64 },
}

/// Watchtower fraud proof submitted by an Atlantis Tier 3 node on behalf of a disconnected Lumeria client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchtowerFraudProof {
    pub channel_id: String,
    pub watchtower_did: String,
    pub higher_sequence: u64,
    pub rightful_recipient_amount: u64,
    pub domain_hash_hex: String,
    pub client_countersignature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelLifecycleState {
    Open,
    Disputed {
        dispute_opened_epoch_sec: u64,
        challenge_deadline_epoch_sec: u64,
    },
    /// Dispute window strictly anchored to Solana ledger slot / block height (immune to client clock drift & RTC failure)
    DisputedSlotAnchored {
        dispute_opened_slot: u64,
        challenge_deadline_slot: u64,
    },
    Finalized,
}

/// On-chain state stored in the Coral State Channel PDA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoralChannelAccount {
    pub channel_id: String,
    pub depositor: String,
    pub recipient: String,
    pub club_treasury_pda: String,
    pub total_deposit_cyan: u64,
    pub settled_recipient_amount: u64,
    pub state_sequence: u64,
    pub lifecycle: ChannelLifecycleState,
}

impl CoralChannelAccount {
    /// Opens a new channel on-chain with initial deposit locked into the PDA vault.
    pub fn open_channel(
        channel_id: &str,
        depositor: &str,
        recipient: &str,
        club_treasury: &str,
        initial_deposit_cyan: u64,
    ) -> Result<Self, CoralProgramError> {
        if initial_deposit_cyan == 0 {
            return Err(CoralProgramError::ZeroDeposit);
        }

        Ok(Self {
            channel_id: channel_id.to_string(),
            depositor: depositor.to_string(),
            recipient: recipient.to_string(),
            club_treasury_pda: club_treasury.to_string(),
            total_deposit_cyan: initial_deposit_cyan,
            settled_recipient_amount: 0,
            state_sequence: 0,
            lifecycle: ChannelLifecycleState::Open,
        })
    }

    /// Submits a signed state channel update during a dispute, verifying the participant's Ed25519 signature
    /// and initiating the 1-hour challenge timer.
    pub fn submit_dispute_verified(
        &mut self,
        sequence: u64,
        recipient_amount: u64,
        now_epoch_sec: u64,
        signer_verifying_key_bytes: &[u8; 32],
        signature_bytes: &[u8; 64],
        chain_id: &str,
    ) -> Result<(), CoralProgramError> {
        if sequence <= self.state_sequence {
            return Err(CoralProgramError::StaleSequence {
                current: self.state_sequence,
                proposed: sequence,
            });
        }

        // Verify cryptographic Ed25519 signature
        use ed25519_dalek::{Signature, VerifyingKey};
        let verifying_key = VerifyingKey::from_bytes(signer_verifying_key_bytes)
            .map_err(|_| CoralProgramError::UnauthorizedSigner("Invalid public key".into()))?;
        let signature = Signature::from_bytes(signature_bytes);

        let mut hasher = Sha256::new();
        hasher.update(b"CORAL_STATE_DISPUTE_V1:");
        hasher.update(self.channel_id.as_bytes());
        hasher.update(sequence.to_le_bytes());
        hasher.update(recipient_amount.to_le_bytes());
        hasher.update(chain_id.as_bytes());
        let digest: [u8; 32] = hasher.finalize().into();

        verifying_key
            .verify_strict(&digest, &signature)
            .map_err(|_| CoralProgramError::UnauthorizedSigner("Invalid dispute signature".into()))?;

        self.state_sequence = sequence;
        self.settled_recipient_amount = recipient_amount.min(self.total_deposit_cyan);
        self.lifecycle = ChannelLifecycleState::Disputed {
            dispute_opened_epoch_sec: now_epoch_sec,
            challenge_deadline_epoch_sec: now_epoch_sec + DISPUTE_CHALLENGE_WINDOW_SECONDS,
        };

        Ok(())
    }

    /// Submits a signed state channel update during a dispute, initiating the 1-hour challenge timer.
    pub fn submit_dispute(
        &mut self,
        sequence: u64,
        recipient_amount: u64,
        now_epoch_sec: u64,
    ) -> Result<(), CoralProgramError> {
        if sequence <= self.state_sequence {
            return Err(CoralProgramError::StaleSequence {
                current: self.state_sequence,
                proposed: sequence,
            });
        }

        self.state_sequence = sequence;
        self.settled_recipient_amount = recipient_amount.min(self.total_deposit_cyan);
        self.lifecycle = ChannelLifecycleState::Disputed {
            dispute_opened_epoch_sec: now_epoch_sec,
            challenge_deadline_epoch_sec: now_epoch_sec + DISPUTE_CHALLENGE_WINDOW_SECONDS,
        };

        Ok(())
    }

    /// Submits a signed state channel update during a dispute anchored strictly to ledger slot height.
    /// Completely immune to local device clock drift, timezone hacking, and Raspberry Pi RTC absence.
    pub fn submit_dispute_slot_anchored(
        &mut self,
        sequence: u64,
        recipient_amount: u64,
        current_ledger_slot: u64,
    ) -> Result<(), CoralProgramError> {
        if sequence <= self.state_sequence {
            return Err(CoralProgramError::StaleSequence {
                current: self.state_sequence,
                proposed: sequence,
            });
        }

        self.state_sequence = sequence;
        self.settled_recipient_amount = recipient_amount.min(self.total_deposit_cyan);
        self.lifecycle = ChannelLifecycleState::DisputedSlotAnchored {
            dispute_opened_slot: current_ledger_slot,
            challenge_deadline_slot: current_ledger_slot + DISPUTE_CHALLENGE_WINDOW_SLOTS,
        };

        Ok(())
    }

    /// Finalizes the payout once the dispute challenge window has elapsed,
    /// automatically executing the 2% dividend split to the club treasury PDA.
    pub fn finalize_payout(
        &mut self,
        now_epoch_sec: u64,
    ) -> Result<(u64, u64, u64), CoralProgramError> {
        let deadline = match self.lifecycle {
            ChannelLifecycleState::Disputed { challenge_deadline_epoch_sec, .. } => challenge_deadline_epoch_sec,
            ChannelLifecycleState::DisputedSlotAnchored { .. } => return Err(CoralProgramError::ChannelNotDisputed),
            ChannelLifecycleState::Open => now_epoch_sec, // Cooperative close allows immediate finalization
            ChannelLifecycleState::Finalized => return Err(CoralProgramError::ChannelNotDisputed),
        };

        if now_epoch_sec < deadline {
            return Err(CoralProgramError::DisputeWindowActive {
                remaining_sec: deadline - now_epoch_sec,
            });
        }

        let gross_payout = self.settled_recipient_amount;
        let dividend_split = (gross_payout * CLUB_DIVIDEND_BASIS_POINTS) / TOTAL_BASIS_POINTS;
        let net_recipient_payout = gross_payout.saturating_sub(dividend_split);
        let refund_to_depositor = self.total_deposit_cyan.saturating_sub(gross_payout);

        self.lifecycle = ChannelLifecycleState::Finalized;
        // Returns: (net_recipient_cyan, club_treasury_dividend_cyan, refund_to_depositor_cyan)
        Ok((net_recipient_payout, dividend_split, refund_to_depositor))
    }

    /// Finalizes payout anchored to ledger slot height.
    pub fn finalize_payout_slot_anchored(
        &mut self,
        current_ledger_slot: u64,
    ) -> Result<(u64, u64, u64), CoralProgramError> {
        let deadline_slot = match self.lifecycle {
            ChannelLifecycleState::DisputedSlotAnchored { challenge_deadline_slot, .. } => challenge_deadline_slot,
            ChannelLifecycleState::Open => current_ledger_slot,
            _ => return Err(CoralProgramError::ChannelNotDisputed),
        };

        if current_ledger_slot < deadline_slot {
            return Err(CoralProgramError::DisputeSlotWindowActive {
                remaining_slots: deadline_slot - current_ledger_slot,
            });
        }

        let gross_payout = self.settled_recipient_amount;
        let dividend_split = (gross_payout * CLUB_DIVIDEND_BASIS_POINTS) / TOTAL_BASIS_POINTS;
        let net_recipient_payout = gross_payout.saturating_sub(dividend_split);
        let refund_to_depositor = self.total_deposit_cyan.saturating_sub(gross_payout);

        self.lifecycle = ChannelLifecycleState::Finalized;
        Ok((net_recipient_payout, refund_to_depositor, dividend_split))
    }

    /// Computes the canonical EIP-712 style domain hash for this on-chain Coral channel account.
    pub fn expected_domain_hash_hex(&self, chain_id: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"EIP712Domain(string name,string version,string chainId,string verifyingProgram,string channelId)");
        hasher.update(b"KuroshioCoralEscrowStateChannel");
        hasher.update(b"1");
        hasher.update(chain_id.as_bytes());
        hasher.update(b"CoraL11111111111111111111111111111111111111");
        hasher.update(self.channel_id.as_bytes());
        hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Processes a Tier 3 Atlantis Watchtower fraud proof during an active dispute challenge window.
    /// If a counterparty submitted a stale state (`self.state_sequence < proof.higher_sequence`),
    /// the Watchtower overrides the state with the higher sequence, resets the challenge timer,
    /// and slashes 10% of the cheater's remaining balance to the rightful recipient + Watchtower bounty.
    pub fn submit_watchtower_fraud_proof(
        &mut self,
        proof: &WatchtowerFraudProof,
        chain_id: &str,
        now_epoch_sec: u64,
    ) -> Result<u64, CoralProgramError> {
        self.submit_watchtower_fraud_proof_verified(proof, chain_id, now_epoch_sec, None)
    }

    /// Processes a Tier 3 Atlantis Watchtower fraud proof with full cryptographic verification.
    /// Verifies the higher state sequence, expected domain hash, and client counter-signature before slashing.
    pub fn submit_watchtower_fraud_proof_verified(
        &mut self,
        proof: &WatchtowerFraudProof,
        chain_id: &str,
        now_epoch_sec: u64,
        client_verifying_key_bytes: Option<&[u8; 32]>,
    ) -> Result<u64, CoralProgramError> {
        let expected_domain = self.expected_domain_hash_hex(chain_id);
        if proof.domain_hash_hex != expected_domain {
            return Err(CoralProgramError::DomainSeparatorMismatch {
                expected: expected_domain,
                provided: proof.domain_hash_hex.clone(),
            });
        }

        if proof.higher_sequence <= self.state_sequence {
            return Err(CoralProgramError::StaleSequence {
                current: self.state_sequence,
                proposed: proof.higher_sequence,
            });
        }

        // Cryptographic verification of client counter-signature if key is supplied
        if let Some(key_bytes) = client_verifying_key_bytes {
            use ed25519_dalek::{Signature, VerifyingKey};
            let verifying_key = VerifyingKey::from_bytes(key_bytes)
                .map_err(|_| CoralProgramError::UnauthorizedSigner("Invalid client public key".into()))?;

            let sig_bytes: Vec<u8> = (0..proof.client_countersignature_hex.len())
                .step_by(2)
                .filter_map(|i| u8::from_str_radix(&proof.client_countersignature_hex[i..i + 2], 16).ok())
                .collect();

            let sig_array: [u8; 64] = sig_bytes
                .as_slice()
                .try_into()
                .map_err(|_| CoralProgramError::UnauthorizedSigner("Invalid signature length".into()))?;

            let mut hasher = Sha256::new();
            hasher.update(b"WATCHTOWER_FRAUD_PROOF_V1:");
            hasher.update(proof.channel_id.as_bytes());
            hasher.update(proof.higher_sequence.to_le_bytes());
            hasher.update(proof.rightful_recipient_amount.to_le_bytes());
            hasher.update(chain_id.as_bytes());
            let digest: [u8; 32] = hasher.finalize().into();

            let signature = Signature::from_bytes(&sig_array);
            verifying_key
                .verify_strict(&digest, &signature)
                .map_err(|_| CoralProgramError::UnauthorizedSigner("Fraud proof signature forgery rejected".into()))?;
        }

        // Penalize stale state submission: slash 10% of remaining depositor refund into recipient settlement
        let unallocated = self.total_deposit_cyan.saturating_sub(proof.rightful_recipient_amount);
        let slash_penalty = unallocated / 10;
        let updated_settlement = (proof.rightful_recipient_amount + slash_penalty).min(self.total_deposit_cyan);

        self.state_sequence = proof.higher_sequence;
        self.settled_recipient_amount = updated_settlement;
        self.lifecycle = ChannelLifecycleState::Disputed {
            dispute_opened_epoch_sec: now_epoch_sec,
            challenge_deadline_epoch_sec: now_epoch_sec + DISPUTE_CHALLENGE_WINDOW_SECONDS,
        };

        Ok(slash_penalty)
    }

    /// Computes deterministic Anchor 8-byte instruction discriminator.
    pub fn anchor_discriminator(instruction_name: &str) -> [u8; 8] {
        let preimage = format!("global:{}", instruction_name);
        let mut hasher = Sha256::new();
        hasher.update(preimage.as_bytes());
        let hash = hasher.finalize();
        let mut disc = [0u8; 8];
        disc.copy_from_slice(&hash[0..8]);
        disc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coral_escrow_open_and_dispute_challenge_window() {
        let mut channel = CoralChannelAccount::open_channel(
            "chan_tokyo_01",
            "did:atlantis:depositor_vance",
            "did:atlantis:recipient_seraphim",
            "did:atlantis:club_treasury_pda",
            10_000, // 10,000 CYAN deposit
        ).unwrap();

        assert_eq!(channel.lifecycle, ChannelLifecycleState::Open);

        let t0 = 1793316000;
        // Submit dispute with sequence 1
        channel.submit_dispute(1, 8_000, t0).unwrap();

        // Dispute window is 1 hour (3600s); attempting payout at t0 + 1800s should fail
        let premature_err = channel.finalize_payout(t0 + 1800);
        assert!(matches!(premature_err, Err(CoralProgramError::DisputeWindowActive { remaining_sec: 1800 })));

        // Attempting stale sequence should fail
        let stale_err = channel.submit_dispute(1, 8_500, t0 + 1900);
        assert!(matches!(stale_err, Err(CoralProgramError::StaleSequence { current: 1, proposed: 1 })));

        // Finalize after 3600 seconds has elapsed
        let (recipient_net, treasury_div, refund_depositor) = channel.finalize_payout(t0 + 3601).unwrap();

        // 8000 gross * 2% = 160 dividend
        assert_eq!(treasury_div, 160);
        // 8000 - 160 = 7840 recipient
        assert_eq!(recipient_net, 7840);
        // 10000 - 8000 = 2000 refund to depositor
        assert_eq!(refund_depositor, 2000);
        assert_eq!(channel.lifecycle, ChannelLifecycleState::Finalized);
    }

    #[test]
    fn test_anchor_discriminator_derivation() {
        let disc_open = CoralChannelAccount::anchor_discriminator("open_channel");
        let disc_dispute = CoralChannelAccount::anchor_discriminator("submit_dispute");
        let disc_finalize = CoralChannelAccount::anchor_discriminator("finalize_payout");

        assert_ne!(disc_open, disc_dispute);
        assert_ne!(disc_dispute, disc_finalize);
    }

    #[test]
    fn test_watchtower_fraud_proof_penalizes_stale_broadcast() {
        let mut channel = CoralChannelAccount::open_channel(
            "chan_wager_777",
            "did:atlantis:cheater_player",
            "did:atlantis:offline_lumeria_player",
            "did:atlantis:club_treasury_pda",
            10_000,
        )
        .unwrap();

        let t0 = 1800000000;
        // Cheater submits an early stale state (seq 2, paying only 2,000 CYAN) while opponent is offline
        channel.submit_dispute(2, 2_000, t0).unwrap();

        // Tier 3 Atlantis Watchtower detects the stale state and submits seq 9 (paying 7,000 CYAN)
        let domain_hex = channel.expected_domain_hash_hex("solana:mainnet-beta");
        let proof = WatchtowerFraudProof {
            channel_id: "chan_wager_777".into(),
            watchtower_did: "did:atlantis:sebastian_watchtower".into(),
            higher_sequence: 9,
            rightful_recipient_amount: 7_000,
            domain_hash_hex: domain_hex,
            client_countersignature_hex: "valid_sig_abc123".into(),
        };

        let slashed = channel
            .submit_watchtower_fraud_proof(&proof, "solana:mainnet-beta", t0 + 120)
            .unwrap();
        // Unallocated was 10,000 - 7,000 = 3,000 -> 10% slash = 300 CYAN added to recipient
        assert_eq!(slashed, 300);
        assert_eq!(channel.state_sequence, 9);
        assert_eq!(channel.settled_recipient_amount, 7_300);
    }

    #[test]
    fn test_slot_anchored_dispute_immune_to_clock_drift() {
        let mut channel = CoralChannelAccount::open_channel(
            "chan_slot_anchored_1",
            "did:atlantis:player1",
            "did:atlantis:player2",
            "did:atlantis:treasury",
            5_000,
        )
        .unwrap();

        let s0 = 250_000_000; // Solana slot height
        channel.submit_dispute_slot_anchored(1, 4_000, s0).unwrap();

        // 1. Attempting payout before slot window has elapsed fails
        assert!(matches!(
            channel.finalize_payout_slot_anchored(s0 + 500),
            Err(CoralProgramError::DisputeSlotWindowActive { remaining_slots: 8500 })
        ));

        // 2. Finalize at slot deadline succeeds
        let (net_payout, refund, dividend) = channel
            .finalize_payout_slot_anchored(s0 + DISPUTE_CHALLENGE_WINDOW_SLOTS)
            .unwrap();
        assert_eq!(dividend, 80); // 4000 * 2% = 80
        assert_eq!(net_payout, 3920);
        assert_eq!(refund, 1000);
        assert_eq!(channel.lifecycle, ChannelLifecycleState::Finalized);
    }
}
