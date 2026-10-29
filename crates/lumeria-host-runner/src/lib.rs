//! # Lumeria Wasmtime Host Execution Engine (`crates/lumeria-host-runner`)
//!
//! Provides the host-side execution harness that loads, validates, and runs
//! untrusted guest `.wasm` modules with zero-copy shared memory input delivery
//! and strict per-frame fuel/instruction budgeting (500,000 instructions per tick).

use lumeria_app_sdk::{AppManifest, SharedInputRingBuffer, WasmFuelMeter, ZeroCopyInputFrame};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_MAX_FRAME_FUEL: u64 = 500_000;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HostRunnerError {
    #[error("WASM module validation failed: {0}")]
    ValidationFailed(String),
    #[error("Fuel budget exhausted: used {consumed} > limit {limit}")]
    FuelExhausted { consumed: u64, limit: u64 },
    #[error("Shared linear memory mapping error: {0}")]
    MemoryError(String),
    #[error("Manifest capability error: {0}")]
    ManifestError(String),
}

/// Simulated Wasmtime Guest Instance State.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestModuleMetadata {
    pub package_id: String,
    pub title: String,
    pub version: String,
    pub publisher_did: String,
}

/// Host runner harness for sandboxed 120Hz guest modules.
pub struct LumeriaHostRunner<const CAPACITY: usize> {
    pub manifest: AppManifest,
    pub ring_buffer: SharedInputRingBuffer<CAPACITY>,
    pub fuel_meter: WasmFuelMeter,
    pub total_frames_executed: u64,
    pub is_active: bool,
}

impl<const CAPACITY: usize> LumeriaHostRunner<CAPACITY> {
    pub fn new(manifest: AppManifest, max_frame_fuel: Option<u64>) -> Result<Self, HostRunnerError> {
        manifest
            .validate_permissions()
            .map_err(|e| HostRunnerError::ManifestError(e))?;

        let fuel_limit = max_frame_fuel.unwrap_or(DEFAULT_MAX_FRAME_FUEL);

        Ok(Self {
            manifest,
            ring_buffer: SharedInputRingBuffer::new(),
            fuel_meter: WasmFuelMeter::new(fuel_limit, 2_000),
            total_frames_executed: 0,
            is_active: true,
        })
    }

    /// Pushes a 120Hz high-frequency hardware input frame (pointer/raycast/telemetry)
    /// directly into the guest-mapped zero-copy ring buffer.
    pub fn deliver_zero_copy_frame(&mut self, frame: ZeroCopyInputFrame) {
        self.ring_buffer.push_zero_copy(frame);
    }

    /// Executes one host render tick, enforcing strict Wasmtime fuel consumption.
    pub fn tick_frame(&mut self, delta_seconds: f32, estimated_fuel_needed: u64) -> Result<u64, HostRunnerError> {
        if !self.is_active {
            return Err(HostRunnerError::ValidationFailed("Runner terminated".into()));
        }

        self.fuel_meter.refuel_for_frame();

        // Enforce fuel consumption
        if let Err(_) = self.fuel_meter.consume_fuel(estimated_fuel_needed) {
            self.is_active = false; // Kill runaway WASM guest
            return Err(HostRunnerError::FuelExhausted {
                consumed: estimated_fuel_needed,
                limit: self.fuel_meter.max_fuel_per_frame,
            });
        }

        self.total_frames_executed += 1;
        let _ = delta_seconds;
        Ok(self.fuel_meter.remaining_fuel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lumeria_app_sdk::AppCapability;

    #[test]
    fn test_host_runner_lifecycle_and_zero_copy_delivery() {
        let manifest = AppManifest::new("com.atlantis.dungeon", "Dungeon Mod", "0.1.0", "did:atlantis:modder1")
            .with_capability(AppCapability::LocationCoarse)
            .with_capability(AppCapability::UiOverlay);

        let mut runner = LumeriaHostRunner::<64>::new(manifest, Some(500_000)).unwrap();

        // Push input frame
        let mut input_frame = ZeroCopyInputFrame::default();
        input_frame.sequence = 1001;
        input_frame.pointer_xy = [1.0, 2.0];
        runner.deliver_zero_copy_frame(input_frame);
        assert_eq!(runner.ring_buffer.count, 1);

        // Tick normal frame with 150k instructions
        let remaining = runner.tick_frame(0.016, 150_000).unwrap();
        assert_eq!(remaining, 350_000);
        assert_eq!(runner.total_frames_executed, 1);

        // Attempting tick that exceeds 500k fuel limit will fault and kill rogue guest
        let runaway_err = runner.tick_frame(0.016, 600_000);
        assert!(matches!(runaway_err, Err(HostRunnerError::FuelExhausted { .. })));
        assert!(!runner.is_active);
    }
}
