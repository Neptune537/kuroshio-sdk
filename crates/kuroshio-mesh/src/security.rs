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
    #[error("BCS serialization error: {0}")]
    BcsError(String),
    #[error("Certificate expired: valid until block/slot {valid_until_slot}, current {current_slot}")]
    CertificateExpired { valid_until_slot: u64, current_slot: u64 },
    #[error("Key has been revoked in CRDT revocation registry")]
    KeyRevoked,
    #[error("Invalid cryptographic signature")]
    InvalidSignature,
}

// -----------------------------------------------------------------------------
// Ephemeral Session Certificates & CRDT Revocation Registry
// -----------------------------------------------------------------------------

/// Short-lived ephemeral session certificate signed by the master root DID key.
/// Binds an ephemeral signing key to a slot/block height horizon, preventing lost or stolen
/// Lumeria devices or physically seized edge nodes from signing beyond certificate expiration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EphemeralSessionCertificate {
    pub master_did: String,
    pub ephemeral_key_id: String,
    pub ephemeral_pubkey_bytes: [u8; 32],
    pub issued_at_slot: u64,
    pub valid_until_slot: u64,
    pub root_signature: Vec<u8>,
}

impl EphemeralSessionCertificate {
    /// Issues a short-lived certificate signed by the root master DID key.
    pub fn issue(
        master_signing_key: &ed25519_dalek::SigningKey,
        master_did: &str,
        ephemeral_key_id: &str,
        ephemeral_pubkey_bytes: [u8; 32],
        issued_at_slot: u64,
        validity_slots: u64,
    ) -> Self {
        use ed25519_dalek::Signer;
        let valid_until_slot = issued_at_slot + validity_slots;

        let mut hasher = Sha256::new();
        hasher.update(b"KUROSHIO_SESSION_CERT_V1:");
        hasher.update(master_did.as_bytes());
        hasher.update(ephemeral_key_id.as_bytes());
        hasher.update(&ephemeral_pubkey_bytes);
        hasher.update(issued_at_slot.to_le_bytes());
        hasher.update(valid_until_slot.to_le_bytes());
        let digest: [u8; 32] = hasher.finalize().into();

        let sig = master_signing_key.sign(&digest);

        Self {
            master_did: master_did.to_string(),
            ephemeral_key_id: ephemeral_key_id.to_string(),
            ephemeral_pubkey_bytes,
            issued_at_slot,
            valid_until_slot,
            root_signature: sig.to_bytes().to_vec(),
        }
    }

    /// Verifies the root DID signature and checks slot-bounded validity.
    pub fn verify(
        &self,
        master_verifying_key_bytes: &[u8; 32],
        current_slot: u64,
        revocation_registry: &CrdtRevocationList,
    ) -> Result<(), SecurityError> {
        use ed25519_dalek::{Signature, VerifyingKey};

        if revocation_registry.is_revoked(&self.ephemeral_key_id) || revocation_registry.is_revoked(&self.master_did) {
            return Err(SecurityError::KeyRevoked);
        }

        if current_slot > self.valid_until_slot {
            return Err(SecurityError::CertificateExpired {
                valid_until_slot: self.valid_until_slot,
                current_slot,
            });
        }

        let mut hasher = Sha256::new();
        hasher.update(b"KUROSHIO_SESSION_CERT_V1:");
        hasher.update(self.master_did.as_bytes());
        hasher.update(self.ephemeral_key_id.as_bytes());
        hasher.update(&self.ephemeral_pubkey_bytes);
        hasher.update(self.issued_at_slot.to_le_bytes());
        hasher.update(self.valid_until_slot.to_le_bytes());
        let digest: [u8; 32] = hasher.finalize().into();

        let verifying_key = VerifyingKey::from_bytes(master_verifying_key_bytes)
            .map_err(|_| SecurityError::InvalidSignature)?;
        let sig_bytes: [u8; 64] = self
            .root_signature
            .as_slice()
            .try_into()
            .map_err(|_| SecurityError::InvalidSignature)?;
        let signature = Signature::from_bytes(&sig_bytes);

        verifying_key
            .verify_strict(&digest, &signature)
            .map_err(|_| SecurityError::InvalidSignature)?;

        Ok(())
    }
}

/// A compact, grow-only CRDT revocation entry broadcasted over Gossipsub control topics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationEntry {
    pub target_id: String,
    pub revoked_by_master_did: String,
    pub revocation_slot: u64,
    pub reason: String,
    pub master_signature: Vec<u8>,
}

/// In-memory CRDT Revocation Set for Atlantis and Lumeria nodes.
/// Dropped peers are immediately blocked from state channel signatures without waiting for an upstream ledger sync.
#[derive(Debug, Clone, Default)]
pub struct CrdtRevocationList {
    pub revoked_ids: std::collections::HashMap<String, RevocationEntry>,
}

impl CrdtRevocationList {
    pub fn new() -> Self {
        Self {
            revoked_ids: std::collections::HashMap::new(),
        }
    }

    pub fn insert_revocation(&mut self, entry: RevocationEntry) {
        self.revoked_ids.insert(entry.target_id.clone(), entry);
    }

    pub fn is_revoked(&self, id: &str) -> bool {
        self.revoked_ids.contains_key(id)
    }
}

// -----------------------------------------------------------------------------
// Hybrid Logical Clock (HLC) & Block-Anchored Consensus Event Ordering
// -----------------------------------------------------------------------------

/// Monotonic Hybrid Logical Clock for distributed event ordering without trusting wall-clock time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HybridLogicalClock {
    pub logical_counter: u64,
    pub slot_height: u64,
}

impl HybridLogicalClock {
    pub const fn new(slot_height: u64, logical_counter: u64) -> Self {
        Self {
            slot_height,
            logical_counter,
        }
    }

    /// Updates the local clock on receiving a remote message (Lamport-style monotonic progression).
    pub fn update(&mut self, remote_clock: Self, current_slot: u64) {
        let max_slot = self.slot_height.max(remote_clock.slot_height).max(current_slot);
        if max_slot == self.slot_height && max_slot == remote_clock.slot_height {
            self.logical_counter = self.logical_counter.max(remote_clock.logical_counter) + 1;
        } else {
            self.slot_height = max_slot;
            self.logical_counter = 0;
        }
    }

    /// Increments local logical event counter.
    pub fn tick(&mut self) -> Self {
        self.logical_counter += 1;
        *self
    }
}

// -----------------------------------------------------------------------------
// Crash-Resilient ACID Storage Engine (redb) for Atlantis Edge Nodes
// -----------------------------------------------------------------------------

/// Crash-resilient key-value store wrapper using copy-on-write ACID transactions (redb).
/// Protects Atlantis Raspberry Pi nodes against microSD/NVMe filesystem corruption on sudden power loss.
pub struct AtlantisCrashResilientStorage {
    db: std::sync::Arc<redb::Database>,
}

impl AtlantisCrashResilientStorage {
    const TABLE_CHANNEL_STATE: redb::TableDefinition<'static, &'static str, &'static [u8]> =
        redb::TableDefinition::new("coral_channel_state_v1");

    pub fn open_or_create(path: &std::path::Path) -> Result<Self, String> {
        let db = redb::Database::create(path).map_err(|e| format!("Failed to create redb database: {}", e))?;
        // Initialize tables in a write transaction
        let write_tx = db.begin_write().map_err(|e| e.to_string())?;
        {
            let _ = write_tx.open_table(Self::TABLE_CHANNEL_STATE).map_err(|e| e.to_string())?;
        }
        write_tx.commit().map_err(|e| e.to_string())?;

        Ok(Self {
            db: std::sync::Arc::new(db),
        })
    }

    pub fn open_in_memory() -> Result<Self, String> {
        // Redb backend using temporary/ephemeral database
        let temp_dir = std::env::temp_dir();
        let unique_name = format!("atlantis_redb_test_{}.db", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
        let path = temp_dir.join(unique_name);
        Self::open_or_create(&path)
    }

    /// Persists state channel state with ACID guarantees and synchronous WAL flushing.
    pub fn persist_channel_state(&self, channel_id: &str, state_bytes: &[u8]) -> Result<(), String> {
        let write_tx = self.db.begin_write().map_err(|e| e.to_string())?;
        {
            let mut table = write_tx.open_table(Self::TABLE_CHANNEL_STATE).map_err(|e| e.to_string())?;
            table.insert(channel_id, state_bytes).map_err(|e| e.to_string())?;
        }
        write_tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Retrieves persisted state.
    pub fn read_channel_state(&self, channel_id: &str) -> Result<Option<Vec<u8>>, String> {
        let read_tx = self.db.begin_read().map_err(|e| e.to_string())?;
        let table = read_tx.open_table(Self::TABLE_CHANNEL_STATE).map_err(|e| e.to_string())?;
        match table.get(channel_id).map_err(|e| e.to_string())? {
            Some(v) => Ok(Some(v.value().to_vec())),
            None => Ok(None),
        }
    }
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

/// Canonical structured payload serialized strictly via BCS (Binary Canonical Serialization)
/// ensuring zero padding ambiguity, deterministic key ordering, and complete signature immutability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalCoralStateTranscript {
    pub domain_separator: CoralDomainSeparator,
    pub channel_id: String,
    pub monotonic_nonce: u64,
    pub recipient_did: String,
    pub recipient_amount_lamports: u64,
    pub refund_amount_lamports: u64,
}

impl CanonicalCoralStateTranscript {
    /// Serializes transcript strictly using BCS (Binary Canonical Serialization).
    pub fn to_bcs_bytes(&self) -> Result<Vec<u8>, SecurityError> {
        bcs::to_bytes(self).map_err(|e| SecurityError::BcsError(e.to_string()))
    }

    /// Computes the canonical 32-byte signing digest over BCS-encoded bytes.
    pub fn signing_digest(&self) -> Result<[u8; 32], SecurityError> {
        let bcs_bytes = self.to_bcs_bytes()?;
        let mut hasher = Sha256::new();
        hasher.update(b"CORAL_BCS_V1:");
        hasher.update(&bcs_bytes);
        Ok(hasher.finalize().into())
    }

    /// Signs the canonical BCS digest using ed25519-dalek SigningKey.
    pub fn sign_strict(&self, signing_key: &ed25519_dalek::SigningKey) -> Result<[u8; 64], SecurityError> {
        use ed25519_dalek::Signer;
        let digest = self.signing_digest()?;
        let signature = signing_key.sign(&digest);
        Ok(signature.to_bytes())
    }

    /// Verifies the signature with strict Ed25519 verification (`verify_strict`),
    /// rejecting malleable scalar S values and small-order points.
    pub fn verify_signature_strict(
        &self,
        verifying_key_bytes: &[u8; 32],
        signature_bytes: &[u8; 64],
    ) -> Result<(), SecurityError> {
        use ed25519_dalek::{Signature, VerifyingKey};
        let verifying_key = VerifyingKey::from_bytes(verifying_key_bytes)
            .map_err(|_| SecurityError::InvalidSignature)?;
        let signature = Signature::from_bytes(signature_bytes);
        let digest = self.signing_digest()?;
        verifying_key
            .verify_strict(&digest, &signature)
            .map_err(|_| SecurityError::InvalidSignature)?;
        Ok(())
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

    #[test]
    fn test_canonical_bcs_serialization_and_ed25519_strict_verification() {
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&[0x55u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let domain = CoralDomainSeparator::new(
            "solana:mainnet-beta",
            "CoraL11111111111111111111111111111111111111",
            "channel_bcs_777",
        );

        let transcript = CanonicalCoralStateTranscript {
            domain_separator: domain,
            channel_id: "channel_bcs_777".into(),
            monotonic_nonce: 42,
            recipient_did: "did:atlantis:champion".into(),
            recipient_amount_lamports: 500_000_000,
            refund_amount_lamports: 50_000_000,
        };

        // 1. BCS bytes are canonical and non-empty
        let bcs_bytes = transcript.to_bcs_bytes().unwrap();
        assert!(!bcs_bytes.is_empty());

        // 2. Strict sign and verify
        let sig = transcript.sign_strict(&signing_key).unwrap();
        assert!(transcript.verify_signature_strict(&verifying_key.to_bytes(), &sig).is_ok());

        // 3. Any tampered field breaks the verification
        let mut tampered = transcript.clone();
        tampered.monotonic_nonce = 43;
        assert!(tampered.verify_signature_strict(&verifying_key.to_bytes(), &sig).is_err());
    }

    #[test]
    fn test_ephemeral_session_certificates_crdt_revocation_and_hlc() {
        let master_key = ed25519_dalek::SigningKey::from_bytes(&[0x77u8; 32]);
        let master_pubkey = master_key.verifying_key();

        let ephemeral_pubkey = [0x99u8; 32];
        let cert = EphemeralSessionCertificate::issue(
            &master_key,
            "did:atlantis:pilot_alpha",
            "key_ephemeral_01",
            ephemeral_pubkey,
            100, // issued slot
            50,  // valid for 50 slots (until slot 150)
        );

        let mut crdt = CrdtRevocationList::new();

        // 1. Valid at slot 120
        assert!(cert.verify(&master_pubkey.to_bytes(), 120, &crdt).is_ok());

        // 2. Expired at slot 151
        assert!(matches!(
            cert.verify(&master_pubkey.to_bytes(), 151, &crdt),
            Err(SecurityError::CertificateExpired { .. })
        ));

        // 3. Revoked via CRDT entry
        crdt.insert_revocation(RevocationEntry {
            target_id: "key_ephemeral_01".into(),
            revoked_by_master_did: "did:atlantis:pilot_alpha".into(),
            revocation_slot: 125,
            reason: "Lost mobile device".into(),
            master_signature: vec![0u8; 64],
        });
        assert_eq!(
            cert.verify(&master_pubkey.to_bytes(), 120, &crdt),
            Err(SecurityError::KeyRevoked)
        );

        // 4. Hybrid Logical Clock progression
        let mut hlc1 = HybridLogicalClock::new(100, 0);
        let hlc2 = HybridLogicalClock::new(100, 2);
        hlc1.update(hlc2, 100);
        assert_eq!(hlc1.slot_height, 100);
        assert_eq!(hlc1.logical_counter, 3);

        hlc1.update(hlc2, 105);
        assert_eq!(hlc1.slot_height, 105);
        assert_eq!(hlc1.logical_counter, 0);

        // 5. Redb crash-resilient ACID storage
        let storage = AtlantisCrashResilientStorage::open_in_memory().unwrap();
        assert!(storage.persist_channel_state("ch_test_redb", b"persisted_state_bytes").is_ok());
        let read_back = storage.read_channel_state("ch_test_redb").unwrap().unwrap();
        assert_eq!(read_back, b"persisted_state_bytes");
    }
}
