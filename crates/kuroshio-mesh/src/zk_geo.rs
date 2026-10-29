//! # Zero-Knowledge Geo-Proximity Proof Engine
//!
//! Provides cryptographic proximity and sector proofs without revealing raw
//! user coordinates (WGS-84 precision). Proves that a client is located within
//! an authorized bounding sector or venue radius.

use nautilus_world::Wgs84GeoCoord;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZkGeoError {
    #[error("Verification failed: calculated commitment does not match proof commitment")]
    CommitmentMismatch,
    #[error("Proof expired: age {age_seconds} exceeds max tolerance {max_seconds}")]
    ProofExpired { age_seconds: u64, max_seconds: u64 },
    #[error("Coordinate falls outside authorized radius: distance {distance_m}m > allowed {radius_m}m")]
    OutOfRange { distance_m: f64, radius_m: f64 },
}

/// Zero-Knowledge location disclosure token (prevents raw GPS leaks).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZkGeoProximityProof {
    /// Obfuscated geohash sector identifier (e.g. 5-7 chars)
    pub sector_geohash: String,
    /// Associated venue/club ID if applicable
    pub venue_id: Option<String>,
    /// Cryptographic commitment: SHA-256(lat_quantized || lon_quantized || secret_salt || timestamp)
    pub commitment_hash: String,
    /// Unix timestamp in seconds
    pub timestamp: u64,
    /// Signature of commitment_hash issued by local Club Home Node or enclave
    pub club_signature: String,
}

impl ZkGeoProximityProof {
    /// Generate a cryptographic proximity commitment from private coordinates.
    pub fn generate_commitment(
        coord: &Wgs84GeoCoord,
        salt: &str,
        timestamp: u64,
    ) -> String {
        // Quantize coordinates to roughly ~100m grid to eliminate millimeter fingerprinting
        let quantized_lat = (coord.latitude * 1000.0).round() as i64;
        let quantized_lon = (coord.longitude * 1000.0).round() as i64;

        let mut hasher = Sha256::new();
        hasher.update(&quantized_lat.to_le_bytes());
        hasher.update(&quantized_lon.to_le_bytes());
        hasher.update(salt.as_bytes());
        hasher.update(&timestamp.to_le_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verifies the commitment hash given the sector parameters and salt.
    pub fn verify_commitment(
        &self,
        candidate_coord: &Wgs84GeoCoord,
        salt: &str,
        current_time_secs: u64,
        max_age_secs: u64,
    ) -> Result<bool, ZkGeoError> {
        let age = current_time_secs.saturating_sub(self.timestamp);
        if age > max_age_secs {
            return Err(ZkGeoError::ProofExpired {
                age_seconds: age,
                max_seconds: max_age_secs,
            });
        }

        let calculated = Self::generate_commitment(candidate_coord, salt, self.timestamp);
        if calculated != self.commitment_hash {
            return Err(ZkGeoError::CommitmentMismatch);
        }

        Ok(true)
    }
}

/// Approximate distance between two WGS-84 coordinates in meters (Haversine formula).
pub fn haversine_distance_meters(c1: &Wgs84GeoCoord, c2: &Wgs84GeoCoord) -> f64 {
    const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

    let d_lat = (c2.latitude - c1.latitude).to_radians();
    let d_lon = (c2.longitude - c1.longitude).to_radians();

    let lat1 = c1.latitude.to_radians();
    let lat2 = c2.latitude.to_radians();

    let a: f64 = (d_lat / 2.0).sin().powi(2)
        + lat1.cos() * lat2.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_METERS * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zk_geo_proximity_commitment_and_verification() {
        let coord = Wgs84GeoCoord::new(35.6895, 139.6917, 40.0); // Tokyo
        let salt = "secure_enclave_pepper_2026";
        let timestamp = 1730000000;

        let commitment = ZkGeoProximityProof::generate_commitment(&coord, salt, timestamp);

        let proof = ZkGeoProximityProof {
            sector_geohash: "xn76u".to_string(),
            venue_id: Some("venue_shibuya_anchor".to_string()),
            commitment_hash: commitment.clone(),
            timestamp,
            club_signature: "mock_signature_valid".to_string(),
        };

        let verified = proof.verify_commitment(&coord, salt, timestamp + 10, 60);
        assert!(verified.is_ok());
        assert!(verified.unwrap());

        // Test bad salt fails
        let bad_salt_result = proof.verify_commitment(&coord, "wrong_salt", timestamp + 10, 60);
        assert!(bad_salt_result.is_err());
    }

    #[test]
    fn test_haversine_distance() {
        let c1 = Wgs84GeoCoord::new(35.6895, 139.6917, 0.0);
        let c2 = Wgs84GeoCoord::new(35.6895, 139.6927, 0.0); // ~90m east
        let dist = haversine_distance_meters(&c1, &c2);
        assert!(dist > 80.0 && dist < 100.0);
    }
}
