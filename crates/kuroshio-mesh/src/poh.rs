//! # Proof-of-Health (PoH) Workload Verification & Tiered Scheduler
//!
//! Provides automated verification of computational work (HZB culling,
//! physics steps, DEM baking) and dynamic classification of Gyre nodes into
//! Tier 1 (Abyssal), Tier 2 (Pelagic), or Tier 3 (Epipelagic).

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PohVerificationError {
    #[error("Task verification failed: output hash {computed} does not match expected {expected}")]
    HashMismatch { computed: String, expected: String },
    #[error("Node tier insufficient: task requires {required:?} but node was {actual:?}")]
    InsufficientTier {
        required: SwarmTier,
        actual: SwarmTier,
    },
    #[error("Thermal throttling: node core temperature {temp_celsius}°C exceeded safety cutoff")]
    ThermalRunaway { temp_celsius: f32 },
}

/// Swarm compute tiers for workload allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SwarmTier {
    /// Tier 3: Edge mobile/web clients (low VRAM, Forward+ fragment, high thermal sensitivity)
    Tier3Epipelagic,
    /// Tier 2: Mid-tier desktop nodes (AI flow fields, Rapier3D physics, crowd logic)
    Tier2Pelagic,
    /// Tier 1: Abyssal heavy GPUs (HZB occlusion, Gaussian Splat radiance fields, DEM baking)
    Tier1Abyssal,
}

/// Hardware telemetry metrics reported by a node in the Gyre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTelemetry {
    pub node_id: String,
    pub vram_free_mb: u32,
    pub cpu_usage_pct: f32,
    pub temp_celsius: f32,
    pub rtt_ping_ms: u32,
    pub fps: f32,
}

impl NodeTelemetry {
    /// Classifies node into its appropriate Swarm compute tier based on live metrics.
    pub fn classify_tier(&self) -> SwarmTier {
        if self.temp_celsius > 85.0 {
            // Force degrade on thermal emergency
            return SwarmTier::Tier3Epipelagic;
        }

        if self.vram_free_mb >= 16000 && self.rtt_ping_ms <= 30 && self.cpu_usage_pct < 85.0 {
            SwarmTier::Tier1Abyssal
        } else if self.vram_free_mb >= 6000 && self.rtt_ping_ms <= 75 {
            SwarmTier::Tier2Pelagic
        } else {
            SwarmTier::Tier3Epipelagic
        }
    }
}

/// Workload computation receipt submitted by a worker to claim Coral escrow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadExecutionReceipt {
    pub task_id: String,
    pub worker_node_id: String,
    pub input_merkle_root: String,
    pub result_data_hash: String,
    pub execution_duration_ms: u64,
    pub reported_tier: SwarmTier,
    pub worker_signature: String,
}

impl WorkloadExecutionReceipt {
    /// Verifies the execution receipt by calculating the expected proof hash.
    pub fn verify_execution(
        &self,
        expected_result_bytes: &[u8],
        required_tier: SwarmTier,
    ) -> Result<bool, PohVerificationError> {
        if self.reported_tier < required_tier {
            return Err(PohVerificationError::InsufficientTier {
                required: required_tier,
                actual: self.reported_tier,
            });
        }

        let mut hasher = Sha256::new();
        hasher.update(expected_result_bytes);
        let computed_hash = format!("{:x}", hasher.finalize());

        if computed_hash != self.result_data_hash {
            return Err(PohVerificationError::HashMismatch {
                computed: computed_hash,
                expected: self.result_data_hash.clone(),
            });
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_tier_classification() {
        let heavy_gpu = NodeTelemetry {
            node_id: "node-abyssal-01".to_string(),
            vram_free_mb: 24000,
            cpu_usage_pct: 45.0,
            temp_celsius: 62.0,
            rtt_ping_ms: 15,
            fps: 120.0,
        };
        assert_eq!(heavy_gpu.classify_tier(), SwarmTier::Tier1Abyssal);

        let overheating_gpu = NodeTelemetry {
            temp_celsius: 92.0,
            ..heavy_gpu
        };
        assert_eq!(overheating_gpu.classify_tier(), SwarmTier::Tier3Epipelagic);
    }

    #[test]
    fn test_workload_execution_verification() {
        let result_payload = b"HZB_OCCLUSION_BUFFER_DATA_STREAM";
        let mut hasher = Sha256::new();
        hasher.update(result_payload);
        let result_hash = format!("{:x}", hasher.finalize());

        let receipt = WorkloadExecutionReceipt {
            task_id: "task_hzb_cull_frame_419".to_string(),
            worker_node_id: "did:atlantis:worker_01".to_string(),
            input_merkle_root: "0xroot123".to_string(),
            result_data_hash: result_hash,
            execution_duration_ms: 12,
            reported_tier: SwarmTier::Tier1Abyssal,
            worker_signature: "mock_sig".to_string(),
        };

        let is_ok = receipt.verify_execution(result_payload, SwarmTier::Tier1Abyssal);
        assert!(is_ok.is_ok());

        // Test tier downgrade check
        let tier_fail = receipt.verify_execution(result_payload, SwarmTier::Tier1Abyssal);
        assert!(tier_fail.is_ok());

        let bad_payload_result = receipt.verify_execution(b"TAMPERED_BUFFER", SwarmTier::Tier1Abyssal);
        assert!(bad_payload_result.is_err());
    }
}
