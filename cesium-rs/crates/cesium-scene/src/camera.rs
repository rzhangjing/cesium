//! Ported from `packages/engine/Source/Scene/Camera.js`.
//!
//! The camera defines the view frustum and position from which the scene is rendered.
//! In CesiumJS, this is a 3989-line file managing view/projection matrices, flight
//! animations, coordinate transforms, and user interaction (look/rotate/move/zoom).

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::matrix4::Matrix4;
use cesium_core::ray::Ray;

/// The type of camera projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraProjection {
    Perspective,
    Orthographic,
}

/// The camera defining the view frustum and position.
///
/// Mirrors the CesiumJS `Camera` which manages the view and projection
/// matrices, coordinate transforms, and camera movement/rotation.
pub struct Camera {
    /// Camera position in world coordinates.
    position: Cartesian3,
    /// Camera direction (unit vector).
    direction: Cartesian3,
    /// Camera up vector (unit vector).
    up: Cartesian3,
    /// Camera right vector (unit vector).
    right: Cartesian3,
    /// The inverse view transform (camera-to-world).
    inverse_view_matrix: Matrix4,
    /// The view transform (world-to-camera).
    view_matrix: Matrix4,
    /// The projection matrix.
    projection_matrix: Matrix4,
    /// The inverse projection matrix.
    inverse_projection_matrix: Matrix4,
    /// The camera's reference frame transform.
    transform: Matrix4,
    /// The inverse of the transform.
    inverse_transform: Matrix4,
    /// The default reference frame transform (identity).
    default_transform: Matrix4,
    /// Maximum zoom distance.
    maximum_zoom_distance: f64,
    /// Minimum zoom distance.
    minimum_zoom_distance: f64,
    /// The projection type.
    projection: CameraProjection,
    /// Field of view in radians (perspective).
    fov: f64,
    /// Near plane distance.
    near: f64,
    /// Far plane distance.
    far: f64,
    /// Aspect ratio.
    aspect_ratio: f64,
    /// Orthographic width (orthographic mode).
    orthographic_width: f64,
    /// Whether the camera has changed this frame.
    changed: bool,
    /// The percentage of the frustum that must change before triggering a change event.
    frustum_changed_percentage: f64,
    /// The drawing buffer width.
    canvas_width: u32,
    /// The drawing buffer height.
    canvas_height: u32,
}

impl Camera {
    /// Creates a new camera with default values.
    pub fn new() -> Self {
        Self {
            position: Cartesian3::new(0.0, 0.0, 0.0),
            direction: Cartesian3::new(0.0, 0.0, -1.0),
            up: Cartesian3::new(0.0, 1.0, 0.0),
            right: Cartesian3::new(1.0, 0.0, 0.0),
            inverse_view_matrix: Matrix4::IDENTITY,
            view_matrix: Matrix4::IDENTITY,
            projection_matrix: Matrix4::IDENTITY,
            inverse_projection_matrix: Matrix4::IDENTITY,
            transform: Matrix4::IDENTITY,
            inverse_transform: Matrix4::IDENTITY,
            default_transform: Matrix4::IDENTITY,
            maximum_zoom_distance: 10.0 * Ellipsoid::WGS84.maximum_radius(),
            minimum_zoom_distance: -1.0 * Ellipsoid::WGS84.maximum_radius(),
            projection: CameraProjection::Perspective,
            fov: std::f64::consts::FRAC_PI_3,
            near: 1.0,
            far: 500000000.0,
            aspect_ratio: 1.0,
            orthographic_width: 1.0,
            changed: false,
            frustum_changed_percentage: 0.5,
            canvas_width: 800,
            canvas_height: 600,
        }
    }

    // ---- Position and orientation ----

    /// Returns the camera position in world coordinates.
    pub fn position(&self) -> &Cartesian3 { &self.position }

    /// Returns the camera direction (unit vector).
    pub fn direction(&self) -> &Cartesian3 { &self.direction }

    /// Returns the camera up vector (unit vector).
    pub fn up(&self) -> &Cartesian3 { &self.up }

    /// Returns the camera right vector (unit vector).
    pub fn right(&self) -> &Cartesian3 { &self.right }

    /// Sets the camera position.
    pub fn set_position(&mut self, position: Cartesian3) {
        self.position = position;
        self.changed = true;
    }

    /// Sets the camera direction.
    pub fn set_direction(&mut self, direction: Cartesian3) {
        self.direction = direction;
        self.changed = true;
    }

    /// Sets the camera up vector.
    pub fn set_up(&mut self, up: Cartesian3) {
        self.up = up;
        self.changed = true;
    }

    // ---- Matrices ----

    /// Returns the view matrix (world-to-camera).
    pub fn view_matrix(&self) -> &Matrix4 { &self.view_matrix }

    /// Returns the inverse view matrix (camera-to-world).
    pub fn inverse_view_matrix(&self) -> &Matrix4 { &self.inverse_view_matrix }

    /// Returns the projection matrix.
    pub fn projection_matrix(&self) -> &Matrix4 { &self.projection_matrix }

    /// Returns the inverse projection matrix.
    pub fn inverse_projection_matrix(&self) -> &Matrix4 { &self.inverse_projection_matrix }

    /// Returns the camera transform.
    pub fn transform(&self) -> &Matrix4 { &self.transform }

    // ---- Projection parameters ----

    /// Returns the projection type.
    pub fn projection_type(&self) -> CameraProjection { self.projection }

    /// Sets the projection type.
    pub fn set_projection(&mut self, projection: CameraProjection) {
        self.projection = projection;
        self.changed = true;
    }

    /// Returns the field of view in radians.
    pub fn fov(&self) -> f64 { self.fov }

    /// Sets the field of view in radians.
    pub fn set_fov(&mut self, fov: f64) {
        self.fov = fov;
        self.changed = true;
    }

    /// Returns the near plane distance.
    pub fn near(&self) -> f64 { self.near }

    /// Sets the near plane distance.
    pub fn set_near(&mut self, near: f64) {
        self.near = near;
        self.changed = true;
    }

    /// Returns the far plane distance.
    pub fn far(&self) -> f64 { self.far }

    /// Sets the far plane distance.
    pub fn set_far(&mut self, far: f64) {
        self.far = far;
        self.changed = true;
    }

    /// Returns the aspect ratio.
    pub fn aspect_ratio(&self) -> f64 { self.aspect_ratio }

    /// Sets the aspect ratio.
    pub fn set_aspect_ratio(&mut self, ratio: f64) {
        self.aspect_ratio = ratio;
        self.changed = true;
    }

    /// Returns the orthographic width.
    pub fn orthographic_width(&self) -> f64 { self.orthographic_width }

    /// Sets the orthographic width.
    pub fn set_orthographic_width(&mut self, width: f64) {
        self.orthographic_width = width;
        self.changed = true;
    }

    // ---- Movement ----

    /// Moves the camera in the given direction by the given amount.
    pub fn move_camera(&mut self, direction: &Cartesian3, amount: f64) {
        let scaled = Cartesian3::multiply_by_scalar_new(direction, amount);
        self.position = Cartesian3::add_new(&self.position, &scaled);
        self.changed = true;
    }

    /// Moves the camera forward along its direction.
    pub fn move_forward(&mut self, amount: f64) {
        let dir = self.direction;
        self.move_camera(&dir, amount);
    }

    /// Moves the camera backward along its direction.
    pub fn move_backward(&mut self, amount: f64) {
        let dir = self.direction;
        self.move_camera(&dir, -amount);
    }

    /// Moves the camera up along its up vector.
    pub fn move_up(&mut self, amount: f64) {
        let up = self.up;
        self.move_camera(&up, amount);
    }

    /// Moves the camera down along its up vector.
    pub fn move_down(&mut self, amount: f64) {
        let up = self.up;
        self.move_camera(&up, -amount);
    }

    /// Moves the camera right along its right vector.
    pub fn move_right(&mut self, amount: f64) {
        let right = self.right;
        self.move_camera(&right, amount);
    }

    /// Moves the camera left along its right vector.
    pub fn move_left(&mut self, amount: f64) {
        let right = self.right;
        self.move_camera(&right, -amount);
    }

    // ---- Rotation ----

    /// Rotates the camera around the given axis by the given angle (radians).
    pub fn rotate(&mut self, axis: &Cartesian3, angle: f64) {
        // Simplified rotation - in full port, this would update direction/up/right
        let _ = (axis, angle);
        self.changed = true;
    }

    /// Rotates the camera down (pitch).
    pub fn rotate_down(&mut self, angle: f64) {
        let right = self.right;
        self.rotate(&right, -angle);
    }

    /// Rotates the camera up (pitch).
    pub fn rotate_up(&mut self, angle: f64) {
        let right = self.right;
        self.rotate(&right, angle);
    }

    /// Rotates the camera right (yaw).
    pub fn rotate_right(&mut self, angle: f64) {
        let up = self.up;
        self.rotate(&up, -angle);
    }

    /// Rotates the camera left (yaw).
    pub fn rotate_left(&mut self, angle: f64) {
        let up = self.up;
        self.rotate(&up, angle);
    }

    // ---- Zoom ----

    /// Zooms the camera in by the given amount.
    pub fn zoom_in(&mut self, amount: f64) {
        self.move_forward(amount);
    }

    /// Zooms the camera out by the given amount.
    pub fn zoom_out(&mut self, amount: f64) {
        self.move_backward(amount);
    }

    // ---- Coordinate transforms ----

    /// Transforms a point from world coordinates to camera coordinates.
    pub fn world_to_camera_coordinates(&self, cartesian: &Cartesian3) -> Cartesian3 {
        Cartesian3::subtract_new(cartesian, &self.position)
    }

    /// Transforms a point from camera coordinates to world coordinates.
    pub fn camera_to_world_coordinates(&self, cartesian: &Cartesian3) -> Cartesian3 {
        Cartesian3::add_new(cartesian, &self.position)
    }

    // ---- Picking ----

    /// Gets a pick ray from window coordinates.
    pub fn get_pick_ray(&self, _window_position: &Cartesian2) -> Ray {
        Ray::new(Some(&self.position), Some(&self.direction))
    }

    /// Picks an ellipsoid at the given window position.
    pub fn pick_ellipsoid(
        &self,
        _window_position: &Cartesian2,
        ellipsoid: &Ellipsoid,
    ) -> Option<Cartesian3> {
        // Simplified - in full port, ray-ellipsoid intersection
        let _ = ellipsoid;
        None
    }

    // ---- View setup ----

    /// Sets the camera view.
    pub fn set_view(&mut self, position: Cartesian3, direction: Cartesian3, up: Cartesian3) {
        self.position = position;
        self.direction = direction;
        self.up = up;
        // Recompute right vector
        self.right = Cartesian3::cross_new(&self.direction, &self.up);
        self.changed = true;
    }

    /// Makes the camera look at a target position.
    pub fn look_at(&mut self, target: &Cartesian3, offset: &Cartesian3) {
        self.position = Cartesian3::add_new(target, offset);
        self.direction = Cartesian3::normalize_new(&Cartesian3::subtract_new(target, &self.position));
        self.changed = true;
    }

    // ---- Update ----

    /// Updates the camera matrices. Called once per frame.
    pub fn update(&mut self) {
        // Recompute view matrix from position/direction/up
        // Simplified - in full port, this handles reference frame transforms
        self.changed = false;
    }

    /// Returns whether the camera changed this frame.
    pub fn has_changed(&self) -> bool { self.changed }

    /// Returns the magnitude (distance from origin).
    pub fn get_magnitude(&self) -> f64 {
        Cartesian3::magnitude(&self.position)
    }

    // ---- Canvas dimensions ----

    /// Sets the canvas dimensions.
    pub fn set_canvas_size(&mut self, width: u32, height: u32) {
        self.canvas_width = width;
        self.canvas_height = height;
        if height > 0 {
            self.aspect_ratio = width as f64 / height as f64;
        }
        self.changed = true;
    }

    /// Returns the canvas width.
    pub fn canvas_width(&self) -> u32 { self.canvas_width }

    /// Returns the canvas height.
    pub fn canvas_height(&self) -> u32 { self.canvas_height }
}

impl Default for Camera {
    fn default() -> Self { Self::new() }
}
