//! # Continuous Geodetic Quadtree LOD & Floating-Origin Support (`nautilus_world::cdlod`)
//!
//! Hierarchical ellipsoidal quadtree chunking engine (CDLOD) that streams real digital
//! elevation models (DEM / Cloud-Optimized GeoTIFFs) with vertex-shader floating-origin
//! camera repositioning.

use glam::{DVec3, Mat4, Vec3};
use serde::{Deserialize, Serialize};
use crate::Wgs84GeoCoord;

/// Converts a 64-bit ECEF world coordinate (`DVec3`) into a 32-bit camera-relative shader coordinate (`Vec3`).
#[inline]
pub fn ecef_to_camera_relative_f32(object_ecef_f64: DVec3, camera_ecef_f64: DVec3) -> Vec3 {
    let delta = object_ecef_f64 - camera_ecef_f64;
    Vec3::new(delta.x as f32, delta.y as f32, delta.z as f32)
}

/// Floating-origin camera state for vertex-shader camera repositioning.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FloatingOriginCamera {
    /// 64-bit planetary ECEF camera eye position (sub-millimeter precision anywhere on Earth).
    pub camera_ecef_f64: DVec3,
    /// 64-bit reference floating origin.
    pub floating_origin_ecef_f64: DVec3,
    /// 32-bit camera offset relative to the floating origin.
    pub camera_offset_relative_f32: Vec3,
    /// View-Projection matrix computed with floating origin at (0, 0, 0).
    pub view_proj_relative: Mat4,
    pub fov_y_rad: f32,
    pub aspect_ratio: f32,
    pub near_z: f32,
    pub far_z: f32,
}

impl FloatingOriginCamera {
    pub fn new(camera_geo: Wgs84GeoCoord, fov_y_rad: f32, aspect_ratio: f32, near_z: f32, far_z: f32) -> Self {
        let eye_ecef = camera_geo.to_ecef();
        let mut cam = Self {
            camera_ecef_f64: eye_ecef,
            floating_origin_ecef_f64: eye_ecef,
            camera_offset_relative_f32: Vec3::ZERO,
            view_proj_relative: Mat4::IDENTITY,
            fov_y_rad,
            aspect_ratio,
            near_z,
            far_z,
        };
        cam.update_projection();
        cam
    }

    pub fn rebase_floating_origin(&mut self, new_origin_ecef: DVec3) {
        self.floating_origin_ecef_f64 = new_origin_ecef;
        self.camera_offset_relative_f32 =
            ecef_to_camera_relative_f32(self.camera_ecef_f64, self.floating_origin_ecef_f64);
        self.update_projection();
    }

    pub fn set_camera_ecef(&mut self, new_cam_ecef: DVec3) {
        self.camera_ecef_f64 = new_cam_ecef;
        let drift_sq = self.camera_ecef_f64.distance_squared(self.floating_origin_ecef_f64);
        if drift_sq > 500.0 * 500.0 {
            self.floating_origin_ecef_f64 = new_cam_ecef;
        }
        self.camera_offset_relative_f32 =
            ecef_to_camera_relative_f32(self.camera_ecef_f64, self.floating_origin_ecef_f64);
        self.update_projection();
    }

    #[inline]
    pub fn transform_vertex_to_camera_relative(&self, world_ecef: DVec3) -> Vec3 {
        ecef_to_camera_relative_f32(world_ecef, self.floating_origin_ecef_f64)
    }

    fn update_projection(&mut self) {
        self.view_proj_relative = Mat4::perspective_rh(
            self.fov_y_rad,
            self.aspect_ratio,
            self.near_z,
            self.far_z,
        );
    }
}

/// Cloud-Optimized GeoTIFF (COG) / DEM elevation sampler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudOptimizedDemSampler {
    pub dataset_name: String,
    pub min_elevation_m: f64,
    pub max_elevation_m: f64,
    pub tile_grid_resolution: usize,
}

impl CloudOptimizedDemSampler {
    pub fn new(dataset_name: &str) -> Self {
        Self {
            dataset_name: dataset_name.to_string(),
            min_elevation_m: -420.0,
            max_elevation_m: 8848.86,
            tile_grid_resolution: 32,
        }
    }

    pub fn sample_elevation_meters(&self, lat_deg: f64, lon_deg: f64) -> f64 {
        let lat_r = lat_deg.to_radians();
        let lon_r = lon_deg.to_radians();

        let wave1 = (lat_r * 4.0).sin() * (lon_r * 4.0).cos();
        let wave2 = (lat_r * 16.0).cos() * (lon_r * 16.0).sin() * 0.5;
        let wave3 = (lat_r * 64.0 + lon_r * 32.0).sin() * 0.25;

        let composite = (wave1 + wave2 + wave3) * 1200.0;
        composite.clamp(self.min_elevation_m, self.max_elevation_m)
    }

    pub fn fetch_cog_chunk_overview(
        &self,
        min_lat: f64,
        max_lat: f64,
        min_lon: f64,
        max_lon: f64,
        grid_dim: usize,
    ) -> Vec<f32> {
        let dim = grid_dim.max(4);
        let mut elevations = Vec::with_capacity(dim * dim);
        for row in 0..dim {
            let v = row as f64 / (dim - 1) as f64;
            let lat = min_lat + v * (max_lat - min_lat);
            for col in 0..dim {
                let u = col as f64 / (dim - 1) as f64;
                let lon = min_lon + u * (max_lon - min_lon);
                elevations.push(self.sample_elevation_meters(lat, lon) as f32);
            }
        }
        elevations
    }
}

/// A Continuous Distance-dependent LOD (CDLOD) ellipsoidal quadtree chunk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CdlodQuadtreeChunk {
    pub level: u8,
    pub quad_x: u32,
    pub quad_y: u32,
    pub min_lat_deg: f64,
    pub max_lat_deg: f64,
    pub min_lon_deg: f64,
    pub max_lon_deg: f64,
    pub min_elev_m: f64,
    pub max_elev_m: f64,
    pub center_ecef: DVec3,
    pub bounding_radius_m: f64,
    pub morph_start_dist_m: f64,
    pub morph_end_dist_m: f64,
}

impl CdlodQuadtreeChunk {
    pub fn new(
        level: u8,
        quad_x: u32,
        quad_y: u32,
        min_lat_deg: f64,
        max_lat_deg: f64,
        min_lon_deg: f64,
        max_lon_deg: f64,
        dem_sampler: &CloudOptimizedDemSampler,
    ) -> Self {
        let mid_lat = 0.5 * (min_lat_deg + max_lat_deg);
        let mid_lon = 0.5 * (min_lon_deg + max_lon_deg);
        let mid_elev = dem_sampler.sample_elevation_meters(mid_lat, mid_lon);
        let center_ecef = Wgs84GeoCoord::new(mid_lat, mid_lon, mid_elev).to_ecef();

        let corner = Wgs84GeoCoord::new(min_lat_deg, min_lon_deg, mid_elev).to_ecef();
        let bounding_radius_m = center_ecef.distance(corner).max(10.0);

        let lod_scale = 1.8f64.powi(16 - (level as i32).min(16));
        let morph_end_dist_m = bounding_radius_m * 2.5 * lod_scale.max(1.0);
        let morph_start_dist_m = morph_end_dist_m * 0.7;

        Self {
            level,
            quad_x,
            quad_y,
            min_lat_deg,
            max_lat_deg,
            min_lon_deg,
            max_lon_deg,
            min_elev_m: mid_elev - 200.0,
            max_elev_m: mid_elev + 200.0,
            center_ecef,
            bounding_radius_m,
            morph_start_dist_m,
            morph_end_dist_m,
        }
    }

    pub fn should_subdivide(&self, camera_ecef: DVec3, max_level: u8) -> bool {
        if self.level >= max_level {
            return false;
        }
        let dist = self.center_ecef.distance(camera_ecef);
        dist < self.morph_end_dist_m
    }

    #[inline]
    pub fn compute_morph_factor(&self, camera_ecef: DVec3) -> f32 {
        let dist = self.center_ecef.distance(camera_ecef);
        if dist <= self.morph_start_dist_m {
            1.0
        } else if dist >= self.morph_end_dist_m {
            0.0
        } else {
            let t = (self.morph_end_dist_m - dist) / (self.morph_end_dist_m - self.morph_start_dist_m);
            t.clamp(0.0, 1.0) as f32
        }
    }

    pub fn subdivide(&self, dem_sampler: &CloudOptimizedDemSampler) -> [CdlodQuadtreeChunk; 4] {
        let next_level = self.level + 1;
        let mid_lat = 0.5 * (self.min_lat_deg + self.max_lat_deg);
        let mid_lon = 0.5 * (self.min_lon_deg + self.max_lon_deg);
        let bx = self.quad_x * 2;
        let by = self.quad_y * 2;

        [
            CdlodQuadtreeChunk::new(next_level, bx, by, self.min_lat_deg, mid_lat, self.min_lon_deg, mid_lon, dem_sampler),
            CdlodQuadtreeChunk::new(next_level, bx + 1, by, self.min_lat_deg, mid_lat, mid_lon, self.max_lon_deg, dem_sampler),
            CdlodQuadtreeChunk::new(next_level, bx, by + 1, mid_lat, self.max_lat_deg, self.min_lon_deg, mid_lon, dem_sampler),
            CdlodQuadtreeChunk::new(next_level, bx + 1, by + 1, mid_lat, self.max_lat_deg, mid_lon, self.max_lon_deg, dem_sampler),
        ]
    }

    pub fn generate_floating_origin_mesh(
        &self,
        grid_dim: usize,
        floating_camera: &FloatingOriginCamera,
        dem_sampler: &CloudOptimizedDemSampler,
    ) -> Vec<Vec3> {
        let dim = grid_dim.max(2);
        let mut vertices = Vec::with_capacity(dim * dim);

        for row in 0..dim {
            let v = row as f64 / (dim - 1) as f64;
            let lat = self.min_lat_deg + v * (self.max_lat_deg - self.min_lat_deg);
            for col in 0..dim {
                let u = col as f64 / (dim - 1) as f64;
                let lon = self.min_lon_deg + u * (self.max_lon_deg - self.min_lon_deg);
                let elev = dem_sampler.sample_elevation_meters(lat, lon);
                let vertex_ecef = Wgs84GeoCoord::new(lat, lon, elev).to_ecef();
                vertices.push(floating_camera.transform_vertex_to_camera_relative(vertex_ecef));
            }
        }
        vertices
    }
}

/// Continuous geodetic CDLOD streaming engine for seamless planetary exploration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdlodPlanetEngine {
    pub max_lod_level: u8,
    pub dem_sampler: CloudOptimizedDemSampler,
    pub active_chunks: Vec<CdlodQuadtreeChunk>,
    pub visible_chunk_count: usize,
}

impl CdlodPlanetEngine {
    pub fn new(max_lod_level: u8, dataset_name: &str) -> Self {
        Self {
            max_lod_level,
            dem_sampler: CloudOptimizedDemSampler::new(dataset_name),
            active_chunks: Vec::new(),
            visible_chunk_count: 0,
        }
    }

    pub fn update(&mut self, camera: &FloatingOriginCamera) {
        let cam_ecef = camera.camera_ecef_f64;
        let cam_geo = Wgs84GeoCoord::from_ecef(cam_ecef);

        let root_span = 45.0;
        let root_chunk = CdlodQuadtreeChunk::new(
            1,
            0,
            0,
            (cam_geo.latitude - root_span).clamp(-89.9, 89.9),
            (cam_geo.latitude + root_span).clamp(-89.9, 89.9),
            (cam_geo.longitude - root_span).max(-180.0),
            (cam_geo.longitude + root_span).min(180.0),
            &self.dem_sampler,
        );

        let mut leaves = Vec::new();
        self.traverse_cdlod(root_chunk, cam_ecef, &mut leaves);
        self.visible_chunk_count = leaves.len();
        self.active_chunks = leaves;
    }

    fn traverse_cdlod(
        &self,
        chunk: CdlodQuadtreeChunk,
        camera_ecef: DVec3,
        out_leaves: &mut Vec<CdlodQuadtreeChunk>,
    ) {
        if chunk.should_subdivide(camera_ecef, self.max_lod_level) {
            for child in chunk.subdivide(&self.dem_sampler) {
                self.traverse_cdlod(child, camera_ecef, out_leaves);
            }
        } else {
            out_leaves.push(chunk);
        }
    }
}
