//! cesium-camera: Camera state, view matrix, movement operations.
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping: `packages/engine/Source/Scene/Camera.js`

use cesium_geospatial::{
    math_utils, Cartographic, CullingVolume, Ellipsoid, HeadingPitchRoll,
    OrthographicFrustum, PerspectiveFrustum,
};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// The camera frustum type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Frustum {
    Perspective(PerspectiveFrustum),
    Orthographic(OrthographicFrustum),
}

impl Default for Frustum {
    fn default() -> Self {
        Self::Perspective(PerspectiveFrustum::new(
            math_utils::to_radians(60.0),
            16.0 / 9.0,
            1.0,
            500_000_000.0,
        ))
    }
}

impl Frustum {
    /// Computes the projection matrix.
    pub fn projection_matrix(&self) -> DMat4 {
        match self {
            Self::Perspective(f) => f.projection_matrix(),
            Self::Orthographic(f) => f.projection_matrix(),
        }
    }

    /// Computes the culling volume at the given position/orientation.
    pub fn compute_culling_volume(
        &self,
        position: DVec3,
        direction: DVec3,
        up: DVec3,
    ) -> CullingVolume {
        match self {
            Self::Perspective(f) => f.compute_culling_volume(position, direction, up),
            Self::Orthographic(f) => f.compute_culling_volume(position, direction, up),
        }
    }

    /// Gets the SSE denominator for screen-space error calculations.
    pub fn sse_denominator(&self) -> f64 {
        match self {
            Self::Perspective(f) => f.sse_denominator(),
            Self::Orthographic(f) => 2.0 / f.height(),
        }
    }
}

/// Camera state: position + orientation (direction/up/right) + frustum.
/// Maps to CesiumJS `Camera` (core state only, no scene dependency)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    /// The position of the camera in world coordinates.
    pub position: DVec3,
    /// The view direction of the camera (unit vector).
    pub direction: DVec3,
    /// The up direction of the camera (unit vector).
    pub up: DVec3,
    /// The right direction of the camera (unit vector).
    pub right: DVec3,
    /// The viewing frustum.
    pub frustum: Frustum,
    /// Default move amount in meters.
    pub default_move_amount: f64,
    /// Default look/rotate amount in radians.
    pub default_look_amount: f64,
    /// Default rotate amount in radians.
    pub default_rotate_amount: f64,
    /// Default zoom amount in meters.
    pub default_zoom_amount: f64,
}

impl Camera {
    /// Creates a new camera at the given position looking in the given direction.
    pub fn new(position: DVec3, direction: DVec3, up: DVec3) -> Self {
        let direction = direction.normalize();
        let up = up.normalize();
        let right = direction.cross(up).normalize();
        let up = right.cross(direction).normalize();

        Self {
            position,
            direction,
            up,
            right,
            frustum: Frustum::default(),
            default_move_amount: 100_000.0,
            default_look_amount: std::f64::consts::PI / 60.0,
            default_rotate_amount: std::f64::consts::PI / 3600.0,
            default_zoom_amount: 100_000.0,
        }
    }

    /// Creates a default camera looking down the -Z axis from the origin.
    pub fn default_camera() -> Self {
        Self::new(DVec3::ZERO, -DVec3::Z, DVec3::Y)
    }

    // ========================================================================
    // Matrix computations
    // ========================================================================

    /// Computes the view matrix.
    /// Maps to `Matrix4.computeView`
    pub fn view_matrix(&self) -> DMat4 {
        compute_view_matrix(self.position, self.direction, self.up, self.right)
    }

    /// Computes the inverse view matrix.
    pub fn inverse_view_matrix(&self) -> DMat4 {
        self.view_matrix().inverse()
    }

    /// Computes the projection matrix.
    pub fn projection_matrix(&self) -> DMat4 {
        self.frustum.projection_matrix()
    }

    /// Computes the view-projection matrix.
    pub fn view_projection_matrix(&self) -> DMat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Computes the culling volume for the current camera state.
    pub fn culling_volume(&self) -> CullingVolume {
        self.frustum
            .compute_culling_volume(self.position, self.direction, self.up)
    }

    // ========================================================================
    // Position queries
    // ========================================================================

    /// Gets the cartographic position (longitude, latitude, height).
    pub fn position_cartographic(&self, ellipsoid: &Ellipsoid) -> Option<Cartographic> {
        ellipsoid.cartesian_to_cartographic(self.position)
    }

    /// Gets the height above the ellipsoid surface.
    pub fn height(&self, ellipsoid: &Ellipsoid) -> Option<f64> {
        self.position_cartographic(ellipsoid).map(|c| c.height)
    }

    /// Gets the magnitude of the camera position (distance from center).
    pub fn position_magnitude(&self) -> f64 {
        self.position.length()
    }

    // ========================================================================
    // Orientation queries
    // ========================================================================

    /// Gets the heading angle in radians.
    /// Maps to `Camera.heading`
    pub fn heading(&self) -> f64 {
        get_heading(self.direction, self.up)
    }

    /// Gets the pitch angle in radians.
    /// Maps to `Camera.pitch`
    pub fn pitch(&self) -> f64 {
        get_pitch(self.direction)
    }

    /// Gets the roll angle in radians.
    /// Maps to `Camera.roll`
    pub fn roll(&self) -> f64 {
        get_roll(self.direction, self.up, self.right)
    }

    /// Gets the heading, pitch, and roll as a struct.
    pub fn heading_pitch_roll(&self) -> HeadingPitchRoll {
        HeadingPitchRoll::new(self.heading(), self.pitch(), self.roll())
    }

    // ========================================================================
    // Movement operations
    // ========================================================================

    /// Moves the camera along the given direction by the given amount.
    /// Maps to `Camera.move`
    pub fn move_along(&mut self, direction: DVec3, amount: f64) {
        self.position += direction.normalize() * amount;
    }

    /// Moves the camera forward (along the view direction).
    /// Maps to `Camera.moveForward`
    pub fn move_forward(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        self.move_along(self.direction, amount);
    }

    /// Moves the camera backward (opposite the view direction).
    /// Maps to `Camera.moveBackward`
    pub fn move_backward(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        self.move_along(self.direction, -amount);
    }

    /// Moves the camera to the right.
    /// Maps to `Camera.moveRight`
    pub fn move_right(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        self.move_along(self.right, amount);
    }

    /// Moves the camera to the left.
    /// Maps to `Camera.moveLeft`
    pub fn move_left(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        self.move_along(self.right, -amount);
    }

    /// Moves the camera up.
    /// Maps to `Camera.moveUp`
    pub fn move_up(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        self.move_along(self.up, amount);
    }

    /// Moves the camera down.
    /// Maps to `Camera.moveDown`
    pub fn move_down(&mut self, amount: Option<f64>) {
        let amount = amount.unwrap_or(self.default_move_amount);
        self.move_along(self.up, -amount);
    }

    /// Rotates the camera around an axis by an angle.
    /// Maps to `Camera.rotate`
    pub fn rotate(&mut self, axis: DVec3, angle: f64) {
        let rotation = glam::DQuat::from_axis_angle(axis.normalize(), angle);
        self.direction = (rotation * self.direction).normalize();
        self.up = (rotation * self.up).normalize();
        self.right = self.direction.cross(self.up).normalize();
    }

    /// Rotates the camera around the world up axis.
    /// Maps to `Camera.rotateUp` / `Camera.rotateDown`
    pub fn rotate_vertical(&mut self, angle: f64) {
        self.rotate(self.right, angle);
    }

    /// Rotates the camera horizontally.
    /// Maps to `Camera.rotateLeft` / `Camera.rotateRight`
    pub fn rotate_horizontal(&mut self, angle: f64) {
        self.rotate(self.up, angle);
    }

    /// Looks along the given axis by an angle (rotates direction and up).
    /// Maps to `Camera.look`
    pub fn look(&mut self, axis: DVec3, angle: f64) {
        self.rotate(axis, angle);
    }

    /// Looks left by the given angle.
    pub fn look_left(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_look_amount);
        self.look(self.up, -angle);
    }

    /// Looks right by the given angle.
    pub fn look_right(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_look_amount);
        self.look(self.up, angle);
    }

    /// Looks up by the given angle.
    pub fn look_up(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_look_amount);
        self.look(self.right, -angle);
    }

    /// Looks down by the given angle.
    pub fn look_down(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_look_amount);
        self.look(self.right, angle);
    }

    /// Twists the camera (rolls) by the given angle.
    pub fn twist(&mut self, angle: f64) {
        self.look(self.direction, angle);
    }

    /// Sets the camera to look at a target from the current position.
    /// Maps to `Camera.lookAt` (simplified)
    pub fn look_at_point(&mut self, target: DVec3, up: DVec3) {
        self.direction = (target - self.position).normalize();
        self.right = self.direction.cross(up).normalize();
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Zooms in by moving forward.
    pub fn zoom_in(&mut self, amount: Option<f64>) {
        self.move_forward(amount);
    }

    /// Zooms out by moving backward.
    pub fn zoom_out(&mut self, amount: Option<f64>) {
        self.move_backward(amount);
    }

    // ========================================================================
    // Setters
    // ========================================================================

    /// Sets the camera position and orientation from heading/pitch/roll at a position.
    pub fn set_view_hpr(
        &mut self,
        position: DVec3,
        heading: f64,
        pitch: f64,
        roll: f64,
        ellipsoid: &Ellipsoid,
    ) {
        self.position = position;

        // Compute ENU frame at position
        let enu = cesium_geospatial::transforms::east_north_up_to_fixed_frame(position, ellipsoid);
        let east = enu.x_axis.truncate();
        let north = enu.y_axis.truncate();
        let up = enu.z_axis.truncate();

        // Apply heading (rotation about up), pitch (rotation about east), roll (rotation about direction)
        let hpr_quat = HeadingPitchRoll::new(heading, pitch, roll).to_quaternion();

        // Transform HPR from ENU to world
        let enu_rotation = glam::DMat3::from_cols(east, north, up);
        let world_quat = glam::DQuat::from_mat3(&enu_rotation) * hpr_quat;

        // In ENU, forward is East, up is Up, right is -North (or South)
        // After HPR rotation in local frame:
        // direction starts as East (1,0,0 in ENU)
        // up starts as Up (0,0,1 in ENU)
        let local_direction = DVec3::X; // East in ENU
        let local_up = DVec3::Z; // Up in ENU

        self.direction = (world_quat * local_direction).normalize();
        self.up = (world_quat * local_up).normalize();
        self.right = self.direction.cross(self.up).normalize();
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Sets the camera to view a bounding sphere.
    /// Maps to `Camera.viewBoundingSphere` (simplified)
    pub fn view_bounding_sphere(
        &mut self,
        center: DVec3,
        radius: f64,
        offset: f64,
        ellipsoid: &Ellipsoid,
    ) {
        let distance = radius / (self.frustum.sse_denominator() * 0.5).max(0.001) + offset;
        let direction = (center - self.position).normalize();
        if direction.length_squared() < 1e-10 {
            return;
        }
        self.position = center - direction * distance;
        self.look_at_point(center, ellipsoid.geodetic_surface_normal(center).unwrap_or(DVec3::Z));
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::default_camera()
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Computes the view matrix from camera vectors.
/// Maps to `Matrix4.computeView`
fn compute_view_matrix(position: DVec3, direction: DVec3, up: DVec3, right: DVec3) -> DMat4 {
    DMat4::from_cols_array(&[
        right.x, up.x, -direction.x, 0.0,
        right.y, up.y, -direction.y, 0.0,
        right.z, up.z, -direction.z, 0.0,
        -right.dot(position), -up.dot(position), direction.dot(position), 1.0,
    ])
}

/// Computes heading from direction and up vectors.
/// Maps to `getHeading` in Camera.js
fn get_heading(direction: DVec3, up: DVec3) -> f64 {
    let heading = if (direction.z.abs() - 1.0).abs() > math_utils::EPSILON3 {
        direction.y.atan2(direction.x) - std::f64::consts::FRAC_PI_2
    } else {
        up.y.atan2(up.x) - std::f64::consts::FRAC_PI_2
    };
    math_utils::TWO_PI - math_utils::zero_to_two_pi(heading)
}

/// Computes pitch from direction vector.
/// Maps to `getPitch` in Camera.js
fn get_pitch(direction: DVec3) -> f64 {
    std::f64::consts::FRAC_PI_2 - direction.z.clamp(-1.0, 1.0).acos()
}

/// Computes roll from direction, up, and right vectors.
/// Maps to `getRoll` in Camera.js
fn get_roll(direction: DVec3, up: DVec3, right: DVec3) -> f64 {
    if (direction.z.abs() - 1.0).abs() > math_utils::EPSILON3 {
        let roll = (-right.z).atan2(up.z);
        math_utils::zero_to_two_pi(roll + math_utils::TWO_PI)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_default_camera() {
        let camera = Camera::default_camera();
        assert!(camera.position.abs_diff_eq(DVec3::ZERO, 1e-10));
        assert!(camera.direction.abs_diff_eq(-DVec3::Z, 1e-10));
        assert!(camera.up.abs_diff_eq(DVec3::Y, 1e-10));
        assert!(camera.right.abs_diff_eq(DVec3::X, 1e-10));
    }

    #[test]
    fn test_view_matrix() {
        let camera = Camera::default_camera();
        let view = camera.view_matrix();

        // For a camera at origin looking down -Z:
        // View matrix should be identity (since we're already at origin looking down -Z)
        // Actually: right=X, up=Y, -direction=Z
        // So the rotation part is identity, translation is zero
        assert!((view.x_axis.x - 1.0).abs() < 1e-10);
        assert!((view.y_axis.y - 1.0).abs() < 1e-10);
        assert!((view.z_axis.z - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_move_forward() {
        let mut camera = Camera::default_camera();
        camera.move_forward(Some(10.0));
        assert!(camera.position.abs_diff_eq(DVec3::new(0.0, 0.0, -10.0), 1e-10));
    }

    #[test]
    fn test_move_right() {
        let mut camera = Camera::default_camera();
        camera.move_right(Some(5.0));
        assert!(camera.position.abs_diff_eq(DVec3::new(5.0, 0.0, 0.0), 1e-10));
    }

    #[test]
    fn test_rotate_horizontal() {
        let mut camera = Camera::default_camera();
        camera.rotate_horizontal(PI / 2.0); // 90 degrees
        // After rotating 90° around up (Y), direction should point to -X
        assert!(
            camera.direction.abs_diff_eq(DVec3::new(-1.0, 0.0, 0.0), 1e-10),
            "direction: {:?}",
            camera.direction
        );
    }

    #[test]
    fn test_look_at_point() {
        let mut camera = Camera::new(
            DVec3::new(0.0, 0.0, 10.0),
            -DVec3::Z,
            DVec3::Y,
        );
        camera.look_at_point(DVec3::ZERO, DVec3::Y);
        assert!(camera.direction.abs_diff_eq(DVec3::new(0.0, 0.0, -1.0), 1e-10));
    }

    #[test]
    fn test_heading_pitch_at_equator() {
        // Camera at equator looking north (horizontally)
        let ellipsoid = Ellipsoid::WGS84;
        let position = DVec3::new(6378137.0, 0.0, 0.0); // On equator at prime meridian
        let mut camera = Camera::default_camera();
        camera.set_view_hpr(position, 0.0, 0.0, 0.0, &ellipsoid);

        // Heading should be 0 (looking north)
        let heading = camera.heading();
        assert!(
            heading.abs() < 0.01 || (heading - math_utils::TWO_PI).abs() < 0.01,
            "heading: {}",
            heading
        );
    }

    #[test]
    fn test_culling_volume() {
        let camera = Camera::default_camera();
        let cv = camera.culling_volume();

        // A sphere in front of the camera should be inside
        let sphere = cesium_geospatial::BoundingSphere::new(DVec3::new(0.0, 0.0, -100.0), 1.0);
        assert_eq!(
            cv.visibility(&sphere),
            cesium_geospatial::Intersect::Inside
        );
    }

    #[test]
    fn test_orthonormalize_after_rotation() {
        let mut camera = Camera::default_camera();
        camera.rotate(DVec3::new(1.0, 1.0, 0.0).normalize(), 0.5);

        // Verify orthonormality
        assert!((camera.direction.length() - 1.0).abs() < 1e-10);
        assert!((camera.up.length() - 1.0).abs() < 1e-10);
        assert!((camera.right.length() - 1.0).abs() < 1e-10);
        assert!(camera.direction.dot(camera.up).abs() < 1e-10);
        assert!(camera.direction.dot(camera.right).abs() < 1e-10);
        assert!(camera.up.dot(camera.right).abs() < 1e-10);
    }
}
