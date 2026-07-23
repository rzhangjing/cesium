//! Frustum and CullingVolume - view frustum definitions and culling.
//! Maps to CesiumJS `Core/PerspectiveFrustum.js`, `Core/OrthographicFrustum.js`, `Core/CullingVolume.js`

use crate::bounding::BoundingSphere;
use crate::ray::{Intersect, Plane};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// A culling volume defined by 6 clipping planes.
/// Maps to CesiumJS `CullingVolume`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CullingVolume {
    /// The 6 clipping planes: left, right, bottom, top, near, far.
    pub planes: [Plane; 6],
}

impl CullingVolume {
    /// Determines the visibility of a bounding sphere relative to this culling volume.
    /// Maps to `CullingVolume.computeVisibility`
    pub fn visibility(&self, sphere: &BoundingSphere) -> Intersect {
        let mut result = Intersect::Inside;

        for plane in &self.planes {
            let distance = plane.point_distance(sphere.center);

            if distance < -sphere.radius {
                return Intersect::Outside;
            }
            if distance < sphere.radius {
                result = Intersect::Intersecting;
            }
        }

        result
    }

    /// Computes the visibility with a parent plane mask for hierarchical culling.
    /// Maps to `CullingVolume.computeVisibilityWithPlaneMask`
    pub fn visibility_with_plane_mask(
        &self,
        sphere: &BoundingSphere,
        parent_plane_mask: u32,
    ) -> u32 {
        if parent_plane_mask == u32::MAX {
            return parent_plane_mask;
        }

        let mut mask = 0u32;
        for (i, plane) in self.planes.iter().enumerate() {
            let parent_mask = 1u32 << i;
            if parent_plane_mask & parent_mask != 0 {
                mask |= parent_mask;
                continue;
            }

            let distance = plane.point_distance(sphere.center);
            if distance < -sphere.radius {
                return u32::MAX; // Completely outside
            }
            if distance >= sphere.radius {
                mask |= parent_mask;
            }
        }

        mask
    }
}

/// A perspective frustum defined by field of view, aspect ratio, and near/far planes.
/// Maps to CesiumJS `PerspectiveFrustum`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PerspectiveFrustum {
    /// The angle of the field of view in radians (vertical).
    pub fov: f64,
    /// The aspect ratio (width / height).
    pub aspect_ratio: f64,
    /// The distance to the near plane.
    pub near: f64,
    /// The distance to the far plane.
    pub far: f64,
    /// Optional horizontal field of view offset (for off-center projections).
    pub x_offset: f64,
    /// Optional vertical field of view offset.
    pub y_offset: f64,
}

impl PerspectiveFrustum {
    pub fn new(fov: f64, aspect_ratio: f64, near: f64, far: f64) -> Self {
        Self {
            fov,
            aspect_ratio,
            near,
            far,
            x_offset: 0.0,
            y_offset: 0.0,
        }
    }

    /// The horizontal field of view in radians.
    /// Maps to `PerspectiveFrustum.fovy` (actually this is the vertical fov)
    #[inline]
    pub fn fovy(&self) -> f64 {
        self.fov
    }

    /// The horizontal field of view.
    pub fn fov_x(&self) -> f64 {
        2.0 * (self.fov_y_half().tan() * self.aspect_ratio).atan()
    }

    fn fov_y_half(&self) -> f64 {
        self.fov * 0.5
    }

    /// Computes the projection matrix.
    /// Maps to `PerspectiveFrustum.projectionMatrix`
    pub fn projection_matrix(&self) -> DMat4 {
        let fovy_half = self.fov_y_half();
        let tan_fovy = fovy_half.tan();
        let top = self.near * tan_fovy;
        let bottom = -top;
        let right = top * self.aspect_ratio;
        let left = -right;

        // Apply offsets
        let left = left + self.x_offset * self.near;
        let right = right + self.x_offset * self.near;
        let bottom = bottom + self.y_offset * self.near;
        let top = top + self.y_offset * self.near;

        perspective_off_center(left, right, bottom, top, self.near, self.far)
    }

    /// Computes an infinite far-plane projection matrix (for shadow mapping).
    /// Maps to `PerspectiveFrustum.infiniteProjectionMatrix`
    pub fn infinite_projection_matrix(&self) -> DMat4 {
        let fovy_half = self.fov_y_half();
        let tan_fovy = fovy_half.tan();
        let top = self.near * tan_fovy;
        let bottom = -top;
        let right = top * self.aspect_ratio;
        let left = -right;

        let e = 1e-10_f64;
        DMat4::from_cols_array(&[
            2.0 * self.near / (right - left), 0.0, 0.0, 0.0,
            0.0, 2.0 * self.near / (top - bottom), 0.0, 0.0,
            (right + left) / (right - left), (top + bottom) / (top - bottom), -1.0 + e, -1.0,
            0.0, 0.0, (-2.0 + e) * self.near, 0.0,
        ])
    }

    /// Computes the culling volume for this frustum at the given position/orientation.
    /// Maps to `PerspectiveOffCenterFrustum.computeCullingVolume`
    pub fn compute_culling_volume(&self, position: DVec3, direction: DVec3, up: DVec3) -> CullingVolume {
        let right = direction.cross(up);

        // Compute off-center frustum parameters (same as projection_matrix)
        let fovy_half = self.fov_y_half();
        let tan_fovy = fovy_half.tan();
        let t = self.near * tan_fovy;
        let b = -t;
        let r = t * self.aspect_ratio;
        let l = -r;

        // Apply offsets
        let l = l + self.x_offset * self.near;
        let r = r + self.x_offset * self.near;
        let b = b + self.y_offset * self.near;
        let t = t + self.y_offset * self.near;

        let near_center = position + direction * self.near;
        let far_center = position + direction * self.far;

        // Left plane: direction from position to left edge of near plane, cross with up
        let left_normal = (near_center + right * l - position).cross(up).normalize();
        let left_plane = Plane::from_point_normal(position, left_normal);

        // Right plane: up cross direction from position to right edge of near plane
        let right_normal = up.cross(near_center + right * r - position).normalize();
        let right_plane = Plane::from_point_normal(position, right_normal);

        // Bottom plane: right cross direction from position to bottom edge of near plane
        let bottom_normal = right.cross(near_center + up * b - position).normalize();
        let bottom_plane = Plane::from_point_normal(position, bottom_normal);

        // Top plane: direction from position to top edge of near plane, cross with right
        let top_normal = (near_center + up * t - position).cross(right).normalize();
        let top_plane = Plane::from_point_normal(position, top_normal);

        // Near plane: normal points along view direction
        let near_plane = Plane::from_point_normal(near_center, direction);

        // Far plane: normal points opposite view direction
        let far_plane = Plane::from_point_normal(far_center, -direction);

        CullingVolume {
            planes: [left_plane, right_plane, bottom_plane, top_plane, near_plane, far_plane],
        }
    }

    /// Computes the pixel dimensions at a given distance.
    /// Maps to `PerspectiveFrustum.getPixelDimensions`
    pub fn pixel_dimensions(&self, _drawing_buffer_width: f64, drawing_buffer_height: f64, distance: f64) -> (f64, f64) {
        let fovy_half = self.fov_y_half();
        let inverse_tan = 1.0 / fovy_half.tan();

        let pixel_height = 2.0 * distance / (drawing_buffer_height * inverse_tan);
        let pixel_width = pixel_height * self.aspect_ratio;

        (pixel_width, pixel_height)
    }

    /// Computes the sse denominator for screen-space error calculations.
    pub fn sse_denominator(&self) -> f64 {
        2.0 * self.fov_y_half().tan()
    }
}

/// An orthographic frustum defined by width, aspect ratio, and near/far planes.
/// Maps to CesiumJS `OrthographicFrustum`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrthographicFrustum {
    /// The width of the frustum at the near plane.
    pub width: f64,
    /// The aspect ratio (width / height).
    pub aspect_ratio: f64,
    /// The distance to the near plane.
    pub near: f64,
    /// The distance to the far plane.
    pub far: f64,
}

impl OrthographicFrustum {
    pub fn new(width: f64, aspect_ratio: f64, near: f64, far: f64) -> Self {
        Self {
            width,
            aspect_ratio,
            near,
            far,
        }
    }

    /// The height of the frustum.
    pub fn height(&self) -> f64 {
        self.width / self.aspect_ratio
    }

    /// Computes the projection matrix.
    /// Maps to `OrthographicFrustum.projectionMatrix`
    pub fn projection_matrix(&self) -> DMat4 {
        let right = self.width * 0.5;
        let left = -right;
        let top = self.height() * 0.5;
        let bottom = -top;

        DMat4::from_cols_array(&[
            2.0 / (right - left), 0.0, 0.0, 0.0,
            0.0, 2.0 / (top - bottom), 0.0, 0.0,
            0.0, 0.0, -2.0 / (self.far - self.near), 0.0,
            -(right + left) / (right - left), -(top + bottom) / (top - bottom),
            -(self.far + self.near) / (self.far - self.near), 1.0,
        ])
    }

    /// Computes the culling volume.
    pub fn compute_culling_volume(&self, position: DVec3, direction: DVec3, up: DVec3) -> CullingVolume {
        let right = direction.cross(up).normalize();
        let half_width = self.width * 0.5;
        let half_height = self.height() * 0.5;

        let near_center = position + direction * self.near;
        let far_center = position + direction * self.far;

        let near_plane = Plane::from_point_normal(near_center, direction);
        let far_plane = Plane::from_point_normal(far_center, -direction);
        let left_plane = Plane::from_point_normal(position - right * half_width, right);
        let right_plane = Plane::from_point_normal(position + right * half_width, -right);
        let bottom_plane = Plane::from_point_normal(position - up * half_height, up);
        let top_plane = Plane::from_point_normal(position + up * half_height, -up);

        CullingVolume {
            planes: [left_plane, right_plane, bottom_plane, top_plane, near_plane, far_plane],
        }
    }

    /// Computes the pixel dimensions at a given distance.
    pub fn pixel_dimensions(&self, drawing_buffer_width: f64, drawing_buffer_height: f64, _distance: f64) -> (f64, f64) {
        let pixel_width = self.width / drawing_buffer_width;
        let pixel_height = self.height() / drawing_buffer_height;
        (pixel_width, pixel_height)
    }
}

// --- Helper functions ---

/// Creates an off-center perspective projection matrix.
fn perspective_off_center(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> DMat4 {
    DMat4::from_cols_array(&[
        2.0 * near / (right - left), 0.0, 0.0, 0.0,
        0.0, 2.0 * near / (top - bottom), 0.0, 0.0,
        (right + left) / (right - left), (top + bottom) / (top - bottom), -(far + near) / (far - near), -1.0,
        0.0, 0.0, -2.0 * far * near / (far - near), 0.0,
    ])
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::math_utils;

    #[test]
    fn test_perspective_projection_matrix() {
        let frustum = PerspectiveFrustum::new(
            math_utils::to_radians(60.0),
            16.0 / 9.0,
            0.1,
            1000.0,
        );
        let proj = frustum.projection_matrix();
        // Check that it's a valid perspective matrix (bottom-right is 0, w-row has -1)
        assert!((proj.w_axis.w).abs() < 1e-10);
        assert!((proj.z_axis.w - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_perspective_fov_x() {
        let frustum = PerspectiveFrustum::new(
            math_utils::to_radians(60.0),
            16.0 / 9.0,
            0.1,
            1000.0,
        );
        let fov_x = frustum.fov_x();
        assert!(fov_x > frustum.fov); // Wider aspect → wider horizontal FOV
    }

    #[test]
    fn test_culling_volume_sphere_inside() {
        let frustum = PerspectiveFrustum::new(
            math_utils::to_radians(90.0),
            1.0,
            1.0,
            100.0,
        );
        let position = DVec3::ZERO;
        let direction = DVec3::new(0.0, 0.0, -1.0);
        let up = DVec3::Y;
        let cv = frustum.compute_culling_volume(position, direction, up);

        // Sphere in front of camera, well within frustum
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, -10.0), 1.0);
        assert_eq!(cv.visibility(&sphere), Intersect::Inside);
    }

    #[test]
    fn test_culling_volume_sphere_outside() {
        let frustum = PerspectiveFrustum::new(
            math_utils::to_radians(60.0),
            1.0,
            1.0,
            100.0,
        );
        let position = DVec3::ZERO;
        let direction = DVec3::new(0.0, 0.0, -1.0);
        let up = DVec3::Y;
        let cv = frustum.compute_culling_volume(position, direction, up);

        // Sphere behind camera
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 10.0), 1.0);
        assert_eq!(cv.visibility(&sphere), Intersect::Outside);
    }

    #[test]
    fn test_pixel_dimensions() {
        let frustum = PerspectiveFrustum::new(
            math_utils::to_radians(60.0),
            1.0,
            1.0,
            1000.0,
        );
        let (pw, ph) = frustum.pixel_dimensions(1024.0, 1024.0, 100.0);
        assert!(pw > 0.0);
        assert!(ph > 0.0);
        assert!((pw - ph).abs() < 1e-10); // aspect 1.0 → square pixels
    }

    #[test]
    fn test_orthographic_projection() {
        let frustum = OrthographicFrustum::new(10.0, 1.0, 0.1, 100.0);
        let proj = frustum.projection_matrix();
        // Orthographic: w-row should be (0, 0, 0, 1)
        assert!((proj.x_axis.w).abs() < 1e-10);
        assert!((proj.y_axis.w).abs() < 1e-10);
        assert!((proj.z_axis.w).abs() < 1e-10);
        assert!((proj.w_axis.w - 1.0).abs() < 1e-10);
    }
}
