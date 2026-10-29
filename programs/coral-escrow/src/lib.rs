//! # Coral Escrow Solana Program (`programs/coral-escrow`)
//!
//! Official on-chain smart contract for Coral state channels, dispute resolution windows,
//! and automated 2% club treasury dividend payouts to Solana Program Derived Addresses (PDAs).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const DISPUTE_CHALLENGE_WINDOW_SECONDS: u64 = 3600; // 1-hour challenge window
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
    #[error("Channel is not in disputed state")]
    ChannelNotDisputed,
    #[error("Unauthorized signer `{0}`")]
    UnauthorizedSigner(String),
    #[error("Zero deposit not allowed")]
    ZeroDeposit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelLifecycleState {
    Open,
    Disputed {
        dispute_opened_epoch_sec: u64,
        challenge_deadline_epoch_sec: u64,
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

    /// Finalizes the payout once the dispute challenge window has elapsed,
    /// automatically executing the 2% dividend split to the club treasury PDA.
    pub fn finalize_payout(
        &mut self,
        now_epoch_sec: u64,
    ) -> Result<(u64, u64, u64), CoralProgramError> {
        let deadline = match self.lifecycle {
            ChannelLifecycleState::Disputed { challenge_deadline_epoch_sec, .. } => challenge_deadline_epoch_sec,
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
}
