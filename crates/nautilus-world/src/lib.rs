//! # Nautilus World Geodetic Engine
//!
//! Dual-coordinate precision geodesy and spatial anchoring for planet Earth:
//! - 64-bit (`f64`) Earth-Centered, Earth-Fixed (ECEF) coordinates.
//! - `LocalTangentFrame` (East-North-Up) bridging local games/arenas to the curved planetary globe.
//! - Jitter-free camera-relative `f32` offsets for 3D shaders.
//! - `AnchoredSpatialOverlay` for pinning venues, stadiums, and storefronts without drift.

use glam::{DMat3, DVec3, Vec3};
use serde::{Deserialize, Serialize};

pub const WGS84_SEMI_MAJOR_AXIS_A: f64 = 6_378_137.0;
pub const WGS84_FLATTENING_F: f64 = 1.0 / 298.257_223_563;
pub const WGS84_ECCENTRICITY_SQ_E2: f64 = WGS84_FLATTENING_F * (2.0 - WGS84_FLATTENING_F);
pub const GAME_FIELD_ALTITUDE_THRESHOLD_METERS: f64 = 500.0;
pub const KARMAN_LINE_ORBIT_METERS: f64 = 80_000.0;

/// Geodetic position on the WGS84 ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Wgs84GeoCoord {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_meters: f64,
}

impl Wgs84GeoCoord {
    pub fn new(latitude: f64, longitude: f64, altitude_meters: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude_meters,
        }
    }

    /// Converts WGS84 (lat/lon/alt) to Earth-Centered, Earth-Fixed (ECEF) f64 vector.
    pub fn to_ecef(&self) -> DVec3 {
        let lat_rad = self.latitude.to_radians();
        let lon_rad = self.longitude.to_radians();

        let sin_lat = lat_rad.sin();
        let cos_lat = lat_rad.cos();
        let sin_lon = lon_rad.sin();
        let cos_lon = lon_rad.cos();

        let n = WGS84_SEMI_MAJOR_AXIS_A / (1.0 - WGS84_ECCENTRICITY_SQ_E2 * sin_lat * sin_lat).sqrt();

        let x = (n + self.altitude_meters) * cos_lat * cos_lon;
        let y = (n + self.altitude_meters) * cos_lat * sin_lon;
        let z = (n * (1.0 - WGS84_ECCENTRICITY_SQ_E2) + self.altitude_meters) * sin_lat;

        DVec3::new(x, y, z)
    }

    /// Converts 64-bit ECEF coordinate back into WGS84 (Bowring method).
    pub fn from_ecef(ecef: DVec3) -> Self {
        let p = (ecef.x * ecef.x + ecef.y * ecef.y).sqrt();
        let b = WGS84_SEMI_MAJOR_AXIS_A * (1.0 - WGS84_FLATTENING_F);
        let ep_sq = (WGS84_SEMI_MAJOR_AXIS_A * WGS84_SEMI_MAJOR_AXIS_A - b * b) / (b * b);

        let theta = (ecef.z * WGS84_SEMI_MAJOR_AXIS_A).atan2(p * b);
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        let lat_rad = (ecef.z + ep_sq * b * sin_theta * sin_theta * sin_theta).atan2(
            p - WGS84_ECCENTRICITY_SQ_E2 * WGS84_SEMI_MAJOR_AXIS_A * cos_theta * cos_theta * cos_theta,
        );
        let lon_rad = ecef.y.atan2(ecef.x);

        let sin_lat = lat_rad.sin();
        let n = WGS84_SEMI_MAJOR_AXIS_A / (1.0 - WGS84_ECCENTRICITY_SQ_E2 * sin_lat * sin_lat).sqrt();
        let alt = p / lat_rad.cos() - n;

        Self {
            latitude: lat_rad.to_degrees(),
            longitude: lon_rad.to_degrees(),
            altitude_meters: alt,
        }
    }
}

/// Converts a 64-bit ECEF world coordinate into a jitter-free 32-bit camera-relative shader coordinate.
#[inline]
pub fn ecef_to_camera_relative_f32(object_ecef: DVec3, camera_ecef: DVec3) -> Vec3 {
    let delta = object_ecef - camera_ecef;
    Vec3::new(delta.x as f32, delta.y as f32, delta.z as f32)
}

/// Local Tangent Frame: Anchors local East-North-Up (ENU) simulation to an authoritative Earth origin.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocalTangentFrame {
    pub origin_geo: Wgs84GeoCoord,
    pub origin_ecef: DVec3,
    pub enu_rotation: DMat3,
}

impl LocalTangentFrame {
    pub fn from_origin(origin_geo: Wgs84GeoCoord) -> Self {
        let origin_ecef = origin_geo.to_ecef();
        let lat_rad = origin_geo.latitude.to_radians();
        let lon_rad = origin_geo.longitude.to_radians();

        let sin_lat = lat_rad.sin();
        let cos_lat = lat_rad.cos();
        let sin_lon = lon_rad.sin();
        let cos_lon = lon_rad.cos();

        let east = DVec3::new(-sin_lon, cos_lon, 0.0);
        let north = DVec3::new(-sin_lat * cos_lon, -sin_lat * sin_lon, cos_lat);
        let up = DVec3::new(cos_lat * cos_lon, cos_lat * sin_lon, sin_lat);

        let enu_rotation = DMat3::from_cols(east, north, up);

        Self {
            origin_geo,
            origin_ecef,
            enu_rotation,
        }
    }

    /// Transforms local Cartesian (X=East, Y=North, Z=Up in meters) to global 64-bit ECEF.
    pub fn local_to_ecef(&self, local_enu: DVec3) -> DVec3 {
        self.origin_ecef + self.enu_rotation * local_enu
    }

    /// Transforms global 64-bit ECEF back to local ENU coordinates.
    pub fn ecef_to_local(&self, global_ecef: DVec3) -> DVec3 {
        let delta = global_ecef - self.origin_ecef;
        self.enu_rotation.transpose() * delta
    }
}

/// Synthetic game arena, 3D storefront, or generative world space anchored to Earth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnchoredSpatialOverlay {
    pub overlay_id: String,
    pub name: String,
    pub origin: Wgs84GeoCoord,
    pub bounding_radius_meters: f64,
}

impl AnchoredSpatialOverlay {
    pub fn new(id: &str, name: &str, origin: Wgs84GeoCoord, radius_meters: f64) -> Self {
        Self {
            overlay_id: id.to_string(),
            name: name.to_string(),
            origin,
            bounding_radius_meters: radius_meters,
        }
    }

    pub fn tangent_frame(&self) -> LocalTangentFrame {
        LocalTangentFrame::from_origin(self.origin)
    }
}
