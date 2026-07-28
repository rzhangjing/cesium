//! Box voxel shape implementation.
//!
//! Maps to CesiumJS `Scene/VoxelBoxShape.js`.
//! Bounds are in [-1, 1]^3 by default.

use glam::{DMat3, DMat4, DVec3};

use crate::shape::{
    clamp_vec3, lerp, BoundingSphere, OrientedBoundingBox, VoxelShape,
};

/// Default minimum bounds for box shape: (-1, -1, -1).
pub const BOX_DEFAULT_MIN_BOUNDS: DVec3 = DVec3::new(-1.0, -1.0, -1.0);
/// Default maximum bounds for box shape: (1, 1, 1).
pub const BOX_DEFAULT_MAX_BOUNDS: DVec3 = DVec3::new(1.0, 1.0, 1.0);

/// A box-shaped voxel region.
///
/// The box shape maps voxel data to a rectangular region in 3D space.
/// Bounds are specified as minimum and maximum XYZ coordinates.
#[derive(Debug, Clone)]
pub struct VoxelBoxShape {
    /// Oriented bounding box containing the bounded shape.
    obb: OrientedBoundingBox,
    /// Bounding sphere containing the bounded shape.
    bounding_sphere: BoundingSphere,
    /// Transform for the bounded shape.
    bound_transform: DMat4,
    /// Transform for the shape ignoring bounds.
    shape_transform: DMat4,
    /// Minimum bounds.
    min_bounds: DVec3,
    /// Maximum bounds.
    max_bounds: DVec3,
    /// Minimum render bounds (after clipping).
    render_min_bounds: DVec3,
    /// Maximum render bounds (after clipping).
    render_max_bounds: DVec3,
    /// UV scale for local-to-shape-UV transform.
    local_to_shape_uv_scale: DVec3,
    /// UV translate for local-to-shape-UV transform.
    local_to_shape_uv_translate: DVec3,
    /// Maximum intersections count.
    max_intersections: u32,
}

impl Default for VoxelBoxShape {
    fn default() -> Self {
        Self {
            obb: OrientedBoundingBox::default(),
            bounding_sphere: BoundingSphere::default(),
            bound_transform: DMat4::IDENTITY,
            shape_transform: DMat4::IDENTITY,
            min_bounds: BOX_DEFAULT_MIN_BOUNDS,
            max_bounds: BOX_DEFAULT_MAX_BOUNDS,
            render_min_bounds: BOX_DEFAULT_MIN_BOUNDS,
            render_max_bounds: BOX_DEFAULT_MAX_BOUNDS,
            local_to_shape_uv_scale: DVec3::new(0.5, 0.5, 0.5),
            local_to_shape_uv_translate: DVec3::new(0.5, 0.5, 0.5),
            max_intersections: 1,
        }
    }
}

impl VoxelBoxShape {
    /// Create a new box shape with default bounds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the minimum bounds.
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

    /// Check if a point in local coordinates is inside the render bounds.
    pub fn contains_local(&self, point: DVec3) -> bool {
        point.x >= self.render_min_bounds.x
            && point.x <= self.render_max_bounds.x
            && point.y >= self.render_min_bounds.y
            && point.y <= self.render_max_bounds.y
            && point.z >= self.render_min_bounds.z
            && point.z <= self.render_max_bounds.z
    }

    /// Compute the OBB for a subregion of the box.
    fn compute_chunk_obb(&self, min_b: DVec3, max_b: DVec3) -> OrientedBoundingBox {
        let is_default = (min_b - BOX_DEFAULT_MIN_BOUNDS).length() < 1e-10
            && (max_b - BOX_DEFAULT_MAX_BOUNDS).length() < 1e-10;

        if is_default {
            let center = self.shape_transform.transform_point3(DVec3::ZERO);
            let half_axes = DMat3::from_cols(
                self.shape_transform.col(0).truncate(),
                self.shape_transform.col(1).truncate(),
                self.shape_transform.col(2).truncate(),
            );
            OrientedBoundingBox::new(center, half_axes)
        } else {
            let scale = DVec3::new(
                self.shape_transform.col(0).truncate().length(),
                self.shape_transform.col(1).truncate().length(),
                self.shape_transform.col(2).truncate().length(),
            );
            let local_center = (min_b + max_b) * 0.5;
            let center = self.shape_transform.transform_point3(local_center);

            let half_scale = DVec3::new(
                scale.x * 0.5 * (max_b.x - min_b.x),
                scale.y * 0.5 * (max_b.y - min_b.y),
                scale.z * 0.5 * (max_b.z - min_b.z),
            );

            // Extract rotation from shape transform
            let rotation = extract_rotation(&self.shape_transform);
            let half_axes = DMat3::from_cols(
                rotation.col(0) * half_scale.x,
                rotation.col(1) * half_scale.y,
                rotation.col(2) * half_scale.z,
            );
            OrientedBoundingBox::new(center, half_axes)
        }
    }
}

impl VoxelShape for VoxelBoxShape {
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

        let render_min = clamp_vec3(min_bounds, clip_min, clip_max);
        let render_max = clamp_vec3(max_bounds, clip_min, clip_max);
        self.render_min_bounds = render_min;
        self.render_max_bounds = render_max;

        // Check visibility
        let scale = DVec3::new(
            model_matrix.col(0).truncate().length(),
            model_matrix.col(1).truncate().length(),
            model_matrix.col(2).truncate().length(),
        );

        let degenerate_count = (if (render_min.x - render_max.x).abs() < 1e-10 { 1 } else { 0 })
            + (if (render_min.y - render_max.y).abs() < 1e-10 { 1 } else { 0 })
            + (if (render_min.z - render_max.z).abs() < 1e-10 { 1 } else { 0 });

        // CesiumJS: invisible if ANY scale component is zero
        // ("too annoying to reconstruct rotation matrix")
        if render_min.x > render_max.x
            || render_min.y > render_max.y
            || render_min.z > render_max.z
            || degenerate_count >= 2
            || scale.x == 0.0
            || scale.y == 0.0
            || scale.z == 0.0
        {
            return false;
        }

        self.shape_transform = model_matrix;
        self.obb = self.compute_chunk_obb(render_min, render_max);
        self.bounding_sphere = BoundingSphere::from_obb(&self.obb);

        // Bound transform from OBB
        self.bound_transform = DMat4::from_cols(
            self.obb.half_axes.col(0).extend(0.0),
            self.obb.half_axes.col(1).extend(0.0),
            self.obb.half_axes.col(2).extend(0.0),
            self.obb.center.extend(1.0),
        );

        // Compute UV scale and translate
        self.local_to_shape_uv_scale = DVec3::new(
            bound_scale(min_bounds.x, max_bounds.x),
            bound_scale(min_bounds.y, max_bounds.y),
            bound_scale(min_bounds.z, max_bounds.z),
        );
        self.local_to_shape_uv_translate = -(self.local_to_shape_uv_scale * min_bounds);

        self.max_intersections = 1;
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
            lerp(min_b.x, max_b.x, size_at_level * tile_x as f64),
            lerp(min_b.y, max_b.y, size_at_level * tile_y as f64),
            lerp(min_b.z, max_b.z, size_at_level * tile_z as f64),
        );
        let tile_max = DVec3::new(
            lerp(min_b.x, max_b.x, size_at_level * (tile_x + 1) as f64),
            lerp(min_b.y, max_b.y, size_at_level * (tile_y + 1) as f64),
            lerp(min_b.z, max_b.z, size_at_level * (tile_z + 1) as f64),
        );

        self.compute_chunk_obb(tile_min, tile_max)
    }
}

/// Compute scale factor for UV mapping.
fn bound_scale(min_bound: f64, max_bound: f64) -> f64 {
    if (min_bound - max_bound).abs() < 1e-7 {
        1.0
    } else {
        1.0 / (max_bound - min_bound)
    }
}

/// Extract rotation matrix from a transform (normalize columns).
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
    fn test_box_shape_default() {
        let shape = VoxelBoxShape::new();
        assert_eq!(shape.min_bounds(), BOX_DEFAULT_MIN_BOUNDS);
        assert_eq!(shape.max_bounds(), BOX_DEFAULT_MAX_BOUNDS);
        assert_eq!(shape.maximum_intersections_length(), 1);
    }

    #[test]
    fn test_box_shape_update_identity() {
        let mut shape = VoxelBoxShape::new();
        let visible = shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(visible);
        assert_eq!(shape.obb.center, DVec3::ZERO);
        assert!(shape.bounding_sphere.radius > 0.0);
    }

    #[test]
    fn test_box_shape_update_with_translation() {
        let mut shape = VoxelBoxShape::new();
        let matrix = DMat4::from_translation(DVec3::new(10.0, 20.0, 30.0));
        let visible = shape.update(
            matrix,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(visible);
        assert!((shape.obb.center - DVec3::new(10.0, 20.0, 30.0)).length() < 1e-10);
    }

    #[test]
    fn test_box_shape_update_with_scale() {
        let mut shape = VoxelBoxShape::new();
        let matrix = DMat4::from_scale(DVec3::new(2.0, 3.0, 4.0));
        let visible = shape.update(
            matrix,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(visible);
        // Bounding sphere radius should reflect scaled box
        assert!(shape.bounding_sphere.radius > 4.0);
    }

    #[test]
    fn test_box_shape_invisible_degenerate() {
        let mut shape = VoxelBoxShape::new();
        // Zero scale for any single component => invisible (CesiumJS behavior)
        let matrix = DMat4::from_scale(DVec3::new(0.0, 1.0, 1.0));
        let visible = shape.update(
            matrix,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(!visible);

        // Two zero scales => also invisible
        let matrix2 = DMat4::from_scale(DVec3::new(0.0, 0.0, 1.0));
        let visible2 = shape.update(
            matrix2,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(!visible2);
    }

    #[test]
    fn test_box_shape_invisible_clipped_away() {
        let mut shape = VoxelBoxShape::new();
        // Clip bounds that exclude the entire shape
        let visible = shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            Some(DVec3::new(5.0, 5.0, 5.0)),
            Some(DVec3::new(10.0, 10.0, 10.0)),
        );
        assert!(!visible);
    }

    #[test]
    fn test_box_shape_uv_transform() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        // Center of [-1,1] should map to UV (0.5, 0.5, 0.5)
        let uv = shape.convert_local_to_shape_uv_space(DVec3::ZERO);
        assert!((uv.x - 0.5).abs() < 1e-10);
        assert!((uv.y - 0.5).abs() < 1e-10);
        assert!((uv.z - 0.5).abs() < 1e-10);

        // Min corner should map to (0, 0, 0)
        let uv_min = shape.convert_local_to_shape_uv_space(BOX_DEFAULT_MIN_BOUNDS);
        assert!(uv_min.x.abs() < 1e-10);
        assert!(uv_min.y.abs() < 1e-10);
        assert!(uv_min.z.abs() < 1e-10);
    }

    #[test]
    fn test_box_shape_tile_obb() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        // Level 0, tile (0,0,0) should be the full box
        let obb = shape.compute_obb_for_tile(0, 0, 0, 0);
        assert!(obb.center.length() < 1e-10);

        // Level 1, tile (0,0,0) should be the first octant
        let obb_octant = shape.compute_obb_for_tile(1, 0, 0, 0);
        assert!((obb_octant.center - DVec3::new(-0.5, -0.5, -0.5)).length() < 1e-10);
    }

    #[test]
    fn test_box_shape_contains_local() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            DMat4::IDENTITY,
            BOX_DEFAULT_MIN_BOUNDS,
            BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );
        assert!(shape.contains_local(DVec3::ZERO));
        assert!(shape.contains_local(DVec3::new(0.9, -0.9, 0.5)));
        assert!(!shape.contains_local(DVec3::new(1.5, 0.0, 0.0)));
    }

    #[test]
    fn test_box_shape_partial_bounds() {
        let mut shape = VoxelBoxShape::new();
        let min_b = DVec3::new(0.0, 0.0, 0.0);
        let max_b = DVec3::new(1.0, 1.0, 1.0);
        let visible = shape.update(DMat4::IDENTITY, min_b, max_b, None, None);
        assert!(visible);
        // UV of (0,0,0) should be (0,0,0)
        let uv = shape.convert_local_to_shape_uv_space(DVec3::ZERO);
        assert!(uv.x.abs() < 1e-10);
        // UV of (1,1,1) should be (1,1,1)
        let uv_max = shape.convert_local_to_shape_uv_space(DVec3::ONE);
        assert!((uv_max.x - 1.0).abs() < 1e-10);
    }
}
