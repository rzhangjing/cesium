//! Voxel shape trait and shape type enum.
//!
//! Maps to CesiumJS `Scene/VoxelShape.js` and `Scene/VoxelShapeType.js`.

use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// An oriented bounding box in 3D space.
#[derive(Debug, Clone, PartialEq)]
pub struct OrientedBoundingBox {
    /// Center of the bounding box.
    pub center: DVec3,
    /// Half-axes as columns of a 3x3 matrix (stored as DMat4 upper-left).
    pub half_axes: glam::DMat3,
}

impl Default for OrientedBoundingBox {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            half_axes: glam::DMat3::IDENTITY,
        }
    }
}

impl OrientedBoundingBox {
    /// Create a new OBB from center and half-axes.
    pub fn new(center: DVec3, half_axes: glam::DMat3) -> Self {
        Self { center, half_axes }
    }

    /// Compute the bounding sphere radius from the half-axes.
    pub fn bounding_sphere_radius(&self) -> f64 {
        let col0 = self.half_axes.col(0);
        let col1 = self.half_axes.col(1);
        let col2 = self.half_axes.col(2);
        (col0.length_squared() + col1.length_squared() + col2.length_squared()).sqrt()
    }

    /// Test if a point is inside the OBB.
    pub fn contains(&self, point: DVec3) -> bool {
        let offset = point - self.center;
        // Project onto each axis
        for i in 0..3 {
            let axis = self.half_axes.col(i);
            let half_len = axis.length();
            if half_len < 1e-15 {
                continue;
            }
            let dir = axis / half_len;
            let proj = offset.dot(dir);
            if proj.abs() > half_len {
                return false;
            }
        }
        true
    }

    /// Compute distance from a point to the OBB surface (0 if inside).
    pub fn distance_to(&self, point: DVec3) -> f64 {
        let offset = point - self.center;
        let mut dist_sq = 0.0;
        for i in 0..3 {
            let axis = self.half_axes.col(i);
            let half_len = axis.length();
            if half_len < 1e-15 {
                continue;
            }
            let dir = axis / half_len;
            let proj = offset.dot(dir);
            let excess = proj.abs() - half_len;
            if excess > 0.0 {
                dist_sq += excess * excess;
            }
        }
        dist_sq.sqrt()
    }
}

/// A bounding sphere in 3D space.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundingSphere {
    /// Center of the sphere.
    pub center: DVec3,
    /// Radius of the sphere.
    pub radius: f64,
}

impl Default for BoundingSphere {
    fn default() -> Self {
        Self {
            center: DVec3::ZERO,
            radius: 0.0,
        }
    }
}

impl BoundingSphere {
    /// Create from an oriented bounding box.
    pub fn from_obb(obb: &OrientedBoundingBox) -> Self {
        Self {
            center: obb.center,
            radius: obb.bounding_sphere_radius(),
        }
    }

    /// Test if a point is inside the sphere.
    pub fn contains(&self, point: DVec3) -> bool {
        (point - self.center).length() <= self.radius
    }

    /// Compute distance from a point to the sphere surface.
    pub fn distance_to(&self, point: DVec3) -> f64 {
        ((point - self.center).length() - self.radius).max(0.0)
    }
}

/// The type of voxel shape, controlling how the voxel grid maps to 3D space.
///
/// Maps to CesiumJS `VoxelShapeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VoxelShapeType {
    /// A box shape with bounds in [-1, 1]^3.
    Box,
    /// An ellipsoid shape with bounds in [lon, lat, height].
    Ellipsoid,
    /// A cylinder shape with bounds in [radius, angle, height].
    Cylinder,
}

impl VoxelShapeType {
    /// Get the default minimum bounds for this shape type.
    pub fn default_min_bounds(&self) -> DVec3 {
        match self {
            Self::Box => DVec3::new(-1.0, -1.0, -1.0),
            Self::Ellipsoid => DVec3::new(-std::f64::consts::PI, -std::f64::consts::FRAC_PI_2, -1.0),
            Self::Cylinder => DVec3::new(0.0, -std::f64::consts::PI, -1.0),
        }
    }

    /// Get the default maximum bounds for this shape type.
    pub fn default_max_bounds(&self) -> DVec3 {
        match self {
            Self::Box => DVec3::new(1.0, 1.0, 1.0),
            Self::Ellipsoid => DVec3::new(std::f64::consts::PI, std::f64::consts::FRAC_PI_2, 1.0),
            Self::Cylinder => DVec3::new(1.0, std::f64::consts::PI, 1.0),
        }
    }
}

/// Trait for voxel shapes that control culling and rendering of voxel grids.
///
/// Maps to CesiumJS `VoxelShape` interface.
pub trait VoxelShape {
    /// Get the oriented bounding box containing the bounded shape.
    fn oriented_bounding_box(&self) -> &OrientedBoundingBox;

    /// Get the bounding sphere containing the bounded shape.
    fn bounding_sphere(&self) -> &BoundingSphere;

    /// Get the transformation matrix containing the bounded shape.
    fn bound_transform(&self) -> DMat4;

    /// Get the transformation matrix containing the shape, ignoring bounds.
    fn shape_transform(&self) -> DMat4;

    /// Get the maximum number of ray-shape intersections for any direction.
    fn maximum_intersections_length(&self) -> u32;

    /// Update the shape's state. Returns whether the shape is visible.
    fn update(
        &mut self,
        model_matrix: DMat4,
        min_bounds: DVec3,
        max_bounds: DVec3,
        clip_min_bounds: Option<DVec3>,
        clip_max_bounds: Option<DVec3>,
    ) -> bool;

    /// Convert a local coordinate to the shape's UV space.
    fn convert_local_to_shape_uv_space(&self, position_local: DVec3) -> DVec3;

    /// Compute an oriented bounding box for a specified tile.
    fn compute_obb_for_tile(
        &self,
        tile_level: u32,
        tile_x: u32,
        tile_y: u32,
        tile_z: u32,
    ) -> OrientedBoundingBox;
}

/// Linear interpolation.
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Clamp a value component-wise between min and max vectors.
#[inline]
pub fn clamp_vec3(v: DVec3, min: DVec3, max: DVec3) -> DVec3 {
    DVec3::new(
        v.x.clamp(min.x, max.x),
        v.y.clamp(min.y, max.y),
        v.z.clamp(min.z, max.z),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_type_default_bounds() {
        let box_min = VoxelShapeType::Box.default_min_bounds();
        let box_max = VoxelShapeType::Box.default_max_bounds();
        assert_eq!(box_min, DVec3::new(-1.0, -1.0, -1.0));
        assert_eq!(box_max, DVec3::new(1.0, 1.0, 1.0));

        let cyl_min = VoxelShapeType::Cylinder.default_min_bounds();
        let cyl_max = VoxelShapeType::Cylinder.default_max_bounds();
        assert_eq!(cyl_min.x, 0.0);
        assert!((cyl_min.y + std::f64::consts::PI).abs() < 1e-10);
        assert_eq!(cyl_max.x, 1.0);
        assert!((cyl_max.y - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_obb_contains() {
        let obb = OrientedBoundingBox::new(
            DVec3::ZERO,
            glam::DMat3::from_cols(
                DVec3::new(2.0, 0.0, 0.0),
                DVec3::new(0.0, 2.0, 0.0),
                DVec3::new(0.0, 0.0, 2.0),
            ),
        );
        assert!(obb.contains(DVec3::new(1.0, 1.0, 1.0)));
        assert!(obb.contains(DVec3::new(-1.5, 0.0, 0.0)));
        assert!(!obb.contains(DVec3::new(2.5, 0.0, 0.0)));
    }

    #[test]
    fn test_obb_distance() {
        let obb = OrientedBoundingBox::new(
            DVec3::ZERO,
            glam::DMat3::from_cols(
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
            ),
        );
        assert_eq!(obb.distance_to(DVec3::ZERO), 0.0);
        assert!((obb.distance_to(DVec3::new(2.0, 0.0, 0.0)) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_bounding_sphere_from_obb() {
        let obb = OrientedBoundingBox::new(
            DVec3::new(1.0, 2.0, 3.0),
            glam::DMat3::from_cols(
                DVec3::new(1.0, 0.0, 0.0),
                DVec3::new(0.0, 1.0, 0.0),
                DVec3::new(0.0, 0.0, 1.0),
            ),
        );
        let bs = BoundingSphere::from_obb(&obb);
        assert_eq!(bs.center, DVec3::new(1.0, 2.0, 3.0));
        assert!((bs.radius - 3.0_f64.sqrt()).abs() < 1e-10);
    }

    #[test]
    fn test_lerp() {
        assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < 1e-10);
        assert!((lerp(-1.0, 1.0, 0.0) - (-1.0)).abs() < 1e-10);
        assert!((lerp(-1.0, 1.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_clamp_vec3() {
        let v = DVec3::new(-2.0, 0.5, 3.0);
        let min = DVec3::new(-1.0, -1.0, -1.0);
        let max = DVec3::new(1.0, 1.0, 1.0);
        let result = clamp_vec3(v, min, max);
        assert_eq!(result, DVec3::new(-1.0, 0.5, 1.0));
    }
}
