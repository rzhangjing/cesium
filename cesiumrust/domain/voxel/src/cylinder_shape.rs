//! Cylinder voxel shape implementation.
//!
//! Maps to CesiumJS `Scene/VoxelCylinderShape.js`.
//! Bounds are [radius, angle, height] with defaults [0, -π, -1] to [1, π, 1].

use glam::{DMat3, DMat4, DVec3};

use crate::shape::{lerp, BoundingSphere, OrientedBoundingBox, VoxelShape};

/// Default minimum bounds for cylinder shape: (0, -π, -1).
pub const CYLINDER_DEFAULT_MIN_BOUNDS: DVec3 =
    DVec3::new(0.0, -std::f64::consts::PI, -1.0);
/// Default maximum bounds for cylinder shape: (1, π, 1).
pub const CYLINDER_DEFAULT_MAX_BOUNDS: DVec3 =
    DVec3::new(1.0, std::f64::consts::PI, 1.0);

/// A cylinder-shaped voxel region.
///
/// Bounds are specified as (radius, angle, height) where:
/// - radius: [0, ∞)
/// - angle: [-π, π]
/// - height: (-∞, ∞)
#[derive(Debug, Clone)]
pub struct VoxelCylinderShape {
    obb: OrientedBoundingBox,
    bounding_sphere: BoundingSphere,
    bound_transform: DMat4,
    shape_transform: DMat4,
    min_bounds: DVec3,
    max_bounds: DVec3,
    render_min_bounds: DVec3,
    render_max_bounds: DVec3,
    /// UV scale: [radial, angle, height].
    local_to_shape_uv_scale: DVec3,
    /// UV translate: [radial, angle, height].
    local_to_shape_uv_translate: DVec3,
    /// Angle range origin for UV mapping.
    shape_uv_angle_range_origin: f64,
    max_intersections: u32,
}

impl Default for VoxelCylinderShape {
    fn default() -> Self {
        Self {
            obb: OrientedBoundingBox::default(),
            bounding_sphere: BoundingSphere::default(),
            bound_transform: DMat4::IDENTITY,
            shape_transform: DMat4::IDENTITY,
            min_bounds: CYLINDER_DEFAULT_MIN_BOUNDS,
            max_bounds: CYLINDER_DEFAULT_MAX_BOUNDS,
            render_min_bounds: CYLINDER_DEFAULT_MIN_BOUNDS,
            render_max_bounds: CYLINDER_DEFAULT_MAX_BOUNDS,
            local_to_shape_uv_scale: DVec3::ONE,
            local_to_shape_uv_translate: DVec3::ZERO,
            shape_uv_angle_range_origin: 0.0,
            max_intersections: 2,
        }
    }
}

impl VoxelCylinderShape {
    /// Create a new cylinder shape with default bounds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the minimum bounds (radius, angle, height).
    pub fn min_bounds(&self) -> DVec3 {
        self.min_bounds
    }

    /// Get the maximum bounds (radius, angle, height).
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

    /// Compute the OBB for a cylinder chunk.
    fn compute_chunk_obb(&self, min_b: DVec3, max_b: DVec3) -> OrientedBoundingBox {
        let radius_start = min_b.x;
        let radius_end = max_b.x;
        let angle_start = min_b.y;
        let angle_end = if max_b.y < angle_start {
            max_b.y + std::f64::consts::TAU
        } else {
            max_b.y
        };
        let height_start = min_b.z;
        let height_end = max_b.z;

        let angle_range = angle_end - angle_start;
        let angle_mid = angle_start + angle_range * 0.5;

        // Test angles for bounding box computation
        let mut test_angles = vec![angle_start, angle_end, angle_mid];
        if angle_range > std::f64::consts::PI {
            test_angles.push(angle_mid - std::f64::consts::FRAC_PI_2);
            test_angles.push(angle_mid + std::f64::consts::FRAC_PI_2);
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for &angle in &test_angles {
            let rel_angle = angle - angle_mid;
            let cos_a = rel_angle.cos();
            let sin_a = rel_angle.sin();
            let x1 = cos_a * radius_start;
            let y1 = sin_a * radius_start;
            let x2 = cos_a * radius_end;
            let y2 = sin_a * radius_end;

            min_x = min_x.min(x1).min(x2);
            min_y = min_y.min(y1).min(y2);
            max_x = max_x.max(x1).max(x2);
            max_y = max_y.max(y1).max(y2);
        }

        let extent_x = max_x - min_x;
        let extent_y = max_y - min_y;
        let extent_z = height_end - height_start;

        let center_local = DVec3::new(
            (min_x + max_x) * 0.5,
            (min_y + max_y) * 0.5,
            (height_start + height_end) * 0.5,
        );

        // Rotation around Z by angle_mid
        let cos_mid = angle_mid.cos();
        let sin_mid = angle_mid.sin();
        let rotation = DMat3::from_cols(
            DVec3::new(cos_mid, sin_mid, 0.0),
            DVec3::new(-sin_mid, cos_mid, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        );

        // CesiumJS algorithm: localMatrix = R(angleMid) * T(center) * S(extent)
        // globalMatrix = modelMatrix * localMatrix
        // OBB center = translation of globalMatrix
        // OBB halfAxes = upper-left 3x3 of globalMatrix
        // Use full upper-left 3x3 of model matrix (includes scale!)
        let model_mat3 = DMat3::from_cols(
            self.shape_transform.col(0).truncate(),
            self.shape_transform.col(1).truncate(),
            self.shape_transform.col(2).truncate(),
        );
        let combined = model_mat3 * rotation;

        // center = modelMatrix * (rotation * center_local)
        let rotated_center = rotation * center_local;
        let world_center = self.shape_transform.transform_point3(rotated_center);

        // halfAxes = 0.5 * modelMat3 * R(angleMid) * diag(extentX, extentY, extentZ)
        // (CesiumJS OrientedBoundingBox.fromTransformation multiplies by 0.5)
        let half_axes = DMat3::from_cols(
            combined.col(0) * extent_x * 0.5,
            combined.col(1) * extent_y * 0.5,
            combined.col(2) * extent_z * 0.5,
        );

        OrientedBoundingBox::new(world_center, half_axes)
    }
}

impl VoxelShape for VoxelCylinderShape {
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

        // Clamp radius to >= 0
        let mut min_b = min_bounds;
        let mut max_b = max_bounds;
        min_b.x = min_b.x.max(0.0);
        max_b.x = max_b.x.max(0.0);

        // Normalize angles to [-π, π]
        min_b.y = negative_pi_to_pi(min_b.y);
        max_b.y = negative_pi_to_pi(max_b.y);

        self.min_bounds = min_b;
        self.max_bounds = max_b;

        // Render bounds = intersection of shape bounds and clip bounds
        let render_min = DVec3::new(
            min_b.x.max(clip_min.x),
            min_b.y.max(clip_min.y),
            min_b.z.max(clip_min.z),
        );
        let render_max = DVec3::new(
            max_b.x.min(clip_max.x),
            max_b.y.min(clip_max.y),
            max_b.z.min(clip_max.z),
        );
        self.render_min_bounds = render_min;
        self.render_max_bounds = render_max;

        // Check visibility
        let scale = DVec3::new(
            model_matrix.col(0).truncate().length(),
            model_matrix.col(1).truncate().length(),
            model_matrix.col(2).truncate().length(),
        );

        if render_max.x == 0.0
            || render_min.x > render_max.x
            || render_min.z > render_max.z
            || scale.x < 1e-10
            || scale.y < 1e-10
            || scale.z < 1e-10
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
        let default_angle_range = std::f64::consts::TAU;
        let shape_is_angle_reversed = max_b.y < min_b.y;
        let shape_angle_range = max_b.y - min_b.y
            + if shape_is_angle_reversed { default_angle_range } else { 0.0 };

        let radius_range = max_b.x - min_b.x;
        let radial_scale = if radius_range != 0.0 { 1.0 / radius_range } else { 0.0 };
        let radial_offset = if radius_range != 0.0 { -min_b.x * radial_scale } else { 1.0 };

        let height_range = max_b.z - min_b.z;
        let height_scale = if height_range != 0.0 { 1.0 / height_range } else { 0.0 };
        let height_offset = if height_range != 0.0 { -min_b.z * height_scale } else { 1.0 };

        let uv_min_angle = (min_b.y - CYLINDER_DEFAULT_MIN_BOUNDS.y) / default_angle_range;
        let uv_max_angle = (max_b.y - CYLINDER_DEFAULT_MIN_BOUNDS.y) / default_angle_range;
        let uv_angle_range_zero = 1.0 - shape_angle_range / default_angle_range;
        let uv_angle_range_origin = (uv_max_angle + 0.5 * uv_angle_range_zero) % 1.0;
        self.shape_uv_angle_range_origin = uv_angle_range_origin;

        let (angle_scale, angle_offset) = if shape_angle_range > 1e-10 {
            let a_scale = default_angle_range / shape_angle_range;
            let shifted_min = uv_min_angle - uv_angle_range_origin;
            let a_offset = -a_scale * (shifted_min - shifted_min.floor());
            (a_scale, a_offset)
        } else {
            (0.0, 1.0)
        };

        self.local_to_shape_uv_scale = DVec3::new(radial_scale, angle_scale, height_scale);
        self.local_to_shape_uv_translate = DVec3::new(radial_offset, angle_offset, height_offset);

        // Compute intersection count
        let mut intersection_count = 1u32; // radius max
        if render_min.x != CYLINDER_DEFAULT_MIN_BOUNDS.x {
            intersection_count += 1; // radius min
        }
        let render_angle_range = {
            let reversed = render_max.y < render_min.y;
            render_max.y - render_min.y + if reversed { default_angle_range } else { 0.0 }
        };
        let epsilon_angle = 1e-10;
        let half_range = default_angle_range * 0.5;
        if render_angle_range >= half_range - epsilon_angle
            && render_angle_range < default_angle_range - epsilon_angle
        {
            intersection_count += 1;
        } else if render_angle_range < half_range - epsilon_angle {
            // Covers both: angle > epsilon (flipped) and angle <= epsilon (zero range)
            intersection_count += 2;
        }
        self.max_intersections = intersection_count;

        true
    }

    fn convert_local_to_shape_uv_space(&self, position_local: DVec3) -> DVec3 {
        let radius = (position_local.x * position_local.x + position_local.y * position_local.y).sqrt();
        let angle = position_local.y.atan2(position_local.x);
        let height = position_local.z;

        let uv_radius = radius * self.local_to_shape_uv_scale.x + self.local_to_shape_uv_translate.x;

        // Convert angle to UV [0,1]
        let mut uv_angle = (angle + std::f64::consts::PI) / std::f64::consts::TAU;
        uv_angle -= self.shape_uv_angle_range_origin;
        uv_angle -= uv_angle.floor();
        uv_angle = uv_angle * self.local_to_shape_uv_scale.y + self.local_to_shape_uv_translate.y;

        let uv_height = height * self.local_to_shape_uv_scale.z + self.local_to_shape_uv_translate.z;

        DVec3::new(uv_radius, uv_angle, uv_height)
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

/// Normalize angle to [-π, π].
fn negative_pi_to_pi(angle: f64) -> f64 {
    let mut a = angle % std::f64::consts::TAU;
    if a > std::f64::consts::PI {
        a -= std::f64::consts::TAU;
    } else if a < -std::f64::consts::PI {
        a += std::f64::consts::TAU;
    }
    a
}

/// Extract rotation matrix from a transform.
fn extract_rotation(matrix: &DMat4) -> DMat3 {
    let col0 = matrix.col(0).truncate();
    let col1 = matrix.col(1).truncate();
    let col2 = matrix.col(2).truncate();
    let l0 = col0.length().max(1e-15);
    let l1 = col1.length().max(1e-15);
    let l2 = col2.length().max(1e-15);
    DMat3::from_cols(col0 / l0, col1 / l1, col2 / l2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cylinder_shape_default() {
        let shape = VoxelCylinderShape::new();
        assert_eq!(shape.min_bounds(), CYLINDER_DEFAULT_MIN_BOUNDS);
        assert_eq!(shape.max_bounds(), CYLINDER_DEFAULT_MAX_BOUNDS);
    }

    #[test]
    fn test_cylinder_shape_update_identity() {
        let mut shape = VoxelCylinderShape::new();
        let visible = shape.update(
            DMat4::IDENTITY,
            CYLINDER_DEFAULT_MIN_BOUNDS,
            CYLINDER_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(visible);
        assert!(shape.bounding_sphere.radius > 0.0);
        assert!(shape.maximum_intersections_length() >= 1);
    }

    #[test]
    fn test_cylinder_shape_invisible_zero_radius() {
        let mut shape = VoxelCylinderShape::new();
        let min_b = DVec3::new(0.0, -std::f64::consts::PI, -1.0);
        let max_b = DVec3::new(0.0, std::f64::consts::PI, 1.0);
        let visible = shape.update(DMat4::IDENTITY, min_b, max_b, None, None);
        assert!(!visible);
    }

    #[test]
    fn test_cylinder_shape_invisible_zero_scale() {
        let mut shape = VoxelCylinderShape::new();
        let matrix = DMat4::from_scale(DVec3::new(0.0, 1.0, 1.0));
        let visible = shape.update(
            matrix,
            CYLINDER_DEFAULT_MIN_BOUNDS,
            CYLINDER_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(!visible);
    }

    #[test]
    fn test_cylinder_shape_uv_center() {
        let mut shape = VoxelCylinderShape::new();
        shape.update(
            DMat4::IDENTITY,
            CYLINDER_DEFAULT_MIN_BOUNDS,
            CYLINDER_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        // Point at radius=0.5, angle=0, height=0
        let uv = shape.convert_local_to_shape_uv_space(DVec3::new(0.5, 0.0, 0.0));
        // Radius UV should be 0.5 (midpoint of [0,1])
        assert!((uv.x - 0.5).abs() < 1e-10);
        // Height UV should be 0.5 (midpoint of [-1,1])
        assert!((uv.z - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_cylinder_shape_tile_obb() {
        let mut shape = VoxelCylinderShape::new();
        shape.update(
            DMat4::IDENTITY,
            CYLINDER_DEFAULT_MIN_BOUNDS,
            CYLINDER_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        // Level 0 tile should encompass the full cylinder
        let obb = shape.compute_obb_for_tile(0, 0, 0, 0);
        assert!(obb.bounding_sphere_radius() > 0.0);
    }

    #[test]
    fn test_cylinder_shape_partial_radius() {
        let mut shape = VoxelCylinderShape::new();
        let min_b = DVec3::new(0.5, -std::f64::consts::PI, -1.0);
        let max_b = DVec3::new(1.0, std::f64::consts::PI, 1.0);
        let visible = shape.update(DMat4::IDENTITY, min_b, max_b, None, None);
        assert!(visible);
        // Should have radius min intersection
        assert!(shape.maximum_intersections_length() >= 2);
    }

    #[test]
    fn test_negative_pi_to_pi() {
        assert!((negative_pi_to_pi(0.0)).abs() < 1e-10);
        assert!((negative_pi_to_pi(std::f64::consts::PI) - std::f64::consts::PI).abs() < 1e-10);
        assert!((negative_pi_to_pi(3.0 * std::f64::consts::PI) - std::f64::consts::PI).abs() < 1e-10);
        assert!((negative_pi_to_pi(-3.0 * std::f64::consts::PI) + std::f64::consts::PI).abs() < 1e-10);
    }
}
