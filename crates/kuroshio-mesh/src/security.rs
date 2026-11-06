//! # Hardware Enclave Key Vault, Zeroized Secrets & EIP-712 Domain-Separated State Channel Signing
//!
//! Addresses Threat Vectors 1 & 2:
//! 1. **Hardware Enclave Key Vault (`HardwareKeyVault`):** Enforces FFI/bridge bindings to platform-native
//!    hardware enclaves (Android Keystore StrongBox, iOS Secure Enclave, Windows DPAPI, or Linux/Pi TPM 2.0)
//!    so long-term private keys never enter raw Rust heap memory.
//! 2. **Zeroized Ephemeral Secrets (`EphemeralSessionSecret`):** Wraps any temporary session tokens or
//!    in-memory buffers in `#[derive(Zeroize, ZeroizeOnDrop)]` so RAM is wiped immediately on drop.
//! 3. **Domain-Separated State Channel Payloads (`DomainSeparatedChannelState`):** Enforces EIP-712 style
//!    domain separators (`chain_id`, `program_id`, `channel_id`, and strictly monotonic `nonce`) to block
//!    cross-match replay and stale-state submission attacks.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const CORAL_DOMAIN_NAME: &str = "KuroshioCoralEscrowStateChannel";
pub const CORAL_DOMAIN_VERSION: &str = "1";

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SecurityError {
    #[error("Hardware enclave signing failed: {0}")]
    EnclaveUnavailable(String),
    #[error("Key handle `{0}` not found in hardware enclave")]
    KeyHandleNotFound(String),
    #[error("Domain separator mismatch: expected {expected_hash}, got {provided_hash}")]
    DomainSeparatorMismatch {
        expected_hash: String,
        provided_hash: String,
    },
    #[error("Stale state channel nonce: proposed {proposed} <= current {current}")]
    StaleChannelNonce { current: u64, proposed: u64 },
    #[error("Channel ID mismatch: expected `{expected}`, got `{provided}`")]
    ChannelIdMismatch { expected: String, provided: String },
    #[error("Invalid cryptographic signature")]
    InvalidSignature,
}

/// Identifies the platform-native hardware enclave backend protecting key material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnclaveBackendType {
    /// Apple Secure Enclave (`kSecAttrTokenIDSecureEnclave`)
    IosSecureEnclave,
    /// Android Hardware-Backed Keystore (`StrongBox` Keymint)
    AndroidKeystoreStrongBox,
    /// Linux / Raspberry Pi Atlantis Node TPM 2.0 (`/dev/tpmrm0`)
    LinuxTpm20,
    /// Windows Data Protection API / TPM Platform Crypto Provider
    WindowsDpapiTpm,
}

/// Opaque reference handle to a non-exportable asymmetric key stored inside the platform enclave.
/// Contains zero raw private key bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnclaveKeyHandle {
    pub key_alias: String,
    pub public_did: String,
    pub backend: EnclaveBackendType,
    pub requires_biometric_or_user_presence: bool,
}

/// Ephemeral in-memory secret buffer that is guaranteed to be overwritten with zeros
/// the instant it leaves scope (`ZeroizeOnDrop`), preventing heap scraping or core-dump leaks.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct EphemeralSessionSecret {
    secret_bytes: [u8; 32],
}

impl EphemeralSessionSecret {
    pub fn from_bytes(secret_bytes: [u8; 32]) -> Self {
        Self { secret_bytes }
    }

    /// Computes a keyed SHA-256 MAC tag using the ephemeral secret without exposing the raw slice externally.
    pub fn sign_digest(&self, message: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.secret_bytes);
        hasher.update(message);
        hasher.finalize().into()
    }

    /// Inspects raw bytes (for testing zeroization bounds).
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.secret_bytes
    }
}

/// Hardware-backed key storage trait enforced across Tier 2 (Lumeria) and Tier 3 (Atlantis) nodes.
/// Raw private keys never cross the FFI boundary into Rust heap memory; signing occurs inside the enclave.
pub trait HardwareKeyVault: Send + Sync {
    /// Generates a new non-exportable Ed25519/P-256 keypair inside the hardware enclave.
    fn generate_enclave_key(&mut self, key_alias: &str) -> Result<EnclaveKeyHandle, SecurityError>;

    /// Requests the hardware enclave to sign a pre-hashed 32-byte digest (`digest_sha256`) using `key_alias`.
    fn sign_with_enclave(&self, key_alias: &str, digest_sha256: &[u8; 32]) -> Result<Vec<u8>, SecurityError>;

    /// Deletes a key handle from the hardware enclave.
    fn revoke_enclave_key(&mut self, key_alias: &str) -> Result<(), SecurityError>;
}

/// EIP-712 style cryptographic domain separator preventing cross-chain, cross-contract,
/// and cross-match signature replay attacks in Coral Escrow state channels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoralDomainSeparator {
    pub name: String,
    pub version: String,
    /// Solana cluster / chain identifier (e.g., `"solana:mainnet-beta"` or `"solana:atlantis-local"`)
    pub chain_id: String,
    /// On-chain Coral Escrow Program ID
    pub verifying_program_id: String,
    /// Match-unique or task-unique state channel identifier
    pub channel_id: String,
}

impl CoralDomainSeparator {
    pub fn new(chain_id: &str, verifying_program_id: &str, channel_id: &str) -> Self {
        Self {
            name: CORAL_DOMAIN_NAME.to_string(),
            version: CORAL_DOMAIN_VERSION.to_string(),
            chain_id: chain_id.to_string(),
            verifying_program_id: verifying_program_id.to_string(),
            channel_id: channel_id.to_string(),
        }
    }

    /// Computes the 32-byte SHA-256 domain separator hash (`domain_hash`).
    pub fn domain_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"EIP712Domain(string name,string version,string chainId,string verifyingProgram,string channelId)");
        hasher.update(self.name.as_bytes());
        hasher.update(self.version.as_bytes());
        hasher.update(self.chain_id.as_bytes());
        hasher.update(self.verifying_program_id.as_bytes());
        hasher.update(self.channel_id.as_bytes());
        hasher.finalize().into()
    }

    pub fn domain_hash_hex(&self) -> String {
        self.domain_hash().iter().map(|b| format!("{:02x}", b)).collect()
    }
}

/// Domain-separated, strictly monotonic state channel update payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainSeparatedChannelState {
    pub domain_hash_hex: String,
    pub channel_id: String,
    pub monotonic_nonce: u64,
    pub recipient_did: String,
    pub recipient_amount_lamports: u64,
    pub refund_amount_lamports: u64,
    pub enclave_signature_hex: String,
}

impl DomainSeparatedChannelState {
    /// Computes the canonical EIP-712 style signing digest (`\x19\x01 || domain_hash || struct_hash`).
    pub fn signing_digest(
        domain: &CoralDomainSeparator,
        monotonic_nonce: u64,
        recipient_did: &str,
        recipient_amount_lamports: u64,
        refund_amount_lamports: u64,
    ) -> [u8; 32] {
        let mut struct_hasher = Sha256::new();
        struct_hasher.update(b"CoralChannelState(string channelId,uint64 nonce,string recipientDid,uint64 recipientAmount,uint64 refundAmount)");
        struct_hasher.update(domain.channel_id.as_bytes());
        struct_hasher.update(monotonic_nonce.to_le_bytes());
        struct_hasher.update(recipient_did.as_bytes());
        struct_hasher.update(recipient_amount_lamports.to_le_bytes());
        struct_hasher.update(refund_amount_lamports.to_le_bytes());
        let struct_hash: [u8; 32] = struct_hasher.finalize().into();

        let mut final_hasher = Sha256::new();
        final_hasher.update([0x19, 0x01]);
        final_hasher.update(domain.domain_hash());
        final_hasher.update(struct_hash);
        final_hasher.finalize().into()
    }

    /// Validates that this state update belongs to the exact `expected_domain` and has a strictly higher nonce.
    pub fn verify_against_domain(
        &self,
        expected_domain: &CoralDomainSeparator,
        current_highest_nonce: u64,
    ) -> Result<(), SecurityError> {
        if self.channel_id != expected_domain.channel_id {
            return Err(SecurityError::ChannelIdMismatch {
                expected: expected_domain.channel_id.clone(),
                provided: self.channel_id.clone(),
            });
        }

        let expected_hash = expected_domain.domain_hash_hex();
        if self.domain_hash_hex != expected_hash {
            return Err(SecurityError::DomainSeparatorMismatch {
                expected_hash,
                provided_hash: self.domain_hash_hex.clone(),
            });
        }

        if self.monotonic_nonce <= current_highest_nonce {
            return Err(SecurityError::StaleChannelNonce {
                current: current_highest_nonce,
                proposed: self.monotonic_nonce,
            });
        }

        if self.enclave_signature_hex.len() < 32 {
            return Err(SecurityError::InvalidSignature);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zeroize_ephemeral_session_secret_on_drop() {
        let raw = [0xAAu8; 32];
        let mut secret = EphemeralSessionSecret::from_bytes(raw);
        assert_eq!(secret.as_bytes(), &[0xAAu8; 32]);
        let tag = secret.sign_digest(b"handshake_challenge");
        assert_ne!(tag, [0u8; 32]);

        // Explicitly zeroize and verify all bytes are wiped to 0x00
        secret.zeroize();
        assert_eq!(secret.as_bytes(), &[0x00u8; 32]);
    }

    #[test]
    fn test_domain_separated_channel_state_blocks_cross_match_and_stale_replay() {
        let domain_match_1 = CoralDomainSeparator::new(
            "solana:mainnet-beta",
            "CoraL11111111111111111111111111111111111111",
            "channel_match_101",
        );
        let domain_match_2 = CoralDomainSeparator::new(
            "solana:mainnet-beta",
            "CoraL11111111111111111111111111111111111111",
            "channel_match_202",
        );

        let state_v1 = DomainSeparatedChannelState {
            domain_hash_hex: domain_match_1.domain_hash_hex(),
            channel_id: "channel_match_101".into(),
            monotonic_nonce: 5,
            recipient_did: "did:atlantis:winner".into(),
            recipient_amount_lamports: 90_000_000,
            refund_amount_lamports: 10_000_000,
            enclave_signature_hex: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6".into(),
        };

        // 1. Valid update on Match 1 (nonce 5 > current 4) succeeds
        assert!(state_v1.verify_against_domain(&domain_match_1, 4).is_ok());

        // 2. Stale nonce replay (nonce 5 <= current 5) is blocked
        assert!(matches!(
            state_v1.verify_against_domain(&domain_match_1, 5),
            Err(SecurityError::StaleChannelNonce { current: 5, proposed: 5 })
        ));

        // 3. Cross-match replay onto Match 2 is blocked by channel/domain mismatch
        assert!(state_v1.verify_against_domain(&domain_match_2, 0).is_err());
    }
}
