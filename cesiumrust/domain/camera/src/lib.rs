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

    /// Gets the heading angle in radians (simplified for CV/2D mode).
    /// Maps to `Camera.heading`
    pub fn heading(&self) -> f64 {
        get_heading(self.direction, self.up)
    }

    /// Gets the heading in 3D mode using ENU frame at camera position.
    /// Maps to `Camera.heading` when mode is SCENE3D
    pub fn heading_3d(&self, ellipsoid: &Ellipsoid) -> f64 {
        get_heading_3d(self.position, self.direction, self.up, self.right, ellipsoid)
    }

    /// Gets the pitch angle in radians (simplified for CV/2D mode).
    /// Maps to `Camera.pitch`
    pub fn pitch(&self) -> f64 {
        get_pitch(self.direction)
    }

    /// Gets the pitch in 3D mode using surface normal at camera position.
    /// Maps to `Camera.pitch` when mode is SCENE3D
    pub fn pitch_3d(&self, ellipsoid: &Ellipsoid) -> f64 {
        get_pitch_3d(self.position, self.direction, ellipsoid)
    }

    /// Gets the roll angle in radians (simplified for CV/2D mode).
    /// Maps to `Camera.roll`
    pub fn roll(&self) -> f64 {
        get_roll(self.direction, self.up, self.right)
    }

    /// Gets the roll in 3D mode using ENU frame at camera position.
    /// Maps to `Camera.roll` when mode is SCENE3D
    pub fn roll_3d(&self, ellipsoid: &Ellipsoid) -> f64 {
        get_roll_3d(self.position, self.direction, self.up, self.right, ellipsoid)
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

    /// Rotates the camera around an axis by an angle (orbits position + rotates orientation).
    /// Maps to `Camera.rotate`
    pub fn rotate(&mut self, axis: DVec3, angle: f64) {
        let axis = axis.normalize();
        // CesiumJS negates the angle: Quaternion.fromAxisAngle(axis, -angle)
        let rotation = glam::DQuat::from_axis_angle(axis, -angle);
        self.position = rotation * self.position;
        self.direction = (rotation * self.direction).normalize();
        self.up = (rotation * self.up).normalize();
        self.right = self.direction.cross(self.up).normalize();
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Rotates the camera upward (orbits around right axis).
    /// Maps to `Camera.rotateUp` which calls rotateVertical(this, -angle)
    pub fn rotate_up(&mut self, angle: f64) {
        let axis = self.right;
        self.rotate(axis, -angle);
    }

    /// Rotates the camera downward (orbits around right axis).
    /// Maps to `Camera.rotateDown` which calls rotateVertical(this, angle)
    pub fn rotate_down(&mut self, angle: f64) {
        let axis = self.right;
        self.rotate(axis, angle);
    }

    /// Rotates the camera to the left (orbits around up axis).
    /// Maps to `Camera.rotateLeft`
    pub fn rotate_left(&mut self, angle: f64) {
        let axis = self.up;
        self.rotate(axis, angle);
    }

    /// Rotates the camera to the right (orbits around up axis, negative angle).
    /// Maps to `Camera.rotateRight`
    pub fn rotate_right(&mut self, angle: f64) {
        let axis = self.up;
        self.rotate(axis, -angle);
    }

    /// Looks along the given axis by an angle (rotates direction and up only, no position change).
    /// Maps to `Camera.look`
    pub fn look(&mut self, axis: DVec3, angle: f64) {
        // CesiumJS negates the angle: Quaternion.fromAxisAngle(axis, -angle)
        let rotation = glam::DQuat::from_axis_angle(axis.normalize(), -angle);
        self.direction = (rotation * self.direction).normalize();
        self.up = (rotation * self.up).normalize();
        self.right = self.direction.cross(self.up).normalize();
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Looks left by the given angle.
    /// Maps to `Camera.lookLeft`
    pub fn look_left(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_look_amount);
        let axis = self.up;
        self.look(axis, -angle);
    }

    /// Looks right by the given angle.
    /// Maps to `Camera.lookRight`
    pub fn look_right(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_look_amount);
        let axis = self.up;
        self.look(axis, angle);
    }

    /// Looks up by the given angle.
    /// Maps to `Camera.lookUp`
    pub fn look_up(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_look_amount);
        let axis = self.right;
        self.look(axis, -angle);
    }

    /// Looks down by the given angle.
    /// Maps to `Camera.lookDown`
    pub fn look_down(&mut self, angle: Option<f64>) {
        let angle = angle.unwrap_or(self.default_look_amount);
        let axis = self.right;
        self.look(axis, angle);
    }

    /// Twists the camera left (rolls counterclockwise).
    /// Maps to `Camera.twistLeft`
    pub fn twist_left(&mut self, angle: f64) {
        let axis = self.direction;
        self.look(axis, angle);
    }

    /// Twists the camera right (rolls clockwise).
    /// Maps to `Camera.twistRight`
    pub fn twist_right(&mut self, angle: f64) {
        let axis = self.direction;
        self.look(axis, -angle);
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
    /// Faithfully maps to CesiumJS `setView3D`:
    /// 1. Compute ENU at position
    /// 2. Adjust heading: heading -= PI/2 (so heading=0 means North)
    /// 3. Compute quaternion from adjusted HPR
    /// 4. direction = rotMat column 0, up = rotMat column 2
    /// 5. Transform from ENU-local to world
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
        let up_enu = enu.z_axis.truncate();

        // CesiumJS setView3D line 1285: hpr.heading = hpr.heading - PI_OVER_TWO
        let adjusted_heading = heading - std::f64::consts::FRAC_PI_2;
        let hpr_quat = HeadingPitchRoll::new(adjusted_heading, pitch, roll).to_quaternion();

        // CesiumJS: direction = Matrix3.getColumn(rotMat, 0) = quat * X
        //           up = Matrix3.getColumn(rotMat, 2) = quat * Z
        let local_direction = hpr_quat * DVec3::X;
        let local_up = hpr_quat * DVec3::Z;

        // Transform from ENU-local to world
        let enu_rotation = glam::DMat3::from_cols(east, north, up_enu);
        self.direction = (enu_rotation * local_direction).normalize();
        self.up = (enu_rotation * local_up).normalize();
        self.right = self.direction.cross(self.up).normalize();
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Sets the camera position and orientation from direction/up vectors.
    /// Maps to CesiumJS `setView3D` with orientation.direction + orientation.up
    pub fn set_view_direction(
        &mut self,
        position: DVec3,
        direction: DVec3,
        up: DVec3,
    ) {
        self.position = position;
        self.direction = direction.normalize();
        self.up = up.normalize();
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

    /// Transforms a Cartesian4 from world coordinates to camera reference frame.
    /// Maps to `Camera.worldToCameraCoordinates`
    pub fn world_to_camera_coordinates(&self, cartesian: glam::DVec4) -> glam::DVec4 {
        self.transform.inverse() * cartesian
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

    /// Transforms a Cartesian4 from camera reference frame to world coordinates.
    /// Maps to `Camera.cameraToWorldCoordinates`
    pub fn camera_to_world_coordinates(&self, cartesian: glam::DVec4) -> glam::DVec4 {
        self.transform * cartesian
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
    pub fn look_at_offset(&mut self, target: DVec3, offset: DVec3, ellipsoid: &Ellipsoid) {
        let transform = cesium_geospatial::transforms::east_north_up_to_fixed_frame(target, ellipsoid);
        self.transform = transform;
        self.position = offset;
        self.direction = -offset.normalize();
        let right = self.direction.cross(DVec3::Z);
        if right.length_squared() < 1e-10 {
            self.right = DVec3::X;
        } else {
            self.right = right.normalize();
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
        let right = self.direction.cross(DVec3::Z);
        if right.length_squared() < 1e-10 {
            self.right = DVec3::X;
        } else {
            self.right = right.normalize();
        }
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Sets the camera transform and positions it with a Cartesian3 offset.
    /// Maps to `Camera.lookAtTransform` with Cartesian3 offset
    pub fn look_at_transform_offset(&mut self, transform: DMat4, offset: DVec3) {
        self.set_transform(transform);
        self.position = offset;
        self.direction = -offset.normalize();
        let right = self.direction.cross(DVec3::Z);
        if right.length_squared() < 1e-10 {
            self.right = DVec3::X;
        } else {
            self.right = right.normalize();
        }
        self.up = self.right.cross(self.direction).normalize();
    }

    /// Sets the camera transform, preserving current world-space position/orientation.
    /// Maps to `Camera.lookAtTransform` with no offset
    pub fn look_at_transform_no_offset(&mut self, transform: DMat4) {
        self.set_transform(transform);
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
        // Maps to CesiumJS Camera.distanceToBoundingSphere:
        // signed distance along view direction minus sphere radius.
        let to_center = sphere.center - self.position;
        let distance = to_center.dot(self.direction) - sphere.radius;
        distance.max(0.0)
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

    /// Gets the inverse of the reference frame transform.
    /// Maps to `Camera.inverseTransform`
    pub fn inverse_transform(&self) -> DMat4 {
        self.transform.inverse()
    }

    // ========================================================================
    // Picking
    // ========================================================================

    /// Creates a pick ray from a window position using perspective frustum.
    /// Maps to `Camera.getPickRay` (perspective branch)
    ///
    /// # Arguments
    /// * `window_x` - Window X coordinate (pixels, left=0)
    /// * `window_y` - Window Y coordinate (pixels, top=0)
    /// * `canvas_width` - Canvas width in pixels
    /// * `canvas_height` - Canvas height in pixels
    pub fn get_pick_ray_perspective(
        &self,
        window_x: f64,
        window_y: f64,
        canvas_width: f64,
        canvas_height: f64,
    ) -> Option<cesium_geospatial::Ray> {
        let (fovy, aspect_ratio, near) = match &self.frustum {
            Frustum::Perspective(f) => (f.fovy(), f.aspect_ratio, f.near),
            Frustum::Orthographic(_) => return None, // Use orthographic version
        };

        let tan_phi = (fovy * 0.5).tan();
        let tan_theta = aspect_ratio * tan_phi;

        // NDC coordinates
        let x = (2.0 / canvas_width) * window_x - 1.0;
        let y = (2.0 / canvas_height) * (canvas_height - window_y) - 1.0;

        let position = self.position_wc();
        let direction_wc = self.direction_wc();
        let right_wc = self.right_wc();
        let up_wc = self.up_wc();

        // direction = normalize(dir*near + right*(x*near*tanTheta) + up*(y*near*tanPhi))
        let dir = direction_wc * near
            + right_wc * (x * near * tan_theta)
            + up_wc * (y * near * tan_phi);

        Some(cesium_geospatial::Ray::new(position, dir.normalize()))
    }

    /// Picks the ellipsoid surface at a window position.
    /// Maps to `Camera.pickEllipsoid` (3D branch)
    ///
    /// Returns the intersection point on the ellipsoid, or None if not visible.
    pub fn pick_ellipsoid(
        &self,
        window_x: f64,
        window_y: f64,
        canvas_width: f64,
        canvas_height: f64,
        ellipsoid: &Ellipsoid,
    ) -> Option<DVec3> {
        let ray = self.get_pick_ray_perspective(window_x, window_y, canvas_width, canvas_height)?;
        let intersection = cesium_geospatial::ray_ellipsoid(&ray, ellipsoid)?;
        let t = if intersection.0 > 0.0 { intersection.0 } else { intersection.1 };
        Some(ray.point_at(t))
    }

    /// Creates a pick ray from a window position using orthographic frustum.
    /// Maps to `Camera.getPickRay` (orthographic branch)
    ///
    /// For orthographic projections, the ray origin is offset by the window position
    /// in the frustum plane, and the direction is always the camera direction.
    pub fn get_pick_ray_orthographic(
        &self,
        window_x: f64,
        window_y: f64,
        canvas_width: f64,
        canvas_height: f64,
    ) -> Option<cesium_geospatial::Ray> {
        let (frustum_width, frustum_height) = match &self.frustum {
            Frustum::Orthographic(f) => (f.width, f.height()),
            Frustum::Perspective(_) => return None,
        };

        // NDC coordinates scaled by frustum half-extents
        let mut x = (2.0 / canvas_width) * window_x - 1.0;
        x *= frustum_width * 0.5;
        let mut y = (2.0 / canvas_height) * (canvas_height - window_y) - 1.0;
        y *= frustum_height * 0.5;

        let position = self.position_wc();
        let right_wc = self.right_wc();
        let up_wc = self.up_wc();
        let direction_wc = self.direction_wc();

        let origin = position + right_wc * x + up_wc * y;

        Some(cesium_geospatial::Ray::new(origin, direction_wc))
    }

    /// Creates a pick ray from a window position (dispatches to perspective or orthographic).
    /// Maps to `Camera.getPickRay`
    pub fn get_pick_ray(
        &self,
        window_x: f64,
        window_y: f64,
        canvas_width: f64,
        canvas_height: f64,
    ) -> Option<cesium_geospatial::Ray> {
        match &self.frustum {
            Frustum::Perspective(_) => self.get_pick_ray_perspective(window_x, window_y, canvas_width, canvas_height),
            Frustum::Orthographic(_) => self.get_pick_ray_orthographic(window_x, window_y, canvas_width, canvas_height),
        }
    }

    /// Computes the pixel size of a bounding sphere at its distance from the camera.
    /// Maps to `Camera.getPixelSize`
    ///
    /// Returns the maximum pixel dimension (width or height) of one pixel
    /// at the distance to the bounding sphere.
    pub fn get_pixel_size(
        &self,
        sphere: &BoundingSphere,
        drawing_buffer_width: f64,
        drawing_buffer_height: f64,
        pixel_ratio: f64,
    ) -> f64 {
        let distance = self.distance_to_bounding_sphere(sphere);
        let (pixel_width, pixel_height) = match &self.frustum {
            Frustum::Perspective(f) => f.pixel_dimensions(drawing_buffer_width, drawing_buffer_height, distance, pixel_ratio),
            Frustum::Orthographic(f) => f.pixel_dimensions(drawing_buffer_width, drawing_buffer_height, distance, pixel_ratio),
        };
        pixel_width.max(pixel_height)
    }

    // ========================================================================
    // Constrained rotation
    // ========================================================================

    /// Rotates with constrained axis enforcement.
    /// If constrained_axis is set, prevents the up vector from crossing it.
    /// Maps to `Camera._rotateConstrained`
    pub fn rotate_constrained(&mut self, axis: DVec3, angle: f64) {
        self.rotate(axis, angle);

        if let Some(constrained) = self.constrained_axis {
            // If up vector crosses the constrained axis, clamp it
            let dot = self.up.dot(constrained);
            if dot < 0.0 {
                // Project up onto the plane perpendicular to constrained axis
                let projected = (self.up - constrained * dot).normalize();
                self.up = projected;
                // Re-derive direction to be perpendicular to clamped up
                self.direction = (self.direction - self.up * self.direction.dot(self.up)).normalize();
                self.right = self.direction.cross(self.up).normalize();
            }
        }
    }

    /// Rotates up with constrained axis enforcement.
    pub fn rotate_up_constrained(&mut self, angle: f64) {
        let axis = self.right;
        self.rotate_constrained(axis, -angle);
    }

    /// Rotates down with constrained axis enforcement.
    pub fn rotate_down_constrained(&mut self, angle: f64) {
        let axis = self.right;
        self.rotate_constrained(axis, angle);
    }

    /// Rotates left with constrained axis enforcement.
    pub fn rotate_left_constrained(&mut self, angle: f64) {
        let axis = self.up;
        self.rotate_constrained(axis, angle);
    }

    /// Rotates right with constrained axis enforcement.
    pub fn rotate_right_constrained(&mut self, angle: f64) {
        let axis = self.up;
        self.rotate_constrained(axis, -angle);
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
    let result = math_utils::TWO_PI - math_utils::zero_to_two_pi(heading);
    // Normalize: TWO_PI ≡ 0 (heading range is [0, TWO_PI))
    if (result - math_utils::TWO_PI).abs() < 1e-15 {
        0.0
    } else {
        result
    }
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
///
/// Faithfully maps to CesiumJS `offsetFromHeadingPitchRange`:
/// 1. Clamp pitch to [-PI/2, PI/2]
/// 2. heading = zeroToTwoPi(heading) - PI/2
/// 3. pitchQuat = fromAxisAngle(Y, -pitch)
/// 4. headingQuat = fromAxisAngle(Z, -heading)
/// 5. rotQuat = headingQuat * pitchQuat
/// 6. offset = -(rotMatrix * UNIT_X) * range
///
/// Equivalent closed-form: uses quaternion product directly (matching CesiumJS).
fn offset_from_heading_pitch_range(heading: f64, pitch: f64, range: f64) -> DVec3 {
    let pitch = pitch.clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
    // CesiumJS: heading = zeroToTwoPi(heading) - PI_OVER_TWO
    let heading = math_utils::zero_to_two_pi(heading) - std::f64::consts::FRAC_PI_2;
    // pitchQuat = fromAxisAngle(Y, -pitch), headingQuat = fromAxisAngle(Z, -heading)
    // rotQuat = headingQuat * pitchQuat
    let sp = (-pitch / 2.0).sin();
    let cp = (-pitch / 2.0).cos();
    let sh = (-heading / 2.0).sin();
    let ch = (-heading / 2.0).cos();
    // rotQuat = (ch*0 - sh*sp, ch*sp + sh*0, ch*0 - sh*0... )
    let qx = -sh * sp;
    let qy = ch * sp;
    let qz = sh * cp;
    let qw = ch * cp;
    // rotMatrix column 0 (direction = rotMat * UNIT_X)
    let m00 = qw * qw + qx * qx - qy * qy - qz * qz;
    let m10 = 2.0 * (qx * qy + qw * qz);
    let m20 = 2.0 * (qx * qz - qw * qy);
    // offset = -(rotMat * UNIT_X) * range
    DVec3::new(-m00 * range, -m10 * range, -m20 * range)
}

/// Computes heading in 3D mode using ENU frame at camera position.
/// Maps to CesiumJS Camera.heading getter (SCENE3D branch):
/// Transform direction/up to ENU local frame, then:
/// heading = TWO_PI - zeroToTwoPi(atan2(dir_local.y, dir_local.x) - PI/2)
/// If |dir_local.z| ≈ 1 (looking straight up/down), use up_local instead.
fn get_heading_3d(position: DVec3, direction: DVec3, up: DVec3, _right: DVec3, ellipsoid: &Ellipsoid) -> f64 {
    let enu = cesium_geospatial::transforms::east_north_up_to_fixed_frame(position, ellipsoid);
    let east = enu.x_axis.truncate();
    let north = enu.y_axis.truncate();
    let up_enu = enu.z_axis.truncate();

    // Transform direction to ENU local frame
    let dir_local = DVec3::new(direction.dot(east), direction.dot(north), direction.dot(up_enu));

    let heading = if (dir_local.z.abs() - 1.0).abs() <= math_utils::EPSILON3 {
        // Looking nearly straight up or down - use up vector
        let up_local = DVec3::new(up.dot(east), up.dot(north), up.dot(up_enu));
        math_utils::TWO_PI - math_utils::zero_to_two_pi(up_local.y.atan2(up_local.x) - std::f64::consts::FRAC_PI_2)
    } else {
        math_utils::TWO_PI - math_utils::zero_to_two_pi(dir_local.y.atan2(dir_local.x) - std::f64::consts::FRAC_PI_2)
    };
    // Normalize: TWO_PI ≡ 0 (heading range is [0, TWO_PI))
    if (heading - math_utils::TWO_PI).abs() < 1e-15 {
        0.0
    } else {
        heading
    }
}

/// Computes pitch in 3D mode using ENU frame at camera position.
/// Maps to CesiumJS Camera.pitch getter (SCENE3D branch):
/// pitch = PI/2 - acosClamped(dir_local.z)
fn get_pitch_3d(position: DVec3, direction: DVec3, ellipsoid: &Ellipsoid) -> f64 {
    let enu = cesium_geospatial::transforms::east_north_up_to_fixed_frame(position, ellipsoid);
    let up_enu = enu.z_axis.truncate();

    // dir_local.z = direction dot ENU up axis (= geodetic surface normal)
    let dir_local_z = direction.dot(up_enu);
    std::f64::consts::FRAC_PI_2 - dir_local_z.clamp(-1.0, 1.0).acos()
}

/// Computes roll in 3D mode using ENU frame at camera position.
/// Maps to CesiumJS Camera.roll getter (SCENE3D branch):
/// If |dir_local.z| < 1-EPSILON3: roll = zeroToTwoPi(atan2(-right_local.z, up_local.z) + TWO_PI)
/// Otherwise: roll = 0
fn get_roll_3d(position: DVec3, direction: DVec3, up: DVec3, right: DVec3, ellipsoid: &Ellipsoid) -> f64 {
    let enu = cesium_geospatial::transforms::east_north_up_to_fixed_frame(position, ellipsoid);
    let up_enu = enu.z_axis.truncate();

    let dir_local_z = direction.dot(up_enu);

    let roll = if (dir_local_z.abs() - 1.0).abs() > math_utils::EPSILON3 {
        let right_local_z = right.dot(up_enu);
        let up_local_z = up.dot(up_enu);
        math_utils::zero_to_two_pi((-right_local_z).atan2(up_local_z) + math_utils::TWO_PI)
    } else {
        0.0
    };
    roll
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
        camera.rotate_right(PI / 2.0); // 90 degrees
        // After rotating 90° around up (Y) with negative angle:
        // direction (-Z) rotated by -PI/2 around Y -> -X
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

        // Heading should be 0 (looking north) using 3D getter
        let heading = camera.heading_3d(&ellipsoid);
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

        camera.look_at_offset(target, offset, &Ellipsoid::WGS84);

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
        camera.constrained_axis = Some(DVec3::Y);

        // Rotate around right axis (X) - constrained axis Y prevents up from crossing
        camera.rotate_constrained(DVec3::X, PI * 0.8);

        // Up should have non-negative Y component (not crossed constrained axis)
        assert!(camera.up.dot(DVec3::Y) >= -1e-10,
            "up.dot(Y) = {}", camera.up.dot(DVec3::Y));
        // Verify orthonormality maintained
        assert!(camera.direction.dot(camera.up).abs() < 1e-10);
        assert!(camera.direction.length() > 0.99);
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
        // Heading=0, Pitch=0, Range=1000 -> offset along -Y (south in ENU, camera looks north)
        // CesiumJS: heading adjusted to -PI/2, rotMatrix*X=(0,1,0), negate→(0,-1,0)
        let offset = offset_from_heading_pitch_range(0.0, 0.0, 1000.0);
        assert!(offset.x.abs() < 1e-10, "x={}", offset.x);
        assert!((offset.y + 1000.0).abs() < 1e-10, "y={}", offset.y);
        assert!(offset.z.abs() < 1e-10, "z={}", offset.z);

        // Heading=0, Pitch=PI/2, Range=1000 -> offset along -Z (below plane, camera looks up)
        let offset = offset_from_heading_pitch_range(0.0, PI / 2.0, 1000.0);
        assert!(offset.x.abs() < 1e-6, "x={}", offset.x);
        assert!(offset.y.abs() < 1e-6, "y={}", offset.y);
        assert!((offset.z + 1000.0).abs() < 1e-6, "z={}", offset.z);
    }
}
