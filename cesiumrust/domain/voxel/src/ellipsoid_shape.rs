//! Ellipsoid voxel shape implementation.
//!
//! Maps to CesiumJS `Scene/VoxelEllipsoidShape.js`.
//! Bounds are [longitude, latitude, height] with defaults [-π, -π/2, -1] to [π, π/2, 1].

use glam::{DMat3, DMat4, DVec3};

use crate::shape::{lerp, BoundingSphere, OrientedBoundingBox, VoxelShape};

/// Default minimum bounds: (-π, -π/2, -1).
pub const ELLIPSOID_DEFAULT_MIN_BOUNDS: DVec3 = DVec3::new(
    -std::f64::consts::PI,
    -std::f64::consts::FRAC_PI_2,
    -1.0,
);
/// Default maximum bounds: (π, π/2, 1).
pub const ELLIPSOID_DEFAULT_MAX_BOUNDS: DVec3 = DVec3::new(
    std::f64::consts::PI,
    std::f64::consts::FRAC_PI_2,
    1.0,
);

/// An ellipsoid-shaped voxel region.
///
/// Bounds are specified as (longitude, latitude, height) where:
/// - longitude: [-π, π]
/// - latitude: [-π/2, π/2]
/// - height: normalized height above/below ellipsoid surface
#[derive(Debug, Clone)]
pub struct VoxelEllipsoidShape {
    obb: OrientedBoundingBox,
    bounding_sphere: BoundingSphere,
    bound_transform: DMat4,
    shape_transform: DMat4,
    min_bounds: DVec3,
    max_bounds: DVec3,
    render_min_bounds: DVec3,
    render_max_bounds: DVec3,
    /// Ellipsoid radii (a, b, c).
    ellipsoid_radii: DVec3,
    /// UV scale: [longitude, latitude, height].
    local_to_shape_uv_scale: DVec3,
    /// UV translate: [longitude, latitude, height].
    local_to_shape_uv_translate: DVec3,
    /// Longitude range origin for UV mapping.
    shape_uv_longitude_range_origin: f64,
    max_intersections: u32,
}

impl Default for VoxelEllipsoidShape {
    fn default() -> Self {
        Self {
            obb: OrientedBoundingBox::default(),
            bounding_sphere: BoundingSphere::default(),
            bound_transform: DMat4::IDENTITY,
            shape_transform: DMat4::IDENTITY,
            min_bounds: ELLIPSOID_DEFAULT_MIN_BOUNDS,
            max_bounds: ELLIPSOID_DEFAULT_MAX_BOUNDS,
            render_min_bounds: ELLIPSOID_DEFAULT_MIN_BOUNDS,
            render_max_bounds: ELLIPSOID_DEFAULT_MAX_BOUNDS,
            ellipsoid_radii: DVec3::new(6378137.0, 6378137.0, 6_356_752.314_245_179),
            local_to_shape_uv_scale: DVec3::new(
                1.0 / std::f64::consts::TAU,
                1.0 / std::f64::consts::PI,
                0.5,
            ),
            local_to_shape_uv_translate: DVec3::new(0.5, 0.5, 0.5),
            shape_uv_longitude_range_origin: 0.0,
            max_intersections: 2,
        }
    }
}

impl VoxelEllipsoidShape {
    /// Create a new ellipsoid shape with default bounds and WGS84 radii.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom ellipsoid radii.
    pub fn with_radii(radii: DVec3) -> Self {
        Self {
            ellipsoid_radii: radii,
            ..Default::default()
        }
    }

    /// Get the minimum bounds (longitude, latitude, height).
    pub fn min_bounds(&self) -> DVec3 {
        self.min_bounds
    }

    /// Get the maximum bounds.
    pub fn max_bounds(&self) -> DVec3 {
        self.max_bounds
    }

    /// Get the render minimum bounds.
    pub fn render_min_bounds(&self) -> DVec3 {
        self.render_min_bounds
    }

    /// Get the render maximum bounds.
    pub fn render_max_bounds(&self) -> DVec3 {
        self.render_max_bounds
    }

    /// Get the ellipsoid radii.
    pub fn ellipsoid_radii(&self) -> DVec3 {
        self.ellipsoid_radii
    }

    /// Convert geodetic (lon, lat, height) to Cartesian on the ellipsoid.
    fn geodetic_to_cartesian(&self, lon: f64, lat: f64, height: f64) -> DVec3 {
        let radii = self.ellipsoid_radii;
        let cos_lat = lat.cos();
        let sin_lat = lat.sin();
        let cos_lon = lon.cos();
        let sin_lon = lon.sin();

        // Normal direction
        let n = DVec3::new(cos_lat * cos_lon, cos_lat * sin_lon, sin_lat);

        // Radii squared
        let r2 = DVec3::new(radii.x * radii.x, radii.y * radii.y, radii.z * radii.z);
        let n_r2 = DVec3::new(n.x / r2.x, n.y / r2.y, n.z / r2.z);
        let gamma = 1.0 / (n.x * n_r2.x + n.y * n_r2.y + n.z * n_r2.z).sqrt();

        let surface_point = DVec3::new(
            gamma * n.x / r2.x * radii.x * radii.x,
            gamma * n.y / r2.y * radii.y * radii.y,
            gamma * n.z / r2.z * radii.z * radii.z,
        );

        // Simplified: surface + height * normal
        surface_point + n * height
    }

    /// Compute OBB for a geodetic region.
    fn compute_chunk_obb(&self, min_b: DVec3, max_b: DVec3) -> OrientedBoundingBox {
        // Sample corners and midpoints to find bounding box
        let lon_min = min_b.x;
        let lon_max = max_b.x;
        let lat_min = min_b.y;
        let lat_max = max_b.y;
        let h_min = min_b.z;
        let h_max = max_b.z;

        let mut points = Vec::with_capacity(8);
        for &lon in &[lon_min, lon_max] {
            for &lat in &[lat_min, lat_max] {
                for &h in &[h_min, h_max] {
                    points.push(self.geodetic_to_cartesian(lon, lat, h));
                }
            }
        }
        // Add center point
        let lon_mid = (lon_min + lon_max) * 0.5;
        let lat_mid = (lat_min + lat_max) * 0.5;
        points.push(self.geodetic_to_cartesian(lon_mid, lat_mid, h_min));
        points.push(self.geodetic_to_cartesian(lon_mid, lat_mid, h_max));

        // Compute center
        let mut center = DVec3::ZERO;
        for p in &points {
            center += *p;
        }
        center /= points.len() as f64;

        // Compute max distance as radius
        let mut max_dist_sq = 0.0_f64;
        for p in &points {
            let d = (*p - center).length_squared();
            if d > max_dist_sq {
                max_dist_sq = d;
            }
        }
        let radius = max_dist_sq.sqrt();

        // Build OBB with identity orientation scaled to radius
        let half_axes = DMat3::from_cols(
            DVec3::new(radius, 0.0, 0.0),
            DVec3::new(0.0, radius, 0.0),
            DVec3::new(0.0, 0.0, radius),
        );
        OrientedBoundingBox::new(center, half_axes)
    }
}

impl VoxelShape for VoxelEllipsoidShape {
    fn oriented_bounding_box(&self) -> &OrientedBoundingBox {
        &self.obb
    }

    fn bounding_sphere(&self) -> &BoundingSphere {
        &self.bounding_sphere
    }

    fn bound_transform(&self) -> DMat4 {
        self.bound_transform
    }

    fn shape_transform(&self) -> DMat4 {
        self.shape_transform
    }

    fn maximum_intersections_length(&self) -> u32 {
        self.max_intersections
    }

    fn update(
        &mut self,
        model_matrix: DMat4,
        min_bounds: DVec3,
        max_bounds: DVec3,
        clip_min_bounds: Option<DVec3>,
        clip_max_bounds: Option<DVec3>,
    ) -> bool {
        let clip_min = clip_min_bounds.unwrap_or(min_bounds);
        let clip_max = clip_max_bounds.unwrap_or(max_bounds);

        self.min_bounds = min_bounds;
        self.max_bounds = max_bounds;

        let render_min = DVec3::new(
            min_bounds.x.max(clip_min.x),
            min_bounds.y.max(clip_min.y),
            min_bounds.z.max(clip_min.z),
        );
        let render_max = DVec3::new(
            max_bounds.x.min(clip_max.x),
            max_bounds.y.min(clip_max.y),
            max_bounds.z.min(clip_max.z),
        );
        self.render_min_bounds = render_min;
        self.render_max_bounds = render_max;

        // Check visibility
        if render_min.x > render_max.x
            || render_min.y > render_max.y
            || render_min.z > render_max.z
        {
            return false;
        }

        self.shape_transform = model_matrix;
        self.obb = self.compute_chunk_obb(render_min, render_max);
        self.bounding_sphere = BoundingSphere::from_obb(&self.obb);

        self.bound_transform = DMat4::from_cols(
            self.obb.half_axes.col(0).extend(0.0),
            self.obb.half_axes.col(1).extend(0.0),
            self.obb.half_axes.col(2).extend(0.0),
            self.obb.center.extend(1.0),
        );

        // Compute UV transforms
        let lon_range = max_bounds.x - min_bounds.x;
        let lat_range = max_bounds.y - min_bounds.y;
        let height_range = max_bounds.z - min_bounds.z;

        let lon_scale = if lon_range.abs() > 1e-10 { 1.0 / lon_range } else { 0.0 };
        let lat_scale = if lat_range.abs() > 1e-10 { 1.0 / lat_range } else { 0.0 };
        let height_scale = if height_range.abs() > 1e-10 { 1.0 / height_range } else { 0.0 };

        self.local_to_shape_uv_scale = DVec3::new(lon_scale, lat_scale, height_scale);
        self.local_to_shape_uv_translate = DVec3::new(
            -min_bounds.x * lon_scale,
            -min_bounds.y * lat_scale,
            -min_bounds.z * height_scale,
        );

        // Longitude range origin
        let default_lon_range = std::f64::consts::TAU;
        let uv_max_lon = (max_bounds.x - ELLIPSOID_DEFAULT_MIN_BOUNDS.x) / default_lon_range;
        let uv_lon_range_zero = 1.0 - lon_range / default_lon_range;
        self.shape_uv_longitude_range_origin = (uv_max_lon + 0.5 * uv_lon_range_zero) % 1.0;

        // Compute intersection count
        let mut count = 2u32; // height min + max
        let epsilon = 1e-10;
        let half_lon_range = default_lon_range * 0.5;
        if lon_range < default_lon_range - epsilon {
            if lon_range >= half_lon_range - epsilon {
                count += 1;
            } else if lon_range > epsilon {
                count += 2;
            }
        }
        if lat_range < std::f64::consts::PI - epsilon {
            count += 1; // latitude bound
        }
        self.max_intersections = count;

        true
    }

    fn convert_local_to_shape_uv_space(&self, position_local: DVec3) -> DVec3 {
        self.local_to_shape_uv_scale * position_local + self.local_to_shape_uv_translate
    }

    fn compute_obb_for_tile(
        &self,
        tile_level: u32,
        tile_x: u32,
        tile_y: u32,
        tile_z: u32,
    ) -> OrientedBoundingBox {
        let size_at_level = 1.0 / (2.0_f64.powi(tile_level as i32));
        let min_b = self.min_bounds;
        let max_b = self.max_bounds;

        let tile_min = DVec3::new(
            lerp(min_b.x, max_b.x, tile_x as f64 * size_at_level),
            lerp(min_b.y, max_b.y, tile_y as f64 * size_at_level),
            lerp(min_b.z, max_b.z, tile_z as f64 * size_at_level),
        );
        let tile_max = DVec3::new(
            lerp(min_b.x, max_b.x, (tile_x + 1) as f64 * size_at_level),
            lerp(min_b.y, max_b.y, (tile_y + 1) as f64 * size_at_level),
            lerp(min_b.z, max_b.z, (tile_z + 1) as f64 * size_at_level),
        );

        self.compute_chunk_obb(tile_min, tile_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ellipsoid_shape_default() {
        let shape = VoxelEllipsoidShape::new();
        assert_eq!(shape.min_bounds(), ELLIPSOID_DEFAULT_MIN_BOUNDS);
        assert_eq!(shape.max_bounds(), ELLIPSOID_DEFAULT_MAX_BOUNDS);
        assert!((shape.ellipsoid_radii().x - 6378137.0).abs() < 1.0);
    }

    #[test]
    fn test_ellipsoid_shape_update_identity() {
        let mut shape = VoxelEllipsoidShape::new();
        let visible = shape.update(
            DMat4::IDENTITY,
            ELLIPSOID_DEFAULT_MIN_BOUNDS,
            ELLIPSOID_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(visible);
        assert!(shape.bounding_sphere.radius > 6000000.0);
    }

    #[test]
    fn test_ellipsoid_shape_invisible_clipped() {
        let mut shape = VoxelEllipsoidShape::new();
        // Clip to a region that doesn't overlap
        let visible = shape.update(
            DMat4::IDENTITY,
            ELLIPSOID_DEFAULT_MIN_BOUNDS,
            ELLIPSOID_DEFAULT_MAX_BOUNDS,
            Some(DVec3::new(5.0, 5.0, 5.0)),
            Some(DVec3::new(10.0, 10.0, 10.0)),
        );
        assert!(!visible);
    }

    #[test]
    fn test_ellipsoid_shape_uv_transform() {
        let mut shape = VoxelEllipsoidShape::new();
        shape.update(
            DMat4::IDENTITY,
            ELLIPSOID_DEFAULT_MIN_BOUNDS,
            ELLIPSOID_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        // Center of bounds should map to UV ~(0.5, 0.5, 0.5)
        let center = (ELLIPSOID_DEFAULT_MIN_BOUNDS + ELLIPSOID_DEFAULT_MAX_BOUNDS) * 0.5;
        let uv = shape.convert_local_to_shape_uv_space(center);
        assert!((uv.x - 0.5).abs() < 1e-10);
        assert!((uv.y - 0.5).abs() < 1e-10);
        assert!((uv.z - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_ellipsoid_shape_tile_obb() {
        let mut shape = VoxelEllipsoidShape::new();
        shape.update(
            DMat4::IDENTITY,
            ELLIPSOID_DEFAULT_MIN_BOUNDS,
            ELLIPSOID_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        let obb = shape.compute_obb_for_tile(0, 0, 0, 0);
        assert!(obb.bounding_sphere_radius() > 0.0);

        // Level 1 tiles should also have valid OBBs
        let obb_l1 = shape.compute_obb_for_tile(1, 0, 0, 0);
        assert!(obb_l1.bounding_sphere_radius() > 0.0);
        // Sub-tile center should be different from full tile center
        assert!((obb_l1.center - obb.center).length() > 1.0);
    }

    #[test]
    fn test_ellipsoid_shape_custom_radii() {
        let shape = VoxelEllipsoidShape::with_radii(DVec3::new(1.0, 1.0, 1.0));
        assert_eq!(shape.ellipsoid_radii(), DVec3::ONE);
    }

    #[test]
    fn test_ellipsoid_shape_partial_region() {
        let mut shape = VoxelEllipsoidShape::new();
        let min_b = DVec3::new(0.0, 0.0, 0.0);
        let max_b = DVec3::new(
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::FRAC_PI_4,
            0.5,
        );
        let visible = shape.update(DMat4::IDENTITY, min_b, max_b, None, None);
        assert!(visible);
        assert!(shape.maximum_intersections_length() >= 2);
    }

    #[test]
    fn test_geodetic_to_cartesian_equator() {
        let shape = VoxelEllipsoidShape::with_radii(DVec3::new(6378137.0, 6378137.0, 6378137.0));
        // At lon=0, lat=0, height=0, should be at (6378137, 0, 0)
        let p = shape.geodetic_to_cartesian(0.0, 0.0, 0.0);
        assert!((p.x - 6378137.0).abs() < 1.0);
        assert!(p.y.abs() < 1.0);
        assert!(p.z.abs() < 1.0);
    }
}
