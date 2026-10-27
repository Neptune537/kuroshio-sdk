//! # Lumeria App SDK
//!
//! Clean-room, public WebAssembly guest SDK for building sandboxed applications,
//! interactive spatial overlays, team dashboards, and 3D storefronts on Lemuria.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Declared capability permissions requested in `app.manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppCapability {
    /// Access to coarse/masked regional geohash (Never raw GPS).
    LocationCoarse,
    /// Permission to send encrypted peer-to-peer packets over Kuroshio mesh.
    NetworkP2pChat,
    /// Permission to overlay 2D/3D spatial UI HUD panels into the Lemuria viewport.
    UiOverlay,
    /// Read-only access to local merchant storefront catalog.
    StoreCatalogReadOnly,
    /// Authority to initiate micro-escrow cart checkout requests through ai_gate.
    StoreInitiateCheckout,
    /// Access to spatial audio environmental nodes.
    AudioSpatialStream,
    /// Access to vehicle telemetry (speed, RPM, tire slip).
    VehicleTelemetryRead,
}

/// Security tier granted to an application package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppSecurityTier {
    /// Sandboxed guest with strictly limited capabilities.
    SandboxedGuest,
    /// Club-verified application signed by a recognized Club Home Node DID.
    ClubVerified,
    /// Core sovereign module with full local node integration.
    CoreSovereign,
}

/// Declarative application manifest (`app.manifest.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppManifest {
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub publisher_did: String,
    pub capabilities: HashSet<AppCapability>,
    pub package_signature: String,
    pub security_tier: AppSecurityTier,
}

impl AppManifest {
    pub fn new(app_id: &str, name: &str, version: &str, publisher_did: &str) -> Self {
        Self {
            app_id: app_id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            publisher_did: publisher_did.to_string(),
            capabilities: HashSet::new(),
            package_signature: String::new(),
            security_tier: AppSecurityTier::SandboxedGuest,
        }
    }

    pub fn with_capability(mut self, cap: AppCapability) -> Self {
        self.capabilities.insert(cap);
        self
    }

    pub fn has_capability(&self, cap: &AppCapability) -> bool {
        self.capabilities.contains(cap)
    }

    pub fn validate_permissions(&self) -> Result<(), String> {
        if self.capabilities.contains(&AppCapability::StoreInitiateCheckout)
            && self.security_tier == AppSecurityTier::SandboxedGuest
        {
            return Err("Capability StoreInitiateCheckout requires ClubVerified or higher tier".into());
        }
        Ok(())
    }
}

/// Input event passed into the sandboxed LumeriaApp guest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LumeriaInputEvent {
    PointerDown { x: f32, y: f32 },
    RaycastHit { hit_point: [f32; 3], normal: [f32; 3] },
    ProximityTrigger { totem_id: String, distance_meters: f32 },
    ActionTrigger { action_id: String },
}

/// Output command emitted by the sandboxed LumeriaApp to the host runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LumeriaHostCommand {
    ShowNotification { title: String, message: String },
    PlaySpatialAudio { sound_id: String, volume: f32 },
    RequestEscrowCheckout { item_id: String, price_cyan: f32 },
    SendMeshMessage { recipient_did: String, payload: Vec<u8> },
}

/// Trait implemented by sandboxed WebAssembly mini-apps.
pub trait LumeriaApp {
    fn on_init(&mut self) -> Result<(), String>;
    fn on_update(&mut self, delta_seconds: f32) -> Vec<LumeriaHostCommand>;
    fn on_input(&mut self, event: LumeriaInputEvent) -> Vec<LumeriaHostCommand>;
}
