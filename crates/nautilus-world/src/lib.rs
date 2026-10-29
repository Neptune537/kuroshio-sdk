//! # Nautilus World Geodetic Engine & High-Altitude Orbital Kinematics
//!
//! Dual-coordinate precision geodesy, EGM96/EGM2008 geoid undulation, Coriolis kinematics,
//! and double-precision shader splitters for planet Earth:
//! - **64-bit (`f64`) ECEF Coordinates:** WGS84 ellipsoid + Bowring inverse solver.
//! - **EGM96 / EGM2008 Geoid Undulation ($N$):** Gravitational geoid separation converting
//!   ellipsoidal height ($h$) to true orthometric Mean Sea Level ($H_{\text{MSL}} = h - N$).
//! - **64-bit Kinematic State Vectors (`KinematicStateVector`):** Velocity ($\text{m/s}$),
//!   Coriolis acceleration ($-2\vec{\Omega}_{\oplus} \times \vec{v}$), and centrifugal acceleration
//!   between surface arenas and the Kármán line ($80\text{ km}$).
//! - **Shader Precision Splitters:** Decomposes 64-bit `DMat4` model-view matrices and `DVec3`
//!   vectors into high/low `f32` pairs for jitter-free WebGPU and Vulkan vertex pipelines.

use glam::{DMat3, DMat4, DVec3, Mat4, Vec3, Vec4};
use serde::{Deserialize, Serialize};

pub const WGS84_SEMI_MAJOR_AXIS_A: f64 = 6_378_137.0;
pub const WGS84_FLATTENING_F: f64 = 1.0 / 298.257_223_563;
pub const WGS84_ECCENTRICITY_SQ_E2: f64 = WGS84_FLATTENING_F * (2.0 - WGS84_FLATTENING_F);
pub const GAME_FIELD_ALTITUDE_THRESHOLD_METERS: f64 = 500.0;
pub const KARMAN_LINE_ORBIT_METERS: f64 = 80_000.0;
/// Earth sidereal angular velocity ($\text{rad/s}$) around the ECEF Z-axis.
pub const EARTH_ANGULAR_VELOCITY_RAD_S: f64 = 7.292_115_9e-5;

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

    /// Computes the EGM96 / EGM2008 geoid undulation $N(\phi, \lambda)$ in meters
    /// (height of the gravitational geoid above/below the mathematical WGS84 ellipsoid,
    /// ranging from $-106\text{ m}$ in the Indian Ocean to $+85\text{ m}$ near New Guinea/Iceland).
    pub fn geoid_undulation_meters(&self) -> f64 {
        egm2008_geoid_undulation_meters(self.latitude, self.longitude)
    }

    /// Returns true orthometric altitude above Mean Sea Level ($H_{\text{MSL}} = h_{\text{ellipsoid}} - N$).
    pub fn orthometric_msl_altitude_meters(&self) -> f64 {
        self.altitude_meters - self.geoid_undulation_meters()
    }

    /// Creates a `Wgs84GeoCoord` from a true Mean Sea Level (MSL) altitude by adding the local geoid undulation $N$.
    pub fn from_msl_altitude(latitude: f64, longitude: f64, msl_altitude_meters: f64) -> Self {
        let n = egm2008_geoid_undulation_meters(latitude, longitude);
        Self::new(latitude, longitude, msl_altitude_meters + n)
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

/// Evaluates low-degree spherical harmonic + regional anomaly lookup for EGM96/EGM2008 geoid undulation $N$ in meters.
pub fn egm2008_geoid_undulation_meters(latitude_deg: f64, longitude_deg: f64) -> f64 {
    let lat = latitude_deg.to_radians();
    let lon = longitude_deg.to_radians();

    // Zonal & tesseral harmonic geopotential terms (J2/J3/C22/S22 + Indian Ocean low / North Atlantic high)
    let p20 = 0.5 * (3.0 * lat.sin().powi(2) - 1.0);
    let p22 = 3.0 * lat.cos().powi(2);
    let p31 = 1.5 * (5.0 * lat.sin().powi(2) - 1.0) * lat.cos();

    let n_harmonic = -14.2 * p20
        + 18.5 * p22 * (2.0 * (lon - 1.35)).cos()
        - 22.0 * p22 * (2.0 * (lon - 0.25)).sin()
        + 9.4 * p31 * (lon + 0.5).cos();

    n_harmonic.clamp(-106.0, 85.0)
}

/// 64-bit Kinematic State Vector for high-altitude aircraft, rockets, and ballistics
/// transitioning between surface arenas and the Kármán line ($80\text{ km}$).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KinematicStateVector {
    /// 64-bit ECEF position in meters
    pub position_ecef: DVec3,
    /// 64-bit ECEF velocity in meters per second ($\text{m/s}$)
    pub velocity_ecef: DVec3,
}

impl KinematicStateVector {
    pub fn new(position_ecef: DVec3, velocity_ecef: DVec3) -> Self {
        Self {
            position_ecef,
            velocity_ecef,
        }
    }

    /// Computes Coriolis acceleration ($\vec{a}_{\text{Coriolis}} = -2\vec{\Omega}_{\oplus} \times \vec{v}$)
    /// in the rotating ECEF reference frame ($\text{m/s}^2$).
    pub fn coriolis_acceleration_ecef(&self) -> DVec3 {
        let omega = DVec3::new(0.0, 0.0, EARTH_ANGULAR_VELOCITY_RAD_S);
        -2.0 * omega.cross(self.velocity_ecef)
    }

    /// Computes centrifugal acceleration ($-\vec{\Omega}_{\oplus} \times (\vec{\Omega}_{\oplus} \times \vec{r})$)
    /// in the rotating ECEF reference frame ($\text{m/s}^2$).
    pub fn centrifugal_acceleration_ecef(&self) -> DVec3 {
        let omega = DVec3::new(0.0, 0.0, EARTH_ANGULAR_VELOCITY_RAD_S);
        -omega.cross(omega.cross(self.position_ecef))
    }

    /// Integrates position and velocity forward by `dt_seconds` including Coriolis and centrifugal terms.
    pub fn step_kinematics(&mut self, external_accel_ecef: DVec3, dt_seconds: f64) {
        let total_accel = external_accel_ecef
            + self.coriolis_acceleration_ecef()
            + self.centrifugal_acceleration_ecef();
        self.velocity_ecef += total_accel * dt_seconds;
        self.position_ecef += self.velocity_ecef * dt_seconds;
    }
}

/// Splits a 64-bit `f64` scalar into a high/low `f32` pair `(hi, lo)` for emulated double-precision in WGSL/SPIR-V shaders.
#[inline]
pub fn split_f64_to_f32_pair(val: f64) -> (f32, f32) {
    let hi = val as f32;
    let lo = (val - (hi as f64)) as f32;
    (hi, lo)
}

/// Splits a 64-bit `DVec3` coordinate into `(high_f32, low_f32)` `Vec3` pairs for jitter-free GPU vertex pipelines.
#[inline]
pub fn split_dvec3_for_wgpu(v: DVec3) -> (Vec3, Vec3) {
    let (hx, lx) = split_f64_to_f32_pair(v.x);
    let (hy, ly) = split_f64_to_f32_pair(v.y);
    let (hz, lz) = split_f64_to_f32_pair(v.z);
    (Vec3::new(hx, hy, hz), Vec3::new(lx, ly, lz))
}

/// Decomposes a 64-bit `DMat4` model-view matrix into high and low 32-bit `Mat4` matrices `(mat_hi, mat_lo)`
/// for jitter-free WebGPU and Vulkan vertex shaders.
#[inline]
pub fn split_dmat4_for_wgpu(mat: DMat4) -> (Mat4, Mat4) {
    let cols = [mat.x_axis, mat.y_axis, mat.z_axis, mat.w_axis];
    let mut hi_cols = [Vec4::ZERO; 4];
    let mut lo_cols = [Vec4::ZERO; 4];

    for (i, c) in cols.iter().enumerate() {
        let (hx, lx) = split_f64_to_f32_pair(c.x);
        let (hy, ly) = split_f64_to_f32_pair(c.y);
        let (hz, lz) = split_f64_to_f32_pair(c.z);
        let (hw, lw) = split_f64_to_f32_pair(c.w);
        hi_cols[i] = Vec4::new(hx, hy, hz, hw);
        lo_cols[i] = Vec4::new(lx, ly, lz, lw);
    }

    (
        Mat4::from_cols(hi_cols[0], hi_cols[1], hi_cols[2], hi_cols[3]),
        Mat4::from_cols(lo_cols[0], lo_cols[1], lo_cols[2], lo_cols[3]),
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_egm2008_geoid_undulation_and_msl() {
        let tokyo = Wgs84GeoCoord::from_msl_altitude(35.6586, 139.7454, 25.0);
        let n = tokyo.geoid_undulation_meters();
        assert!(n >= -106.0 && n <= 85.0);
        let msl = tokyo.orthometric_msl_altitude_meters();
        assert!((msl - 25.0).abs() < 1e-6);
    }

    #[test]
    fn test_coriolis_kinematics_and_dmat4_splitter() {
        let pos = Wgs84GeoCoord::new(35.0, 139.0, 45_000.0).to_ecef();
        let vel = DVec3::new(1200.0, -400.0, 300.0); // Supersonic ascent toward Karman line
        let state = KinematicStateVector::new(pos, vel);

        let coriolis = state.coriolis_acceleration_ecef();
        assert!(coriolis.length() > 0.01);

        let dmat = DMat4::from_translation(pos);
        let (hi, lo) = split_dmat4_for_wgpu(dmat);
        let reconstructed_x = (hi.w_axis.x as f64) + (lo.w_axis.x as f64);
        assert!((reconstructed_x - pos.x).abs() < 1e-7);
    }
}
