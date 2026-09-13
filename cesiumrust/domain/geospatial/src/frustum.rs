//! Frustum and CullingVolume - view frustum definitions and culling.
//! Maps to CesiumJS `Core/PerspectiveFrustum.js`, `Core/OrthographicFrustum.js`, `Core/CullingVolume.js`

use crate::bounding::{AxisAlignedBoundingBox, BoundingSphere};
use crate::ray::{Intersect, Plane};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// Trait for bounding volumes that can be tested against culling planes.
/// Maps to the duck-typed `boundingVolume.intersectPlane(plane)` in CesiumJS.
pub trait Cullable {
    /// Determines which side of a plane this bounding volume is located.
    fn cullable_intersect_plane(&self, plane: &Plane) -> Intersect;
}

impl Cullable for BoundingSphere {
    fn cullable_intersect_plane(&self, plane: &Plane) -> Intersect {
        self.intersect_plane(plane.normal, plane.distance)
    }
}

impl Cullable for AxisAlignedBoundingBox {
    fn cullable_intersect_plane(&self, plane: &Plane) -> Intersect {
        self.intersect_plane(plane.normal, plane.distance)
    }
}

/// A culling volume defined by 6 clipping planes.
/// Maps to CesiumJS `CullingVolume`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CullingVolume {
    /// The 6 clipping planes: left, right, bottom, top, near, far.
    pub planes: [Plane; 6],
}

impl CullingVolume {
    /// The object is entirely outside the culling volume.
    /// Maps to `CullingVolume.MASK_OUTSIDE`
    pub const MASK_OUTSIDE: u32 = 0xffffffff;
    /// The object is entirely inside the culling volume.
    /// Maps to `CullingVolume.MASK_INSIDE`
    pub const MASK_INSIDE: u32 = 0x00000000;
    /// The object may intersect all planes of the culling volume.
    /// Maps to `CullingVolume.MASK_INDETERMINATE`
    pub const MASK_INDETERMINATE: u32 = 0x7fffffff;

    /// Constructs a culling volume from a bounding sphere.
    /// Creates six planes that create a box containing the sphere,
    /// aligned to the x, y, and z axes in world coordinates.
    /// Maps to `CullingVolume.fromBoundingSphere`
    pub fn from_bounding_sphere(sphere: &BoundingSphere) -> Self {
        let center = sphere.center;
        let radius = sphere.radius;
        let faces = [DVec3::X, DVec3::Y, DVec3::Z];

        let mut planes = [Plane::ORIGIN_XY_PLANE; 6];
        let mut plane_index = 0;

        for face_normal in &faces {
            // plane0: normal = faceNormal, through (center - faceNormal * radius)
            let point0 = center - *face_normal * radius;
            planes[plane_index] = Plane::from_point_normal(point0, *face_normal);

            // plane1: normal = -faceNormal, through (center + faceNormal * radius)
            let point1 = center + *face_normal * radius;
            planes[plane_index + 1] = Plane::from_point_normal(point1, -*face_normal);

            plane_index += 2;
        }

        CullingVolume { planes }
    }

    /// Determines the visibility of a bounding volume relative to this culling volume.
    /// Maps to `CullingVolume.computeVisibility`
    pub fn visibility(&self, volume: &impl Cullable) -> Intersect {
        let mut intersecting = false;

        for plane in &self.planes {
            let result = volume.cullable_intersect_plane(plane);
            if result == Intersect::Outside {
                return Intersect::Outside;
            } else if result == Intersect::Intersecting {
                intersecting = true;
            }
        }

        if intersecting {
            Intersect::Intersecting
        } else {
            Intersect::Inside
        }
    }

    /// Computes the visibility with a parent plane mask for hierarchical culling.
    /// Maps to `CullingVolume.computeVisibilityWithPlaneMask`
    pub fn visibility_with_plane_mask(
        &self,
        volume: &impl Cullable,
        parent_plane_mask: u32,
    ) -> u32 {
        if parent_plane_mask == Self::MASK_OUTSIDE || parent_plane_mask == Self::MASK_INSIDE {
            return parent_plane_mask;
        }

        let mut mask = Self::MASK_INSIDE;

        for (k, plane) in self.planes.iter().enumerate() {
            let flag = if k < 31 { 1u32 << k } else { 0 };
            if k < 31 && (parent_plane_mask & flag) == 0 {
                // bounding volume is known to be INSIDE this plane
                continue;
            }

            let result = volume.cullable_intersect_plane(plane);
            if result == Intersect::Outside {
                return Self::MASK_OUTSIDE;
            } else if result == Intersect::Intersecting {
                mask |= flag;
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
    /// Maps to `PerspectiveOffCenterFrustum.getPixelDimensions`
    ///
    /// # Arguments
    /// * `drawing_buffer_width` - Width of the drawing buffer in pixels
    /// * `drawing_buffer_height` - Height of the drawing buffer in pixels
    /// * `distance` - Distance from the camera to the object
    /// * `pixel_ratio` - The pixel ratio (default 1.0)
    ///
    /// Returns (pixel_width, pixel_height) - the size of a pixel in world units at the given distance.
    pub fn pixel_dimensions(&self, drawing_buffer_width: f64, drawing_buffer_height: f64, distance: f64, pixel_ratio: f64) -> (f64, f64) {
        let tan_phi = self.fov_y_half().tan();
        let tan_theta = tan_phi * self.aspect_ratio;
        let pixel_width = (2.0 * pixel_ratio * distance * tan_theta) / drawing_buffer_width;
        let pixel_height = (2.0 * pixel_ratio * distance * tan_phi) / drawing_buffer_height;
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
    /// Maps to `OrthographicOffCenterFrustum.getPixelDimensions`
    ///
    /// Returns (pixel_width, pixel_height) - the size of a pixel in world units.
    pub fn pixel_dimensions(&self, drawing_buffer_width: f64, drawing_buffer_height: f64, _distance: f64, pixel_ratio: f64) -> (f64, f64) {
        let pixel_width = (pixel_ratio * self.width) / drawing_buffer_width;
        let pixel_height = (pixel_ratio * self.height()) / drawing_buffer_height;
        (pixel_width, pixel_height)
    }
}

// --- Off-center frustums ---

/// A perspective frustum defined by six clipping plane distances
/// (left, right, top, bottom, near, far).
///
/// This is the lower-level frustum used by `PerspectiveFrustum`; it allows
/// off-center (asymmetric) projections. `left`/`right`/`top`/`bottom` are
/// `Option` because CesiumJS leaves them `undefined` until set, and accessing
/// the projection matrix before they are set throws a `DeveloperError`.
/// Maps to CesiumJS `PerspectiveOffCenterFrustum`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PerspectiveOffCenterFrustum {
    /// The left clipping plane distance (`undefined` until set).
    pub left: Option<f64>,
    /// The right clipping plane distance (`undefined` until set).
    pub right: Option<f64>,
    /// The top clipping plane distance (`undefined` until set).
    pub top: Option<f64>,
    /// The bottom clipping plane distance (`undefined` until set).
    pub bottom: Option<f64>,
    /// The distance of the near plane (default `1.0`).
    pub near: f64,
    /// The distance of the far plane (default `500000000.0`).
    pub far: f64,
}

impl Default for PerspectiveOffCenterFrustum {
    fn default() -> Self {
        Self {
            left: None,
            right: None,
            top: None,
            bottom: None,
            near: 1.0,
            far: 500_000_000.0,
        }
    }
}

impl PerspectiveOffCenterFrustum {
    /// Creates a default (empty) off-center perspective frustum.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an off-center perspective frustum from explicit bounds.
    pub fn from_bounds(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> Self {
        Self {
            left: Some(left),
            right: Some(right),
            top: Some(top),
            bottom: Some(bottom),
            near,
            far,
        }
    }

    /// Resolves the four lateral bounds, panicking if any is unset
    /// (mirrors CesiumJS `update()` throwing `DeveloperError`).
    fn bounds(&self) -> (f64, f64, f64, f64) {
        let left = self
            .left
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        let right = self
            .right
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        let top = self
            .top
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        let bottom = self
            .bottom
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        (left, right, bottom, top)
    }

    /// The perspective projection matrix.
    /// Maps to `PerspectiveOffCenterFrustum.projectionMatrix`
    pub fn projection_matrix(&self) -> DMat4 {
        let (left, right, bottom, top) = self.bounds();
        perspective_off_center(left, right, bottom, top, self.near, self.far)
    }

    /// The perspective projection matrix with an infinite far plane.
    /// Maps to `PerspectiveOffCenterFrustum.infiniteProjectionMatrix`
    pub fn infinite_projection_matrix(&self) -> DMat4 {
        let (left, right, bottom, top) = self.bounds();
        infinite_perspective_off_center(left, right, bottom, top, self.near)
    }

    /// Creates a culling volume for this frustum at the given pose.
    /// Maps to `PerspectiveOffCenterFrustum.computeCullingVolume`
    pub fn compute_culling_volume(&self, position: DVec3, direction: DVec3, up: DVec3) -> CullingVolume {
        let (left, right, bottom, top) = self.bounds();
        let l = left;
        let r = right;
        let b = bottom;
        let t = top;
        let n = self.near;
        let f = self.far;

        let right_vec = direction.cross(up);
        let near_center = position + direction * n;
        let far_center = position + direction * f;

        // Left plane: normalize(nearCenter + right*l - position) x up
        let left_normal = (near_center + right_vec * l - position).cross(up).normalize();
        let left_plane = Plane::from_point_normal(position, left_normal);

        // Right plane: up x normalize(nearCenter + right*r - position)
        let right_normal = up.cross(near_center + right_vec * r - position).normalize();
        let right_plane = Plane::from_point_normal(position, right_normal);

        // Bottom plane: right x normalize(nearCenter + up*b - position)
        let bottom_normal = right_vec.cross(near_center + up * b - position).normalize();
        let bottom_plane = Plane::from_point_normal(position, bottom_normal);

        // Top plane: normalize(nearCenter + up*t - position) x right
        let top_normal = (near_center + up * t - position).cross(right_vec).normalize();
        let top_plane = Plane::from_point_normal(position, top_normal);

        // Near plane: normal along view direction through near center.
        let near_plane = Plane::from_point_normal(near_center, direction);

        // Far plane: normal opposite view direction through far center.
        let far_plane = Plane::from_point_normal(far_center, -direction);

        CullingVolume {
            planes: [left_plane, right_plane, bottom_plane, top_plane, near_plane, far_plane],
        }
    }

    /// Returns the pixel's width and height in meters.
    /// Maps to `PerspectiveOffCenterFrustum.getPixelDimensions`
    pub fn pixel_dimensions(
        &self,
        drawing_buffer_width: f64,
        drawing_buffer_height: f64,
        distance: f64,
        pixel_ratio: f64,
    ) -> (f64, f64) {
        let top = self
            .top
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        let right = self
            .right
            .expect("right, left, top, bottom, near, or far parameters are not set.");

        let inverse_near = 1.0 / self.near;
        let tan_theta = top * inverse_near;
        let pixel_height = (2.0 * pixel_ratio * distance * tan_theta) / drawing_buffer_height;
        let tan_theta = right * inverse_near;
        let pixel_width = (2.0 * pixel_ratio * distance * tan_theta) / drawing_buffer_width;
        (pixel_width, pixel_height)
    }

    /// Componentwise equality.
    /// Maps to `PerspectiveOffCenterFrustum.equals`
    pub fn equals(&self, other: &Self) -> bool {
        self.right == other.right
            && self.left == other.left
            && self.top == other.top
            && self.bottom == other.bottom
            && self.near == other.near
            && self.far == other.far
    }

    /// Componentwise equality within a relative/absolute tolerance.
    /// Maps to `PerspectiveOffCenterFrustum.equalsEpsilon`
    pub fn equals_epsilon(&self, other: &Self, relative_epsilon: f64, absolute_epsilon: f64) -> bool {
        crate::math_utils::equals_epsilon(self.right.unwrap_or(f64::NAN), other.right.unwrap_or(f64::NAN), relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.left.unwrap_or(f64::NAN), other.left.unwrap_or(f64::NAN), relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.top.unwrap_or(f64::NAN), other.top.unwrap_or(f64::NAN), relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.bottom.unwrap_or(f64::NAN), other.bottom.unwrap_or(f64::NAN), relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.near, other.near, relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.far, other.far, relative_epsilon, absolute_epsilon)
    }
}

/// An orthographic frustum defined by six clipping plane distances
/// (left, right, top, bottom, near, far).
/// Maps to CesiumJS `OrthographicOffCenterFrustum`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OrthographicOffCenterFrustum {
    /// The left clipping plane (`undefined` until set).
    pub left: Option<f64>,
    /// The right clipping plane (`undefined` until set).
    pub right: Option<f64>,
    /// The top clipping plane (`undefined` until set).
    pub top: Option<f64>,
    /// The bottom clipping plane (`undefined` until set).
    pub bottom: Option<f64>,
    /// The distance of the near plane (default `1.0`).
    pub near: f64,
    /// The distance of the far plane (default `500000000.0`).
    pub far: f64,
}

impl Default for OrthographicOffCenterFrustum {
    fn default() -> Self {
        Self {
            left: None,
            right: None,
            top: None,
            bottom: None,
            near: 1.0,
            far: 500_000_000.0,
        }
    }
}

impl OrthographicOffCenterFrustum {
    /// Creates a default (empty) off-center orthographic frustum.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an off-center orthographic frustum from explicit bounds.
    pub fn from_bounds(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> Self {
        Self {
            left: Some(left),
            right: Some(right),
            top: Some(top),
            bottom: Some(bottom),
            near,
            far,
        }
    }

    /// Resolves the four lateral bounds, panicking if any is unset
    /// (mirrors CesiumJS `update()` throwing `DeveloperError`).
    fn bounds(&self) -> (f64, f64, f64, f64) {
        let left = self
            .left
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        let right = self
            .right
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        let top = self
            .top
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        let bottom = self
            .bottom
            .expect("right, left, top, bottom, near, or far parameters are not set.");
        (left, right, bottom, top)
    }

    /// The orthographic projection matrix.
    /// Maps to `OrthographicOffCenterFrustum.projectionMatrix`
    pub fn projection_matrix(&self) -> DMat4 {
        let (left, right, bottom, top) = self.bounds();
        orthographic_off_center(left, right, bottom, top, self.near, self.far)
    }

    /// Creates a culling volume for this frustum at the given pose.
    /// Maps to `OrthographicOffCenterFrustum.computeCullingVolume`
    pub fn compute_culling_volume(&self, position: DVec3, direction: DVec3, up: DVec3) -> CullingVolume {
        let (left, right, bottom, top) = self.bounds();
        let l = left;
        let r = right;
        let b = bottom;
        let t = top;
        let n = self.near;
        let f = self.far;

        // Note: the orthographic variant normalizes the right vector.
        let right_vec = direction.cross(up).normalize();
        let near_center = position + direction * n;

        // Left plane: normal = right, through nearCenter + right*l.
        let left_plane = Plane::from_point_normal(near_center + right_vec * l, right_vec);
        // Right plane: normal = -right, through nearCenter + right*r.
        let right_plane = Plane::from_point_normal(near_center + right_vec * r, -right_vec);
        // Bottom plane: normal = up, through nearCenter + up*b.
        let bottom_plane = Plane::from_point_normal(near_center + up * b, up);
        // Top plane: normal = -up, through nearCenter + up*t.
        let top_plane = Plane::from_point_normal(near_center + up * t, -up);
        // Near plane: normal along view direction through near center.
        let near_plane = Plane::from_point_normal(near_center, direction);
        // Far plane: normal opposite view direction through far center.
        let far_plane = Plane::from_point_normal(position + direction * f, -direction);

        CullingVolume {
            planes: [left_plane, right_plane, bottom_plane, top_plane, near_plane, far_plane],
        }
    }

    /// Returns the pixel's width and height in meters.
    /// Maps to `OrthographicOffCenterFrustum.getPixelDimensions`
    pub fn pixel_dimensions(
        &self,
        drawing_buffer_width: f64,
        drawing_buffer_height: f64,
        _distance: f64,
        pixel_ratio: f64,
    ) -> (f64, f64) {
        let (left, right, bottom, top) = self.bounds();
        let frustum_width = right - left;
        let frustum_height = top - bottom;
        let pixel_width = (pixel_ratio * frustum_width) / drawing_buffer_width;
        let pixel_height = (pixel_ratio * frustum_height) / drawing_buffer_height;
        (pixel_width, pixel_height)
    }

    /// Componentwise equality.
    /// Maps to `OrthographicOffCenterFrustum.equals`
    pub fn equals(&self, other: &Self) -> bool {
        self.right == other.right
            && self.left == other.left
            && self.top == other.top
            && self.bottom == other.bottom
            && self.near == other.near
            && self.far == other.far
    }

    /// Componentwise equality within a relative/absolute tolerance.
    /// Maps to `OrthographicOffCenterFrustum.equalsEpsilon`
    pub fn equals_epsilon(&self, other: &Self, relative_epsilon: f64, absolute_epsilon: f64) -> bool {
        crate::math_utils::equals_epsilon(self.right.unwrap_or(f64::NAN), other.right.unwrap_or(f64::NAN), relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.left.unwrap_or(f64::NAN), other.left.unwrap_or(f64::NAN), relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.top.unwrap_or(f64::NAN), other.top.unwrap_or(f64::NAN), relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.bottom.unwrap_or(f64::NAN), other.bottom.unwrap_or(f64::NAN), relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.near, other.near, relative_epsilon, absolute_epsilon)
            && crate::math_utils::equals_epsilon(self.far, other.far, relative_epsilon, absolute_epsilon)
    }
}

// --- Helper functions ---

/// Creates an off-center perspective projection matrix.
/// Maps to `Matrix4.computePerspectiveOffCenter`
fn perspective_off_center(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> DMat4 {
    DMat4::from_cols_array(&[
        2.0 * near / (right - left), 0.0, 0.0, 0.0,
        0.0, 2.0 * near / (top - bottom), 0.0, 0.0,
        (right + left) / (right - left), (top + bottom) / (top - bottom), -(far + near) / (far - near), -1.0,
        0.0, 0.0, -2.0 * far * near / (far - near), 0.0,
    ])
}

/// Creates an off-center perspective projection matrix with an infinite far plane.
/// Maps to `Matrix4.computeInfinitePerspectiveOffCenter`
fn infinite_perspective_off_center(left: f64, right: f64, bottom: f64, top: f64, near: f64) -> DMat4 {
    DMat4::from_cols_array(&[
        2.0 * near / (right - left), 0.0, 0.0, 0.0,
        0.0, 2.0 * near / (top - bottom), 0.0, 0.0,
        (right + left) / (right - left), (top + bottom) / (top - bottom), -1.0, -1.0,
        0.0, 0.0, -2.0 * near, 0.0,
    ])
}

/// Creates an off-center orthographic projection matrix.
/// Maps to `Matrix4.computeOrthographicOffCenter`
fn orthographic_off_center(left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) -> DMat4 {
    let mut a = 1.0 / (right - left);
    let mut b = 1.0 / (top - bottom);
    let mut c = 1.0 / (far - near);

    let tx = -(right + left) * a;
    let ty = -(top + bottom) * b;
    let tz = -(far + near) * c;
    a *= 2.0;
    b *= 2.0;
    c *= -2.0;

    DMat4::from_cols_array(&[
        a, 0.0, 0.0, 0.0,
        0.0, b, 0.0, 0.0,
        0.0, 0.0, c, 0.0,
        tx, ty, tz, 1.0,
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
        let (pw, ph) = frustum.pixel_dimensions(1024.0, 1024.0, 100.0, 1.0);
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
