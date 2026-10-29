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

// =========================================================================
// 1. PLANETARY SPATIAL CHUNKING & DISCRETE GLOBAL GRID INDEXING (H3 / S2)
// =========================================================================

/// Planetary Level-of-Detail (LOD) tier for continuous streaming from orbit to ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlanetaryLod {
    /// Orbital tier (altitude > 100km): Low-resolution planetary sphere & coarse atmospheric limb.
    Lod0Orbit = 0,
    /// Continental tier (altitude 25km - 100km): Regional coastline and macro elevation tiles.
    Lod4Continent = 4,
    /// Sector / Metropolitan tier (altitude 5km - 25km): City boundaries & major road splines.
    Lod8Sector = 8,
    /// Tactical Venue tier (altitude 500m - 5km): High-poly local terrain mesh & landmarks.
    Lod12Venue = 12,
    /// Local Turf tier (altitude 0m - 500m): Centimeter-precision Rapier3D collision heightfields.
    Lod18Turf = 18,
}

impl PlanetaryLod {
    /// Selects optimal LOD resolution from camera altitude above Mean Sea Level.
    pub fn from_altitude_meters(altitude_msl: f64) -> Self {
        if altitude_msl >= 100_000.0 {
            PlanetaryLod::Lod0Orbit
        } else if altitude_msl >= 25_000.0 {
            PlanetaryLod::Lod4Continent
        } else if altitude_msl >= 5_000.0 {
            PlanetaryLod::Lod8Sector
        } else if altitude_msl >= 500.0 {
            PlanetaryLod::Lod12Venue
        } else {
            PlanetaryLod::Lod18Turf
        }
    }
}

/// Discrete Global Grid System (DGGS) cell index (64-bit quantized Morton / S2 / H3 equivalent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DggsCellIndex(pub u64);

impl DggsCellIndex {
    /// Quantizes a WGS84 coordinate into a Discrete Global Grid Cell at the target resolution (0..=18).
    pub fn from_wgs84(coord: Wgs84GeoCoord, resolution: u8) -> Self {
        let res = resolution.min(18);
        // Normalize lat [-90, 90] -> [0, 1] and lon [-180, 180] -> [0, 1]
        let norm_lat = ((coord.latitude + 90.0) / 180.0).clamp(0.0, 1.0);
        let norm_lon = ((coord.longitude + 180.0) / 360.0).clamp(0.0, 1.0);

        let steps = 1u64 << (res as u64);
        let x = ((norm_lon * (steps as f64)).floor() as u64).min(steps - 1);
        let y = ((norm_lat * (steps as f64)).floor() as u64).min(steps - 1);

        // Interleave 2D Morton bits: (res: 6 bits) | (x_y interleaved: up to 36 bits)
        let mut interleaved = 0u64;
        for i in 0..res {
            let bit_x = (x >> i) & 1;
            let bit_y = (y >> i) & 1;
            interleaved |= (bit_x << (2 * i)) | (bit_y << (2 * i + 1));
        }

        let encoded = ((res as u64) << 58) | (interleaved & 0x03FF_FFFF_FFFF_FFFF);
        DggsCellIndex(encoded)
    }

    pub fn resolution(&self) -> u8 {
        ((self.0 >> 58) & 0x3F) as u8
    }

    /// Computes the 64-bit ECEF axis-aligned bounding box (min_ecef, max_ecef) for spatial culling.
    pub fn ecef_bounding_box(&self) -> (DVec3, DVec3) {
        let res = self.resolution();
        let steps = 1u64 << (res as u64);
        let interleaved = self.0 & 0x03FF_FFFF_FFFF_FFFF;

        let mut x = 0u64;
        let mut y = 0u64;
        for i in 0..res {
            x |= ((interleaved >> (2 * i)) & 1) << i;
            y |= ((interleaved >> (2 * i + 1)) & 1) << i;
        }

        let min_lon = (x as f64 / steps as f64) * 360.0 - 180.0;
        let max_lon = ((x + 1) as f64 / steps as f64) * 360.0 - 180.0;
        let min_lat = (y as f64 / steps as f64) * 180.0 - 90.0;
        let max_lat = ((y + 1) as f64 / steps as f64) * 180.0 - 90.0;

        let c1 = Wgs84GeoCoord::new(min_lat, min_lon, 0.0).to_ecef();
        let c2 = Wgs84GeoCoord::new(max_lat, max_lon, 1000.0).to_ecef();

        let min = DVec3::new(c1.x.min(c2.x), c1.y.min(c2.y), c1.z.min(c2.z));
        let max = DVec3::new(c1.x.max(c2.x), c1.y.max(c2.y), c1.z.max(c2.z));
        (min, max)
    }
}

/// Spatial Interest Management (SIM) filter for localized peer subscriptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialInterestManager {
    pub home_cell: DggsCellIndex,
    pub interest_radius_meters: f64,
    pub active_subscriptions: Vec<DggsCellIndex>,
}

impl SpatialInterestManager {
    pub fn new(player_pos: Wgs84GeoCoord, radius_meters: f64) -> Self {
        // Default to resolution 8 (~460m hex / quad cell radius)
        let home_cell = DggsCellIndex::from_wgs84(player_pos, 8);
        let mut sim = Self {
            home_cell,
            interest_radius_meters: radius_meters,
            active_subscriptions: vec![],
        };
        sim.recompute_interest(player_pos);
        sim
    }

    /// Recomputes 3x3 neighbor cells surrounding the observer for interest management.
    pub fn recompute_interest(&mut self, player_pos: Wgs84GeoCoord) {
        self.home_cell = DggsCellIndex::from_wgs84(player_pos, 8);
        let mut cells = Vec::with_capacity(9);
        cells.push(self.home_cell);

        // Approximate 8-way delta offsets in lat/lon
        let d_lat = (self.interest_radius_meters / 111_000.0).clamp(0.001, 1.0);
        let d_lon = (self.interest_radius_meters / (111_000.0 * (player_pos.latitude.to_radians().cos().abs().max(0.1)))).clamp(0.001, 1.0);

        for dy in [-1.0, 0.0, 1.0] {
            for dx in [-1.0, 0.0, 1.0] {
                if dx == 0.0 && dy == 0.0 {
                    continue;
                }
                let neighbor = Wgs84GeoCoord::new(
                    (player_pos.latitude + dy * d_lat).clamp(-90.0, 90.0),
                    (player_pos.longitude + dx * d_lon).clamp(-180.0, 180.0),
                    player_pos.altitude_meters,
                );
                let cell = DggsCellIndex::from_wgs84(neighbor, 8);
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
        }
        self.active_subscriptions = cells;
    }

    /// Evaluates if a remote entity's coordinate falls within this node's active SIM radius.
    pub fn should_receive_packet(&self, entity_coord: Wgs84GeoCoord) -> bool {
        let cell = DggsCellIndex::from_wgs84(entity_coord, 8);
        self.active_subscriptions.contains(&cell)
    }
}

// =========================================================================
// 5. PLANETARY DEM TERRAIN ELEVATION BAKING & RAPIER3D MESH GENERATOR
// =========================================================================

/// Format of the raw Digital Elevation Model (DEM) tile buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemTileFormat {
    /// Mapbox Terrain-RGB encoding (height = -10000 + ((R * 256 * 256 + G * 256 + B) * 0.1)).
    MapboxTerrainRgb,
    /// 16-bit signed integer SRTM elevation in meters (big-endian).
    Srtm16BitSigned,
    /// 32-bit floating-point GeoTIFF elevation raster.
    GeoTiffFloat32,
}

/// Decoded planetary heightfield raster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanetaryDemTile {
    pub width: usize,
    pub height: usize,
    pub elevations_meters: Vec<f32>,
    pub min_elevation: f32,
    pub max_elevation: f32,
}

impl PlanetaryDemTile {
    /// Decodes a raw DEM tile buffer according to its specified format.
    pub fn decode(raw_bytes: &[u8], width: usize, height: usize, format: DemTileFormat) -> Result<Self, String> {
        let total_samples = width * height;
        let mut elevations = Vec::with_capacity(total_samples);
        let mut min_el = f32::MAX;
        let mut max_el = f32::MIN;

        match format {
            DemTileFormat::MapboxTerrainRgb => {
                if raw_bytes.len() < total_samples * 3 {
                    return Err(format!("Insufficient bytes for Terrain-RGB: got {}, need {}", raw_bytes.len(), total_samples * 3));
                }
                for i in 0..total_samples {
                    let r = raw_bytes[i * 3] as f32;
                    let g = raw_bytes[i * 3 + 1] as f32;
                    let b = raw_bytes[i * 3 + 2] as f32;
                    // Standard Mapbox Terrain-RGB equation
                    let elevation = -10000.0 + ((r * 65536.0 + g * 256.0 + b) * 0.1);
                    min_el = min_el.min(elevation);
                    max_el = max_el.max(elevation);
                    elevations.push(elevation);
                }
            }
            DemTileFormat::Srtm16BitSigned => {
                if raw_bytes.len() < total_samples * 2 {
                    return Err(format!("Insufficient bytes for SRTM16: got {}, need {}", raw_bytes.len(), total_samples * 2));
                }
                for i in 0..total_samples {
                    let b1 = raw_bytes[i * 2] as u16;
                    let b2 = raw_bytes[i * 2 + 1] as u16;
                    let raw_val = ((b1 << 8) | b2) as i16;
                    let elevation = if raw_val == -32768 { 0.0 } else { raw_val as f32 };
                    min_el = min_el.min(elevation);
                    max_el = max_el.max(elevation);
                    elevations.push(elevation);
                }
            }
            DemTileFormat::GeoTiffFloat32 => {
                if raw_bytes.len() < total_samples * 4 {
                    return Err(format!("Insufficient bytes for Float32 GeoTIFF: got {}, need {}", raw_bytes.len(), total_samples * 4));
                }
                for i in 0..total_samples {
                    let chunk: [u8; 4] = raw_bytes[i * 4..i * 4 + 4].try_into().map_err(|e| format!("{:?}", e))?;
                    let elevation = f32::from_le_bytes(chunk);
                    min_el = min_el.min(elevation);
                    max_el = max_el.max(elevation);
                    elevations.push(elevation);
                }
            }
        }

        Ok(Self {
            width,
            height,
            elevations_meters: elevations,
            min_elevation: min_el,
            max_elevation: max_el,
        })
    }

    /// Bakes a 3D triangle mesh heightfield in LocalTangentFrame coordinates (meters)
    /// suitable for GPU rendering and Rapier3D `SharedShape::trimesh` collision hulls.
    pub fn bake_trimesh_heightfield(
        &self,
        grid_step_meters: f32,
    ) -> (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<[f32; 3]>) {
        let mut vertices = Vec::with_capacity(self.width * self.height);
        let mut normals = vec![[0.0f32, 0.0, 1.0]; self.width * self.height];
        let mut indices = Vec::with_capacity((self.width - 1) * (self.height - 1) * 2);

        let half_w = (self.width as f32 * grid_step_meters) / 2.0;
        let half_h = (self.height as f32 * grid_step_meters) / 2.0;

        // Generate vertices
        for y in 0..self.height {
            for x in 0..self.width {
                let px = (x as f32 * grid_step_meters) - half_w;
                let py = (y as f32 * grid_step_meters) - half_h;
                let pz = self.elevations_meters[y * self.width + x];
                vertices.push([px, py, pz]);
            }
        }

        // Generate triangle indices (two triangles per quad)
        for y in 0..(self.height - 1) {
            for x in 0..(self.width - 1) {
                let top_left = (y * self.width + x) as u32;
                let top_right = top_left + 1;
                let bottom_left = ((y + 1) * self.width + x) as u32;
                let bottom_right = bottom_left + 1;

                indices.push([top_left, bottom_left, top_right]);
                indices.push([top_right, bottom_left, bottom_right]);
            }
        }

        // Compute smooth face normals
        for tri in &indices {
            let v0 = glam::Vec3::from(vertices[tri[0] as usize]);
            let v1 = glam::Vec3::from(vertices[tri[1] as usize]);
            let v2 = glam::Vec3::from(vertices[tri[2] as usize]);
            let n = (v1 - v0).cross(v2 - v0).normalize_or_zero();

            for &idx in tri {
                let current = glam::Vec3::from(normals[idx as usize]);
                let updated = (current + n).normalize_or_zero();
                normals[idx as usize] = [updated.x, updated.y, updated.z];
            }
        }

        (vertices, indices, normals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dggs_chunking_and_spatial_interest_management() {
        let tokyo = Wgs84GeoCoord::new(35.6586, 139.7454, 25.0);
        let cell = DggsCellIndex::from_wgs84(tokyo, 8);
        assert_eq!(cell.resolution(), 8);

        let (min_box, max_box) = cell.ecef_bounding_box();
        assert!(min_box.length() > 6_000_000.0);
        assert!(max_box.length() > 6_000_000.0);

        let sim = SpatialInterestManager::new(tokyo, 500.0);
        assert!(sim.active_subscriptions.contains(&cell));

        // Point 100m away should be accepted
        let nearby = Wgs84GeoCoord::new(35.6590, 139.7460, 25.0);
        assert!(sim.should_receive_packet(nearby));

        // Point on opposite side of globe should be culled
        let distant = Wgs84GeoCoord::new(-33.8688, 151.2093, 10.0); // Sydney
        assert!(!sim.should_receive_packet(distant));
    }

    #[test]
    fn test_planetary_lod_selection() {
        assert_eq!(PlanetaryLod::from_altitude_meters(150_000.0), PlanetaryLod::Lod0Orbit);
        assert_eq!(PlanetaryLod::from_altitude_meters(40_000.0), PlanetaryLod::Lod4Continent);
        assert_eq!(PlanetaryLod::from_altitude_meters(8_000.0), PlanetaryLod::Lod8Sector);
        assert_eq!(PlanetaryLod::from_altitude_meters(1_200.0), PlanetaryLod::Lod12Venue);
        assert_eq!(PlanetaryLod::from_altitude_meters(50.0), PlanetaryLod::Lod18Turf);
    }

    #[test]
    fn test_planetary_dem_baking_mapbox_terrain_rgb() {
        // 2x2 grid with synthetic heights
        let mut raw_rgb = Vec::new();
        for _ in 0..4 {
            // Encode ~500m elevation
            let val = ((500.0 + 10000.0) / 0.1) as u32;
            let r = ((val >> 16) & 0xFF) as u8;
            let g = ((val >> 8) & 0xFF) as u8;
            let b = (val & 0xFF) as u8;
            raw_rgb.extend_from_slice(&[r, g, b]);
        }

        let dem = PlanetaryDemTile::decode(&raw_rgb, 2, 2, DemTileFormat::MapboxTerrainRgb).unwrap();
        assert_eq!(dem.elevations_meters.len(), 4);
        assert!((dem.elevations_meters[0] - 500.0).abs() < 1.0);

        let (verts, indices, normals) = dem.bake_trimesh_heightfield(10.0);
        assert_eq!(verts.len(), 4);
        assert_eq!(indices.len(), 2);
        assert_eq!(normals.len(), 4);
    }

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
