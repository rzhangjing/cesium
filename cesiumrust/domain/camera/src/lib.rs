//! cesium-camera: Camera state, view matrix, movement operations.
//! Domain layer - pure Rust, f64 precision.
//!
//! CesiumJS mapping: `packages/engine/Source/Scene/Camera.js`

use cesium_geospatial::{
    math_utils, BoundingSphere, Cartographic, CullingVolume, Ellipsoid, HeadingPitchRange,
    HeadingPitchRoll, OrthographicFrustum, PerspectiveFrustum, Rectangle,
};
use glam::{DMat4, DVec3};
use serde::{Deserialize, Serialize};

/// The scene rendering mode.
/// Maps to CesiumJS `Scene/SceneMode.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SceneMode {
    /// 2D map projection (top-down).
    Scene2D,
    /// 3D globe view.
    #[default]
    Scene3D,
    /// 2.5D Columbus view (flat map with 3D objects).
    ColumbusView,
    /// Transitioning between modes.
    Morphing,
}

/// Easing functions for camera flights.
/// Maps to CesiumJS `Core/EasingFunction.js`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EasingFunction {
    /// Linear interpolation.
    Linear,
    /// Sinusoidal ease-in-out.
    SinusoidalInOut,
    /// Quadratic ease-in.
    QuadraticIn,
    /// Quadratic ease-out.
    QuadraticOut,
    /// Quadratic ease-in-out.
    QuadraticInOut,
    /// Cubic ease-in-out.
    CubicInOut,
    /// Exponential ease-in-out.
    ExponentialInOut,
}

impl EasingFunction {
    /// Evaluates the easing function at time t (0..1).
    pub fn evaluate(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::SinusoidalInOut => 0.5 * (1.0 - (std::f64::consts::PI * t).cos()),
            Self::QuadraticIn => t * t,
            Self::QuadraticOut => t * (2.0 - t),
            Self::QuadraticInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Self::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            Self::ExponentialInOut => {
                if t < 0.5 {
                    (2.0_f64).powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - (2.0_f64).powf(-20.0 * t + 10.0)) / 2.0
                }
            }
        }
    }
}

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
    /// The scene mode (2D/3D/Columbus View/Morphing).
    pub mode: SceneMode,
    /// The reference frame transform (identity = world coordinates).
    pub transform: DMat4,
    /// If set, the camera cannot rotate past this axis.
    pub constrained_axis: Option<DVec3>,
    /// The amount the camera has to change before `changed` fires (0..1).
    pub percentage_changed: f64,
    /// Maximum zoom factor for 2D mode.
    pub maximum_zoom_factor: f64,
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
            mode: SceneMode::Scene3D,
            transform: DMat4::IDENTITY,
            constrained_axis: None,
            percentage_changed: 0.5,
            maximum_zoom_factor: 1.5,
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

    // ========================================================================
    // Transform & coordinate conversions
    // ========================================================================

    /// Sets the reference frame transform.
    /// Maps to `Camera._setTransform`
    pub fn set_transform(&mut self, transform: DMat4) {
        // Save world-space state
        let position_wc = self.position_wc();
        let direction_wc = self.direction_wc();
        let up_wc = self.up_wc();

        self.transform = transform;
        let inv = transform.inverse();

        // Convert to local frame
        self.position = (inv * position_wc.extend(1.0)).truncate();
        self.direction = (inv * direction_wc.extend(0.0)).truncate().normalize();
        self.up = (inv * up_wc.extend(0.0)).truncate().normalize();
        self.right = self.direction.cross(self.up).normalize();
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Gets the position in world coordinates.
    pub fn position_wc(&self) -> DVec3 {
        (self.transform * self.position.extend(1.0)).truncate()
    }

    /// Gets the direction in world coordinates.
    pub fn direction_wc(&self) -> DVec3 {
        (self.transform * self.direction.extend(0.0)).truncate().normalize()
    }

    /// Gets the up vector in world coordinates.
    pub fn up_wc(&self) -> DVec3 {
        (self.transform * self.up.extend(0.0)).truncate().normalize()
    }

    /// Gets the right vector in world coordinates.
    pub fn right_wc(&self) -> DVec3 {
        (self.transform * self.right.extend(0.0)).truncate().normalize()
    }

    /// Transforms a point from world coordinates to camera reference frame.
    /// Maps to `Camera.worldToCameraCoordinatesPoint`
    pub fn world_to_camera_point(&self, point: DVec3) -> DVec3 {
        (self.transform.inverse() * point.extend(1.0)).truncate()
    }

    /// Transforms a vector from world coordinates to camera reference frame.
    /// Maps to `Camera.worldToCameraCoordinatesVector`
    pub fn world_to_camera_vector(&self, vector: DVec3) -> DVec3 {
        (self.transform.inverse() * vector.extend(0.0)).truncate()
    }

    /// Transforms a point from camera reference frame to world coordinates.
    /// Maps to `Camera.cameraToWorldCoordinatesPoint`
    pub fn camera_to_world_point(&self, point: DVec3) -> DVec3 {
        (self.transform * point.extend(1.0)).truncate()
    }

    /// Transforms a vector from camera reference frame to world coordinates.
    /// Maps to `Camera.cameraToWorldCoordinatesVector`
    pub fn camera_to_world_vector(&self, vector: DVec3) -> DVec3 {
        (self.transform * vector.extend(0.0)).truncate()
    }

    // ========================================================================
    // setView / lookAt / lookAtTransform
    // ========================================================================

    /// Sets the camera view with destination and orientation.
    /// Maps to `Camera.setView`
    ///
    /// # Arguments
    /// * `destination` - Target position (ECEF) or computed from rectangle
    /// * `heading` - Heading in radians (default 0)
    /// * `pitch` - Pitch in radians (default -PI/2 = looking down)
    /// * `roll` - Roll in radians (default 0)
    /// * `ellipsoid` - The ellipsoid
    pub fn set_view(
        &mut self,
        destination: DVec3,
        heading: f64,
        pitch: f64,
        roll: f64,
        ellipsoid: &Ellipsoid,
    ) {
        if self.mode == SceneMode::Morphing {
            return;
        }
        self.set_view_hpr(destination, heading, pitch, roll, ellipsoid);
    }

    /// Sets the camera to view a rectangle.
    /// Computes the camera position needed to view the given rectangle.
    /// Maps to `Camera.setView` with Rectangle destination
    pub fn set_view_rectangle(
        &mut self,
        rectangle: &Rectangle,
        ellipsoid: &Ellipsoid,
    ) {
        let position = self.get_rectangle_camera_coordinates(rectangle, ellipsoid);
        self.position = position;
        self.direction = -position.normalize();
        self.right = self.direction.cross(DVec3::Z).normalize();
        if self.right.length_squared() < 1e-10 {
            self.right = DVec3::X;
        }
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Sets the camera to look at a target with a HeadingPitchRange offset.
    /// Maps to `Camera.lookAt`
    pub fn look_at(&mut self, target: DVec3, offset: &HeadingPitchRange, ellipsoid: &Ellipsoid) {
        let transform = cesium_geospatial::transforms::east_north_up_to_fixed_frame(target, ellipsoid);
        self.look_at_transform(transform, offset);
    }

    /// Sets the camera to look at a target with a Cartesian3 offset.
    /// Maps to `Camera.lookAt` with Cartesian3 offset
    pub fn look_at_offset(&mut self, target: DVec3, offset: DVec3) {
        let transform = DMat4::from_translation(target);
        self.transform = transform;
        self.position = offset;
        self.direction = -offset.normalize();
        self.right = self.direction.cross(DVec3::Z).normalize();
        if self.right.length_squared() < 1e-10 {
            self.right = DVec3::X;
        }
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Sets the camera transform and positions it relative to the new frame.
    /// Maps to `Camera.lookAtTransform`
    pub fn look_at_transform(&mut self, transform: DMat4, offset: &HeadingPitchRange) {
        self.set_transform(transform);

        // Convert HeadingPitchRange to Cartesian offset in local frame
        let cartesian_offset = offset_from_heading_pitch_range(
            offset.heading,
            offset.pitch,
            offset.range,
        );

        self.position = cartesian_offset;
        self.direction = -cartesian_offset.normalize();
        self.right = self.direction.cross(DVec3::Z).normalize();
        if self.right.length_squared() < 1e-10 {
            self.right = DVec3::X;
        }
        self.up = self.right.cross(self.direction).normalize();
    }

    // ========================================================================
    // Rectangle viewing
    // ========================================================================

    /// Computes the camera position needed to view a rectangle.
    /// Maps to `Camera.getRectangleCameraCoordinates`
    pub fn get_rectangle_camera_coordinates(
        &self,
        rectangle: &Rectangle,
        ellipsoid: &Ellipsoid,
    ) -> DVec3 {
        // Compute the center of the rectangle
        let center_lon = (rectangle.west + rectangle.east) * 0.5;
        let center_lat = (rectangle.south + rectangle.north) * 0.5;
        let center = Cartographic::from_radians(center_lon, center_lat, 0.0);
        let center_ecef = ellipsoid.cartographic_to_cartesian(&center);

        // Compute the angular extent
        let delta_lon = (rectangle.east - rectangle.west).abs();
        let delta_lat = (rectangle.north - rectangle.south).abs();
        let max_delta = delta_lon.max(delta_lat);

        // Compute distance needed to view the rectangle
        let fov = match &self.frustum {
            Frustum::Perspective(f) => f.fov,
            Frustum::Orthographic(_) => std::f64::consts::FRAC_PI_3,
        };
        let half_angle = fov * 0.5;
        let arc_length = max_delta * ellipsoid.maximum_radius();
        let distance = (arc_length * 0.5) / half_angle.tan().max(0.001);

        // Position camera above the center
        let normal = center_ecef.normalize();
        center_ecef + normal * distance.max(ellipsoid.maximum_radius() * 0.1)
    }

    // ========================================================================
    // Bounding sphere utilities
    // ========================================================================

    /// Computes the distance from the camera to a bounding sphere.
    /// Maps to `Camera.distanceToBoundingSphere`
    pub fn distance_to_bounding_sphere(&self, sphere: &BoundingSphere) -> f64 {
        let to_center = self.position - sphere.center;
        let proj = self.direction * to_center.dot(self.direction);
        (proj.length() - sphere.radius).max(0.0)
    }

    /// Gets the magnitude of the camera position based on mode.
    /// Maps to `Camera.getMagnitude`
    pub fn get_magnitude(&self) -> f64 {
        match self.mode {
            SceneMode::Scene3D => self.position.length(),
            SceneMode::ColumbusView => self.position.z.abs(),
            SceneMode::Scene2D => 1.0, // Simplified: would use frustum extent
            SceneMode::Morphing => self.position.length(),
        }
    }

    // ========================================================================
    // Constrained rotation
    // ========================================================================

    /// Rotates with constrained axis enforcement.
    /// If constrained_axis is set, prevents the up vector from crossing it.
    pub fn rotate_constrained(&mut self, axis: DVec3, angle: f64) {
        self.rotate(axis, angle);

        if let Some(constrained) = self.constrained_axis {
            // If up vector crosses the constrained axis, clamp it
            let dot = self.up.dot(constrained);
            if dot < 0.0 {
                // Project up onto the plane perpendicular to constrained axis
                let projected = (self.up - constrained * dot).normalize();
                self.up = projected;
                self.right = self.direction.cross(self.up).normalize();
                self.up = self.right.cross(self.direction).normalize();
            }
        }
    }

    // ========================================================================
    // Change detection
    // ========================================================================

    /// Checks if the camera has changed significantly from a reference state.
    /// Returns the change percentage (0..1+) if changed beyond threshold.
    /// Maps to `Camera._updateCameraChanged`
    pub fn compute_change_percentage(
        &self,
        reference_position: DVec3,
        reference_direction: DVec3,
    ) -> f64 {
        // Direction change percentage
        let dir_angle = self.direction.dot(reference_direction).clamp(-1.0, 1.0).acos();
        let fov = match &self.frustum {
            Frustum::Perspective(f) => f.fov,
            Frustum::Orthographic(_) => 1.0,
        };
        let dir_percentage = if fov > 0.0 { dir_angle / (fov * 0.5) } else { dir_angle };

        // Position change percentage (relative to height)
        let distance = (self.position - reference_position).length();
        let height = self.position.length().max(1.0);
        let height_percentage = distance / height;

        dir_percentage.max(height_percentage)
    }

    /// Checks if the camera has changed beyond the percentage_changed threshold.
    pub fn has_changed(
        &self,
        reference_position: DVec3,
        reference_direction: DVec3,
    ) -> bool {
        self.compute_change_percentage(reference_position, reference_direction)
            > self.percentage_changed
    }

    // ========================================================================
    // Fly home
    // ========================================================================

    /// Returns the default "home" camera position for viewing the default rectangle.
    /// Maps to `Camera.flyHome` destination computation
    pub fn default_home_position(ellipsoid: &Ellipsoid) -> DVec3 {
        // Default view rectangle: roughly North America
        let default_rect = Rectangle::new(
            math_utils::to_radians(-95.0),
            math_utils::to_radians(-20.0),
            math_utils::to_radians(-70.0),
            math_utils::to_radians(90.0),
        );
        let center_lon = (default_rect.west + default_rect.east) * 0.5;
        let center_lat = (default_rect.south + default_rect.north) * 0.5;
        let center = Cartographic::from_radians(center_lon, center_lat, 0.0);
        let center_ecef = ellipsoid.cartographic_to_cartesian(&center);

        // Position at ~2.5x Earth radius above center
        let normal = center_ecef.normalize();
        normal * ellipsoid.maximum_radius() * 2.5
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

/// Converts a HeadingPitchRange offset to a Cartesian3 offset in local ENU frame.
/// Maps to `offsetFromHeadingPitchRange` in Camera.js
fn offset_from_heading_pitch_range(heading: f64, pitch: f64, range: f64) -> DVec3 {
    let cos_pitch = pitch.cos();
    let x = range * cos_pitch * heading.cos();
    let y = range * cos_pitch * heading.sin();
    let z = range * pitch.sin();
    DVec3::new(x, y, z)
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

    // ========================================================================
    // P4: New camera tests
    // ========================================================================

    #[test]
    fn test_scene_mode_default() {
        let camera = Camera::default_camera();
        assert_eq!(camera.mode, SceneMode::Scene3D);
    }

    #[test]
    fn test_easing_functions() {
        // Linear
        assert!((EasingFunction::Linear.evaluate(0.0)).abs() < 1e-10);
        assert!((EasingFunction::Linear.evaluate(0.5) - 0.5).abs() < 1e-10);
        assert!((EasingFunction::Linear.evaluate(1.0) - 1.0).abs() < 1e-10);

        // SinusoidalInOut
        assert!((EasingFunction::SinusoidalInOut.evaluate(0.0)).abs() < 1e-10);
        assert!((EasingFunction::SinusoidalInOut.evaluate(0.5) - 0.5).abs() < 1e-10);
        assert!((EasingFunction::SinusoidalInOut.evaluate(1.0) - 1.0).abs() < 1e-10);

        // QuadraticIn
        assert!((EasingFunction::QuadraticIn.evaluate(0.5) - 0.25).abs() < 1e-10);

        // QuadraticOut
        assert!((EasingFunction::QuadraticOut.evaluate(0.5) - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_position_wc_identity_transform() {
        let camera = Camera::new(
            DVec3::new(1000.0, 2000.0, 3000.0),
            -DVec3::Z,
            DVec3::Y,
        );
        // With identity transform, position_wc == position
        assert!(camera.position_wc().abs_diff_eq(camera.position, 1e-10));
    }

    #[test]
    fn test_set_transform() {
        let mut camera = Camera::new(
            DVec3::new(100.0, 0.0, 0.0),
            -DVec3::X,
            DVec3::Z,
        );
        // Set a translation transform
        let transform = DMat4::from_translation(DVec3::new(50.0, 0.0, 0.0));
        camera.set_transform(transform);

        // Position in local frame should be offset by -50 in x
        assert!((camera.position.x - 50.0).abs() < 1e-10);
        // World position should still be 100
        assert!((camera.position_wc().x - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_world_to_camera_roundtrip() {
        let mut camera = Camera::new(
            DVec3::new(100.0, 200.0, 300.0),
            -DVec3::Z,
            DVec3::Y,
        );
        camera.transform = DMat4::from_translation(DVec3::new(10.0, 20.0, 30.0));

        let world_point = DVec3::new(500.0, 600.0, 700.0);
        let local = camera.world_to_camera_point(world_point);
        let back = camera.camera_to_world_point(local);
        assert!(back.abs_diff_eq(world_point, 1e-6));
    }

    #[test]
    fn test_look_at_with_hpr() {
        let mut camera = Camera::default_camera();
        let target = DVec3::new(6378137.0, 0.0, 0.0);
        let offset = HeadingPitchRange::new(0.0, -PI / 4.0, 1000000.0);

        camera.look_at(target, &offset, &Ellipsoid::WGS84);

        // Camera should be positioned at range from target
        let world_pos = camera.position_wc();
        let dist = (world_pos - target).length();
        assert!((dist - 1000000.0).abs() / 1000000.0 < 0.01);
    }

    #[test]
    fn test_look_at_offset() {
        let mut camera = Camera::default_camera();
        let target = DVec3::new(0.0, 0.0, 0.0);
        let offset = DVec3::new(1000.0, 0.0, 500.0);

        camera.look_at_offset(target, offset);

        // Direction should point from offset toward target
        let expected_dir = -offset.normalize();
        assert!(camera.direction.abs_diff_eq(expected_dir, 1e-10));
    }

    #[test]
    fn test_get_rectangle_camera_coordinates() {
        let camera = Camera::default_camera();
        let rect = Rectangle::new(
            math_utils::to_radians(-10.0),
            math_utils::to_radians(-10.0),
            math_utils::to_radians(10.0),
            math_utils::to_radians(10.0),
        );

        let pos = camera.get_rectangle_camera_coordinates(&rect, &Ellipsoid::WGS84);

        // Position should be above the surface
        let height = pos.length() - Ellipsoid::WGS84.maximum_radius();
        assert!(height > 0.0);
    }

    #[test]
    fn test_set_view_rectangle() {
        let mut camera = Camera::default_camera();
        let rect = Rectangle::new(
            math_utils::to_radians(-10.0),
            math_utils::to_radians(-10.0),
            math_utils::to_radians(10.0),
            math_utils::to_radians(10.0),
        );

        camera.set_view_rectangle(&rect, &Ellipsoid::WGS84);

        // Camera should be above the surface looking down
        let height = camera.position.length() - Ellipsoid::WGS84.maximum_radius();
        assert!(height > 0.0);
        // Direction should have a component toward center
        assert!(camera.direction.dot(-camera.position.normalize()) > 0.5);
    }

    #[test]
    fn test_distance_to_bounding_sphere() {
        let camera = Camera::new(
            DVec3::new(0.0, 0.0, 1000.0),
            -DVec3::Z,
            DVec3::Y,
        );
        let sphere = BoundingSphere::new(DVec3::new(0.0, 0.0, 0.0), 100.0);

        let dist = camera.distance_to_bounding_sphere(&sphere);
        // Distance should be ~900 (1000 - 100)
        assert!((dist - 900.0).abs() < 1.0);
    }

    #[test]
    fn test_get_magnitude() {
        let mut camera = Camera::new(
            DVec3::new(6378137.0 * 2.0, 0.0, 0.0),
            -DVec3::X,
            DVec3::Z,
        );
        camera.mode = SceneMode::Scene3D;
        let mag = camera.get_magnitude();
        assert!((mag - 6378137.0 * 2.0).abs() < 1.0);
    }

    #[test]
    fn test_constrained_rotation() {
        let mut camera = Camera::default_camera();
        camera.constrained_axis = Some(DVec3::Z);

        // Rotate significantly - constrained axis should prevent up from going below
        camera.rotate_constrained(DVec3::X, PI * 0.8);

        // Up should still have positive Z component (not crossed constrained axis)
        assert!(camera.up.dot(DVec3::Z) >= -1e-10);
    }

    #[test]
    fn test_change_detection() {
        let camera = Camera::new(
            DVec3::new(6378137.0 * 2.0, 0.0, 0.0),
            -DVec3::X,
            DVec3::Z,
        );

        // Same state = no change
        let pct = camera.compute_change_percentage(camera.position, camera.direction);
        assert!(pct < 0.01);

        // Very different position = significant change
        let pct = camera.compute_change_percentage(
            DVec3::new(6378137.0 * 3.0, 0.0, 0.0),
            camera.direction,
        );
        assert!(pct > 0.1);
    }

    #[test]
    fn test_has_changed() {
        let camera = Camera::new(
            DVec3::new(6378137.0 * 2.0, 0.0, 0.0),
            -DVec3::X,
            DVec3::Z,
        );

        // No change
        assert!(!camera.has_changed(camera.position, camera.direction));

        // Big direction change
        assert!(camera.has_changed(camera.position, DVec3::X));
    }

    #[test]
    fn test_default_home_position() {
        let pos = Camera::default_home_position(&Ellipsoid::WGS84);
        // Should be at ~2.5x Earth radius
        let mag = pos.length();
        assert!((mag - Ellipsoid::WGS84.maximum_radius() * 2.5).abs() / mag < 0.01);
    }

    #[test]
    fn test_offset_from_heading_pitch_range() {
        // Heading=0, Pitch=0, Range=1000 -> offset along X
        let offset = offset_from_heading_pitch_range(0.0, 0.0, 1000.0);
        assert!((offset.x - 1000.0).abs() < 1e-10);
        assert!(offset.y.abs() < 1e-10);
        assert!(offset.z.abs() < 1e-10);

        // Heading=0, Pitch=PI/2, Range=1000 -> offset along Z (up)
        let offset = offset_from_heading_pitch_range(0.0, PI / 2.0, 1000.0);
        assert!(offset.x.abs() < 1e-6);
        assert!((offset.z - 1000.0).abs() < 1e-6);
    }
}
