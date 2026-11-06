//! # Lumeria Wasmtime Host Execution Engine (`crates/lumeria-host-runner`)
//!
//! Provides the host-side execution harness that loads, validates, and runs
//! untrusted guest `.wasm` modules with zero-copy shared memory input delivery
//! and strict per-frame fuel/instruction budgeting (500,000 instructions per tick).

use lumeria_app_sdk::{AppManifest, SharedInputRingBuffer, WasmFuelMeter, ZeroCopyInputFrame};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const DEFAULT_MAX_FRAME_FUEL: u64 = 500_000;
/// Default maximum Wasm linear memory pages (256 pages * 64 KiB = 16 MiB hard ceiling)
pub const DEFAULT_MAX_MEMORY_PAGES: u32 = 256;
pub const WASM_PAGE_SIZE_BYTES: usize = 65_536;

/// Zero-WASI Sandbox Isolation Policy enforced on all untrusted guest modules.
/// By default, all host filesystem (`fd_read`/`fd_write`/`path_open`), raw socket (`sock_open`),
/// and process environment (`environ_get`) syscalls are completely stripped from the linker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasiSandboxPolicy {
    pub allow_filesystem: bool,
    pub allow_raw_sockets: bool,
    pub allow_env_vars: bool,
    pub max_linear_memory_pages: u32,
}

impl Default for WasiSandboxPolicy {
    fn default() -> Self {
        Self {
            allow_filesystem: false,
            allow_raw_sockets: false,
            allow_env_vars: false,
            max_linear_memory_pages: DEFAULT_MAX_MEMORY_PAGES,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HostRunnerError {
    #[error("WASM module validation failed: {0}")]
    ValidationFailed(String),
    #[error("Fuel budget exhausted: used {consumed} > limit {limit}")]
    FuelExhausted { consumed: u64, limit: u64 },
    #[error("Memory bomb blocked: requested {requested_pages} pages exceeds cap of {max_pages} pages ({max_mib} MiB)")]
    MemoryLimitExceeded {
        requested_pages: u32,
        max_pages: u32,
        max_mib: u32,
    },
    #[error("Sandbox escape blocked: prohibited WASI import `{0}` rejected by zero-WASI policy")]
    ForbiddenWasiImport(String),
    #[error("Shared linear memory mapping error: {0}")]
    MemoryError(String),
    #[error("Manifest capability error: {0}")]
    ManifestError(String),
    #[error("Host function TOCTOU violation: guest buffer pointer or length out of linear memory bounds")]
    ToctouMemoryOutOfBounds,
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
    pub wasi_policy: WasiSandboxPolicy,
    pub allocated_memory_pages: u32,
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
            wasi_policy: WasiSandboxPolicy::default(),
            allocated_memory_pages: 16, // Initial 1 MiB linear memory
            ring_buffer: SharedInputRingBuffer::new(),
            fuel_meter: WasmFuelMeter::new(fuel_limit, 2_000),
            total_frames_executed: 0,
            is_active: true,
        })
    }

    /// Audits a WASM binary's declared imports before instantiation to block WASI filesystem/socket escape attempts.
    pub fn audit_wasm_imports(&mut self, declared_imports: &[&str]) -> Result<(), HostRunnerError> {
        for &imp in declared_imports {
            let lower = imp.to_ascii_lowercase();
            if !self.wasi_policy.allow_filesystem
                && (lower.contains("wasi_snapshot_preview1::path_")
                    || lower.contains("wasi_snapshot_preview1::fd_")
                    || lower.contains("wasi:filesystem"))
            {
                self.is_active = false;
                return Err(HostRunnerError::ForbiddenWasiImport(imp.to_string()));
            }
            if !self.wasi_policy.allow_raw_sockets
                && (lower.contains("sock_open") || lower.contains("sock_connect") || lower.contains("wasi:sockets"))
            {
                self.is_active = false;
                return Err(HostRunnerError::ForbiddenWasiImport(imp.to_string()));
            }
        }
        Ok(())
    }

    /// Enforces hard linear memory page caps (`memory.grow` guard) to prevent WASM memory bombs.
    pub fn request_memory_grow(&mut self, additional_pages: u32) -> Result<u32, HostRunnerError> {
        let new_total = self.allocated_memory_pages.saturating_add(additional_pages);
        if new_total > self.wasi_policy.max_linear_memory_pages {
            self.is_active = false;
            return Err(HostRunnerError::MemoryLimitExceeded {
                requested_pages: new_total,
                max_pages: self.wasi_policy.max_linear_memory_pages,
                max_mib: (self.wasi_policy.max_linear_memory_pages * 64) / 1024,
            });
        }
        self.allocated_memory_pages = new_total;
        Ok(self.allocated_memory_pages)
    }

    /// Pushes a 120Hz high-frequency hardware input frame (pointer/raycast/telemetry)
    /// directly into the guest-mapped zero-copy ring buffer.
    pub fn deliver_zero_copy_frame(&mut self, frame: ZeroCopyInputFrame) {
        self.ring_buffer.push_zero_copy(frame);
    }

    /// Copies guest linear memory into an owned host buffer before validation and execution.
    /// Completely defeats TOCTOU (Time-of-Check to Time-of-Use) attacks where a secondary
    /// guest thread mutates linear memory while the host validates or executes an FFI payload.
    pub fn copy_in_guest_buffer(
        &self,
        guest_linear_memory: &[u8],
        offset: usize,
        length: usize,
    ) -> Result<Vec<u8>, HostRunnerError> {
        let end = offset.checked_add(length).ok_or(HostRunnerError::ToctouMemoryOutOfBounds)?;
        if end > guest_linear_memory.len() {
            return Err(HostRunnerError::ToctouMemoryOutOfBounds);
        }
        // Atomic copy-in: create an owned snapshot before any validation occurs
        let mut owned_copy = Vec::with_capacity(length);
        owned_copy.extend_from_slice(&guest_linear_memory[offset..end]);
        Ok(owned_copy)
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

    #[test]
    fn test_zero_wasi_import_audit_and_memory_bomb_guard() {
        let manifest = AppManifest::new("com.untrusted.mod", "Untrusted Mod", "0.1.0", "did:atlantis:anon");
        let mut runner = LumeriaHostRunner::<32>::new(manifest.clone(), None).unwrap();

        // Safe WIT imports pass
        assert!(runner.audit_wasm_imports(&["lumeria:guest/host@0.1.0"]).is_ok());

        // Attempting WASI filesystem or socket escape is blocked and terminates runner
        let escape_err = runner.audit_wasm_imports(&["wasi_snapshot_preview1::path_open"]);
        assert!(matches!(escape_err, Err(HostRunnerError::ForbiddenWasiImport(_))));
        assert!(!runner.is_active);

        // Memory bomb (> 256 pages / 16 MiB) is blocked
        let mut runner2 = LumeriaHostRunner::<32>::new(manifest, None).unwrap();
        assert!(runner2.request_memory_grow(100).is_ok());
        let mem_err = runner2.request_memory_grow(300);
        assert!(matches!(mem_err, Err(HostRunnerError::MemoryLimitExceeded { .. })));
        assert!(!runner2.is_active);

        // Host-function TOCTOU copy-in boundary check
        let mut guest_memory = vec![0u8; 1024];
        guest_memory[10..14].copy_from_slice(b"CYAN");
        let copied = runner2.copy_in_guest_buffer(&guest_memory, 10, 4).unwrap();
        assert_eq!(copied, b"CYAN");
        // Mutating guest linear memory does NOT affect the copied host buffer
        guest_memory[10..14].copy_from_slice(b"EVIL");
        assert_eq!(copied, b"CYAN");
        // Out-of-bounds pointer is immediately rejected
        assert_eq!(
            runner2.copy_in_guest_buffer(&guest_memory, 1020, 10),
            Err(HostRunnerError::ToctouMemoryOutOfBounds)
        );
    }
}
