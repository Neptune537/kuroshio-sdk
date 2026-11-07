//! # Native Far-Field Horizon Impostor & Landmark Billboards
//!
//! Renders distant mountain peaks, city skylines, and orbital beacons that reside
//! between 1.5km and 50km in a separate far-plane depth buffer, fading seamlessly
//! as the player transitions into high-resolution CDLOD terrain chunks.

use glam::{DVec3, Vec3};
use serde::{Deserialize, Serialize};
use crate::{ecef_to_camera_relative_f32, Wgs84GeoCoord};

/// Category of distant horizon landmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HorizonImpostorKind {
    MountainPeak,
    CitySkyline,
    OrbitalBeacon,
    NavigationalLighthouse,
}

/// A distant horizon landmark billboard or silhouette impostor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HorizonLandmarkImpostor {
    pub id: String,
    pub name: String,
    pub kind: HorizonImpostorKind,
    pub world_geo: Wgs84GeoCoord,
    pub world_ecef: DVec3,
    pub base_elevation_m: f64,
    pub height_meters: f32,
    pub width_meters: f32,
    /// Distance thresholds in meters:
    /// - `fade_in_distance_m`: Where the billboard starts becoming visible (e.g. 50km).
    /// - `fade_out_distance_m`: Where the billboard cross-fades into CDLOD mesh (e.g. 1.5km).
    pub max_visible_distance_m: f32,
    pub near_cdlod_transition_distance_m: f32,
}

impl HorizonLandmarkImpostor {
    pub fn new(
        id: &str,
        name: &str,
        kind: HorizonImpostorKind,
        world_geo: Wgs84GeoCoord,
        height_meters: f32,
        width_meters: f32,
    ) -> Self {
        let ecef = world_geo.to_ecef();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            kind,
            base_elevation_m: world_geo.altitude_meters,
            world_geo,
            world_ecef: ecef,
            height_meters,
            width_meters,
            max_visible_distance_m: 50_000.0, // 50 km default horizon reach
            near_cdlod_transition_distance_m: 1_500.0, // 1.5 km CDLOD transition
        }
    }

    /// Evaluates visibility and opacity factor for far-depth rendering.
    /// Returns `(is_visible, opacity_alpha [0.0, 1.0])`.
    pub fn compute_visibility_and_opacity(&self, camera_ecef: DVec3) -> (bool, f32) {
        let dist = self.world_ecef.distance(camera_ecef) as f32;

        if dist > self.max_visible_distance_m {
            return (false, 0.0);
        }

        // When closer than 1.5km, cross-fade out so CDLOD full geometry takes over
        if dist < self.near_cdlod_transition_distance_m * 0.5 {
            return (false, 0.0);
        }

        let opacity = if dist < self.near_cdlod_transition_distance_m {
            // Smoothly fade out between 1500m and 750m
            let range = self.near_cdlod_transition_distance_m * 0.5;
            let offset = dist - (self.near_cdlod_transition_distance_m * 0.5);
            (offset / range).clamp(0.0, 1.0)
        } else if dist > (self.max_visible_distance_m * 0.8) {
            // Atmospheric fog fade-in between 40km and 50km
            let fade_range = self.max_visible_distance_m * 0.2;
            let offset = self.max_visible_distance_m - dist;
            (offset / fade_range).clamp(0.0, 1.0)
        } else {
            1.0
        };

        (true, opacity)
    }

    /// Computes the camera-relative position vector for the far-depth billboard pass.
    #[inline]
    pub fn camera_relative_position(&self, camera_ecef: DVec3) -> Vec3 {
        ecef_to_camera_relative_f32(self.world_ecef, camera_ecef)
    }
}

/// Far-plane depth impostor manager for distant cities, mountains, and beacons (1.5km to 50km).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FarFieldHorizonManager {
    pub landmarks: Vec<HorizonLandmarkImpostor>,
}

impl FarFieldHorizonManager {
    pub fn new() -> Self {
        Self {
            landmarks: Vec::new(),
        }
    }

    pub fn add_landmark(&mut self, landmark: HorizonLandmarkImpostor) {
        self.landmarks.push(landmark);
    }

    /// Collects all active far-field landmarks that must be rendered in the far depth buffer pass.
    pub fn query_visible_impostors(
        &self,
        camera_ecef: DVec3,
    ) -> Vec<(&HorizonLandmarkImpostor, Vec3, f32)> {
        let mut visible = Vec::new();
        for lm in &self.landmarks {
            let (is_vis, opacity) = lm.compute_visibility_and_opacity(camera_ecef);
            if is_vis && opacity > 0.01 {
                let rel_pos = lm.camera_relative_position(camera_ecef);
                visible.push((lm, rel_pos, opacity));
            }
        }
        visible
    }
}

/// Travel mode dynamics with scaled velocities and kinetic parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TravelMode {
    /// Walking exploration: ~1.4 m/s (5.0 km/h)
    Walking,
    /// Mounted / Steed traversal: ~8.3 m/s to 11.1 m/s (30 - 40 km/h)
    MountedSteed,
    /// Fast tactical vehicle / hovercar: ~22.2 m/s to 33.3 m/s (80 - 120 km/h)
    TacticalVehicle,
    /// Atmospheric aerial sprint: ~150 m/s (540 km/h)
    AtmosphericFlight,
}

impl TravelMode {
    /// Maximum velocity in meters per second for this travel mode.
    pub fn max_speed_mps(&self) -> f32 {
        match self {
            TravelMode::Walking => 1.4,
            TravelMode::MountedSteed => 9.5,     // ~34.2 km/h
            TravelMode::TacticalVehicle => 27.8, // 100.0 km/h
            TravelMode::AtmosphericFlight => 150.0,
        }
    }

    /// Acceleration in meters per second squared.
    pub fn acceleration_mps2(&self) -> f32 {
        match self {
            TravelMode::Walking => 3.5,
            TravelMode::MountedSteed => 7.0,
            TravelMode::TacticalVehicle => 14.0,
            TravelMode::AtmosphericFlight => 35.0,
        }
    }
}
