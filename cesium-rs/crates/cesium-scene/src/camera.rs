//! Ported from `packages/engine/Source/Scene/Camera.js`.
//!
//! The camera defines the view frustum and position from which the scene is rendered.
//! In CesiumJS, this is a 3989-line file managing view/projection matrices, flight
//! animations, coordinate transforms, and user interaction (look/rotate/move/zoom).
//!
//! B4-1 materialization: the view matrix computation (`updateViewMatrix`), the
//! perspective/orthographic projection matrices, `getPickRay`, `pickEllipsoid`,
//! and the orthonormalizing `updateMembers` semantics are ported one-to-one.
//!
//! M3/S3 materialization: flight animations are driven through the shared
//! flight channel ([`crate::camera_flight_path`]); `Camera::update` applies
//! the in-flight pose each frame. Screen-space controllers remain future
//! work.

use cesium_core::cartesian2::Cartesian2;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::intersection_tests::IntersectionTests;
use cesium_core::math::CesiumMath;
use cesium_core::matrix3::Matrix3;
use cesium_core::matrix4::Matrix4;
use cesium_core::orthographic_off_center_frustum::OrthographicOffCenterFrustum;
use cesium_core::perspective_off_center_frustum::PerspectiveOffCenterFrustum;
use cesium_core::quaternion::Quaternion;
use cesium_core::ray::Ray;
use cesium_core::transforms;

use crate::camera_flight_path::{CameraFlightChannel, CameraFlightPath};

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
    /// The shared camera-flight channel installed by the [`crate::scene::Scene`]
    /// (M3/S3). `None` for standalone cameras; when set, [`Camera::update`]
    /// applies the in-flight pose each frame (mirrors CesiumJS
    /// `Camera#flyTo` driving `position`/`direction`/`up` from the tween).
    flight_channel: Option<CameraFlightChannel>,
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
            flight_channel: None,
        }
    }

    /// Attaches the shared flight channel (the `Scene` creates it and shares
    /// it with its camera so [`crate::scene::Scene::fly_to`] can drive the
    /// camera through `&self`).
    pub fn set_flight_channel(&mut self, channel: CameraFlightChannel) {
        self.flight_channel = Some(channel);
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

    /// Sets the camera right vector.
    pub fn set_right(&mut self, right: Cartesian3) {
        self.right = right;
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

    /// Returns the inverse camera transform.
    pub fn inverse_transform(&self) -> &Matrix4 { &self.inverse_transform }

    /// Sets the camera's reference frame transform.
    ///
    /// Mirrors CesiumJS `Camera.prototype.lookAtTransform` bookkeeping:
    /// the inverse is recomputed as an affine (rotation + translation) inverse.
    pub fn set_transform(&mut self, transform: Matrix4) {
        self.transform = transform;
        self.inverse_transform = Matrix4::inverse_transformation_new(&self.transform);
        self.changed = true;
    }

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

    /// Returns the SSE denominator for perspective LOD selection.
    ///
    /// Mirrors CesiumJS `PerspectiveFrustum#sseDenominator`:
    /// `2 * tan(fov / 2)`. Used by the quadtree screen-space error formula
    /// `sse = geometricError * drawingBufferHeight / (distance * sseDenominator)`.
    pub fn sse_denominator(&self) -> f64 {
        match self.projection {
            CameraProjection::Perspective => 2.0 * (self.fov * 0.5).tan(),
            // Orthographic projection has no distance-based SSE; CesiumJS
            // uses the 2D formula instead. Return 1.0 as a neutral value.
            CameraProjection::Orthographic => 1.0,
        }
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
    ///
    /// Mirrors CesiumJS `Camera#rotate`: builds an axis-angle rotation and
    /// applies it to the direction, up, and right vectors.
    pub fn rotate(&mut self, axis: &Cartesian3, angle: f64) {
        let axis = Cartesian3::normalize_new(axis);
        let rotation = Matrix3::from_quaternion_new(&Quaternion::from_axis_angle_new(&axis, angle));
        self.direction = Matrix3::multiply_by_vector_new(&rotation, &self.direction);
        self.up = Matrix3::multiply_by_vector_new(&rotation, &self.up);
        self.right = Matrix3::multiply_by_vector_new(&rotation, &self.right);
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
    ///
    /// Mirrors CesiumJS `Camera#worldToCameraCoordinatesPoint` (`Matrix4
    /// .multiplyByPoint` on the view matrix).
    pub fn world_to_camera_coordinates(&self, cartesian: &Cartesian3) -> Cartesian3 {
        Matrix4::multiply_by_point_new(&self.view_matrix, cartesian)
    }

    /// Transforms a point from camera coordinates to world coordinates.
    pub fn camera_to_world_coordinates(&self, cartesian: &Cartesian3) -> Cartesian3 {
        Matrix4::multiply_by_point_new(&self.inverse_view_matrix, cartesian)
    }

    // ---- Picking ----

    /// Gets a pick ray from window coordinates.
    ///
    /// Mirrors CesiumJS `Camera#getPickRay` → `PerspectiveFrustum#getPickRay`
    /// (or the orthographic branch, which offsets the ray origin instead).
    pub fn get_pick_ray(&self, window_position: &Cartesian2) -> Ray {
        let width = self.canvas_width as f64;
        let height = self.canvas_height as f64;
        match self.projection {
            CameraProjection::Perspective => {
                let tan_phi = (self.fov * 0.5).tan();
                let tan_theta = self.aspect_ratio * tan_phi;
                let x = (2.0 * window_position.x / width) - 1.0;
                let y = 1.0 - (2.0 * window_position.y / height);
                let right_offset = Cartesian3::multiply_by_scalar_new(&self.right, x * tan_theta);
                let up_offset = Cartesian3::multiply_by_scalar_new(&self.up, y * tan_phi);
                let mut direction = Cartesian3::add_new(&self.direction, &right_offset);
                direction = Cartesian3::add_new(&direction, &up_offset);
                direction = Cartesian3::normalize_new(&direction);
                Ray::new(Some(&self.position), Some(&direction))
            }
            CameraProjection::Orthographic => {
                // Mirrors CesiumJS Camera#getPickRay orthographic branch:
                // the ray keeps the camera direction; its origin is shifted
                // within the camera plane by the frustum extents.
                let frustum_height = self.orthographic_width / self.aspect_ratio;
                let x = ((2.0 * window_position.x / width) - 1.0) * (self.orthographic_width * 0.5);
                let y = (1.0 - (2.0 * window_position.y / height)) * (frustum_height * 0.5);
                let right_offset = Cartesian3::multiply_by_scalar_new(&self.right, x);
                let up_offset = Cartesian3::multiply_by_scalar_new(&self.up, y);
                let origin = Cartesian3::add_new(&self.position, &right_offset);
                let origin = Cartesian3::add_new(&origin, &up_offset);
                Ray::new(Some(&origin), Some(&self.direction))
            }
        }
    }

    /// Picks an ellipsoid at the given window position.
    ///
    /// Mirrors CesiumJS `Camera#pickEllipsoid`: casts the pick ray against the
    /// ellipsoid (`IntersectionTests.rayEllipsoid`) and returns the closest
    /// intersection point, or `None` when the ray misses.
    pub fn pick_ellipsoid(
        &self,
        window_position: &Cartesian2,
        ellipsoid: &Ellipsoid,
    ) -> Option<Cartesian3> {
        let ray = self.get_pick_ray(window_position);
        IntersectionTests::ray_ellipsoid(&ray, ellipsoid)
            .map(|interval| Ray::get_point_new(&ray, Some(interval.start)))
    }

    // ---- View setup ----

    /// Sets the camera view.
    ///
    /// Mirrors the pure-math core of CesiumJS `Camera#setView` in scene3D:
    /// with only a destination the camera looks straight down at the surface
    /// (direction = -surface normal); with an explicit direction/up the
    /// orientation is used verbatim and `right` is derived.
    pub fn set_view(
        &mut self,
        destination: &Cartesian3,
        direction: Option<&Cartesian3>,
        up: Option<&Cartesian3>,
        ellipsoid: &Ellipsoid,
    ) {
        self.position = *destination;
        self.changed = true;

        match (direction, up) {
            (Some(direction), Some(up)) => {
                self.direction = Cartesian3::normalize_new(direction);
                self.up = Cartesian3::normalize_new(up);
                self.right = Cartesian3::cross_new(&self.direction, &self.up);
                self.right = Cartesian3::normalize_new(&self.right);
                self.up = Cartesian3::cross_new(&self.right, &self.direction);
            }
            _ => {
                // Destination only: CesiumJS `setView` scene3D default derives
                // the orientation from the local east-north-up frame — the
                // camera looks straight down (direction = -up axis), up points
                // north, and right points east.
                let frame = transforms::east_north_up_to_fixed_frame_new(
                    destination,
                    Some(ellipsoid),
                );
                let e = &frame.elements;
                self.right = Cartesian3::new(e[0], e[1], e[2]);
                self.up = Cartesian3::new(e[4], e[5], e[6]);
                self.direction = Cartesian3::new(-e[8], -e[9], -e[10]);
            }
        }
    }

    /// Makes the camera look at a target position.
    ///
    /// Mirrors CesiumJS `Camera#lookAt`: builds the east-north-up frame at
    /// the target and delegates to [`Camera::look_at_transform`].
    pub fn look_at(
        &mut self,
        target: &Cartesian3,
        offset: &Cartesian3,
        ellipsoid: &Ellipsoid,
    ) {
        let transform =
            transforms::east_north_up_to_fixed_frame_new(target, Some(ellipsoid));
        self.look_at_transform(&transform, offset);
    }

    /// Makes the camera look at the origin of a reference frame.
    ///
    /// Mirrors the 3D path of CesiumJS `Camera#lookAtTransform`: position and
    /// direction are expressed in the transform's reference frame, `right`
    /// keeps the world Z axis aligned (`direction × UNIT_Z`), and the view
    /// matrix later folds in the inverse transform.
    pub fn look_at_transform(&mut self, transform: &Matrix4, offset: &Cartesian3) {
        self.set_transform(*transform);
        self.position = *offset;
        let negated = Cartesian3::multiply_by_scalar_new(&self.position, -1.0);
        self.direction = Cartesian3::normalize_new(&negated);
        let mut right = Cartesian3::cross_new(&self.direction, &Cartesian3::UNIT_Z);
        if Cartesian3::magnitude_squared(&right) < CesiumMath::EPSILON10 {
            right = Cartesian3::UNIT_X;
        }
        self.right = Cartesian3::normalize_new(&right);
        self.up = Cartesian3::normalize_new(&Cartesian3::cross_new(
            &self.right,
            &self.direction,
        ));
        self.changed = true;
    }

    // ---- Update ----

    /// Updates the camera matrices. Called once per frame.
    ///
    /// Mirrors the CesiumJS view/projection matrix getters (which recompute
    /// lazily): `updateViewMatrix` + the frustum projection matrix.
    pub fn update(&mut self) {
        self.apply_flight();
        self.update_members();
        self.update_view_matrix();
        self.update_projection_matrix();
        self.changed = false;
    }

    /// Applies the in-flight camera pose from the shared flight channel
    /// (M3/S3, mirrors the CesiumJS `Camera#flyTo` tween update closure).
    ///
    /// While a flight is active the interpolated pose is applied; when the
    /// tween signals completion the exact end pose is applied and the
    /// channel is consumed.
    fn apply_flight(&mut self) {
        let Some(channel) = self.flight_channel.as_ref() else {
            return;
        };
        let mut slot = channel.borrow_mut();
        let Some(flight) = slot.as_ref() else {
            return;
        };
        if flight.completed {
            self.position = flight.end_position;
            self.direction = flight.end_direction;
            self.up = flight.end_up;
            self.changed = true;
            *slot = None;
            return;
        }
        let (position, direction, up) = CameraFlightPath::interpolate(flight, flight.t);
        self.position = position;
        self.direction = direction;
        self.up = up;
        self.changed = true;
    }

    /// Orthonormalizes the direction/up/right axes.
    ///
    /// Mirrors CesiumJS `updateMembers` which the `viewMatrix` getter runs
    /// before computing the matrix.
    fn update_members(&mut self) {
        let cross = Cartesian3::cross_new(&self.direction, &self.up);
        if Cartesian3::magnitude(&cross) < CesiumMath::EPSILON6 {
            // Direction and up are nearly parallel; derive up from right.
            let up_from_right = Cartesian3::cross_new(&self.right, &self.direction);
            if Cartesian3::magnitude(&up_from_right) >= CesiumMath::EPSILON6 {
                self.up = Cartesian3::normalize_new(&up_from_right);
            }
        }
        self.direction = Cartesian3::normalize_new(&self.direction);
        let right = Cartesian3::cross_new(&self.direction, &self.up);
        if Cartesian3::magnitude(&right) >= CesiumMath::EPSILON6 {
            self.right = Cartesian3::normalize_new(&right);
        }
        self.up = Cartesian3::cross_new(&self.right, &self.direction);
    }

    /// Recomputes the view matrix from position/direction/up/right.
    ///
    /// Mirrors CesiumJS `updateViewMatrix`:
    /// `view = computeView(position, direction, up, right) * actualInvTransform`,
    /// then the inverse view is the affine inverse.
    fn update_view_matrix(&mut self) {
        let view = Matrix4::compute_view_new(
            &self.position,
            &self.direction,
            &self.up,
            &self.right,
        );
        self.view_matrix = Matrix4::multiply_new(&view, &self.inverse_transform);
        self.inverse_view_matrix = Matrix4::inverse_transformation_new(&self.view_matrix);
    }

    /// Recomputes the projection matrix from the current frustum parameters.
    ///
    /// Mirrors CesiumJS `PerspectiveFrustum#update` → off-center projection
    /// (fov drives both halves symmetrically), and the orthographic frustum
    /// whose height follows from width / aspectRatio.
    fn update_projection_matrix(&mut self) {
        match self.projection {
            CameraProjection::Perspective => {
                // CesiumJS PerspectiveFrustum#update: `fov` is the vertical
                // FOV — `top = near * tan(fov/2)`, `right = aspect * top`.
                let tan_half_fov = (self.fov * 0.5).tan();
                let top = self.near * tan_half_fov;
                let right = self.aspect_ratio * top;
                let mut frustum = PerspectiveOffCenterFrustum::new();
                frustum.left = Some(-right);
                frustum.right = Some(right);
                frustum.top = Some(top);
                frustum.bottom = Some(-top);
                frustum.near = self.near;
                frustum.far = self.far;
                self.projection_matrix = frustum.compute_projection_matrix();
            }
            CameraProjection::Orthographic => {
                let half_width = self.orthographic_width * 0.5;
                let half_height = half_width / self.aspect_ratio;
                let mut frustum = OrthographicOffCenterFrustum::new();
                frustum.left = Some(-half_width);
                frustum.right = Some(half_width);
                frustum.top = Some(half_height);
                frustum.bottom = Some(-half_height);
                frustum.near = self.near;
                frustum.far = self.far;
                self.projection_matrix = frustum.compute_projection_matrix();
            }
        }
        self.inverse_projection_matrix =
            Matrix4::inverse_new(&self.projection_matrix).unwrap_or(Matrix4::IDENTITY);
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
