//! Camera controller for orbit, pan, and zoom interactions.
//!
//! Maps to CesiumJS `Scene/ScreenSpaceCameraController.js`

use cesium_camera::Camera;
use cesium_geospatial::ellipsoid::Ellipsoid;
use glam::{DVec2, DVec3};

use crate::inertia::{InertiaController, InertiaSample, InertiaState};

/// Camera controller configuration.
#[derive(Debug, Clone)]
pub struct CameraControllerConfig {
    /// Minimum zoom distance from the surface (meters).
    pub minimum_zoom_distance: f64,
    /// Maximum zoom distance from the surface (meters).
    pub maximum_zoom_distance: f64,
    /// Rotation speed factor.
    pub rotation_speed: f64,
    /// Pan speed factor.
    pub pan_speed: f64,
    /// Zoom speed factor.
    pub zoom_speed: f64,
    /// Whether rotation is enabled.
    pub enable_rotation: bool,
    /// Whether panning is enabled.
    pub enable_pan: bool,
    /// Whether zooming is enabled.
    pub enable_zoom: bool,
    /// Whether collision detection with the ellipsoid is enabled.
    pub enable_collision_detection: bool,
}

impl Default for CameraControllerConfig {
    fn default() -> Self {
        Self {
            minimum_zoom_distance: 1.0,
            maximum_zoom_distance: f64::INFINITY,
            rotation_speed: 1.0,
            pan_speed: 1.0,
            zoom_speed: 1.0,
            enable_rotation: true,
            enable_pan: true,
            enable_zoom: true,
            enable_collision_detection: true,
        }
    }
}

/// The camera controller that processes user input and updates the camera.
///
/// Maps to CesiumJS `ScreenSpaceCameraController`
#[derive(Debug, Clone)]
pub struct CameraController {
    /// Configuration.
    pub config: CameraControllerConfig,
    /// The ellipsoid for surface calculations.
    pub ellipsoid: Ellipsoid,
}

impl CameraController {
    /// Creates a new camera controller.
    pub fn new(ellipsoid: Ellipsoid) -> Self {
        Self {
            config: CameraControllerConfig::default(),
            ellipsoid,
        }
    }

    /// Orbits the camera around a target point.
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `target` - The point to orbit around (ECEF)
    /// * `delta_heading` - Change in heading (radians)
    /// * `delta_pitch` - Change in pitch (radians)
    /// * `delta_range` - Change in distance (meters, positive = zoom out)
    pub fn orbit(
        &self,
        camera: &mut Camera,
        target: DVec3,
        delta_heading: f64,
        delta_pitch: f64,
        delta_range: f64,
    ) {
        if !self.config.enable_rotation {
            return;
        }

        let heading = delta_heading * self.config.rotation_speed;
        let pitch = delta_pitch * self.config.rotation_speed;

        // Vector from target to camera
        let offset = camera.position - target;
        let range = offset.length() + delta_range * self.config.zoom_speed;

        // Clamp range
        let range = range.max(self.config.minimum_zoom_distance);
        let range = if self.config.maximum_zoom_distance.is_finite() {
            range.min(self.config.maximum_zoom_distance)
        } else {
            range
        };

        // Convert to spherical coordinates
        let mut current_heading = offset.z.atan2(offset.x);
        let horizontal_dist = (offset.x * offset.x + offset.z * offset.z).sqrt();
        let mut current_pitch = offset.y.atan2(horizontal_dist);

        // Apply deltas
        current_heading += heading;
        current_pitch += pitch;

        // Clamp pitch to avoid gimbal issues
        let max_pitch = std::f64::consts::FRAC_PI_2 - 0.001;
        current_pitch = current_pitch.clamp(-max_pitch, max_pitch);

        // Convert back to Cartesian
        let cos_pitch = current_pitch.cos();
        let new_offset = DVec3::new(
            range * cos_pitch * current_heading.cos(),
            range * current_pitch.sin(),
            range * cos_pitch * current_heading.sin(),
        );

        camera.position = target + new_offset;
        camera.direction = (target - camera.position).normalize();
        camera.right = camera.direction.cross(DVec3::Y).normalize();
        camera.up = camera.right.cross(camera.direction).normalize();
    }

    /// Pans the camera along the view plane.
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `delta_x` - Horizontal pan amount (normalized, -1 to 1)
    /// * `delta_y` - Vertical pan amount (normalized, -1 to 1)
    pub fn pan(&self, camera: &mut Camera, delta_x: f64, delta_y: f64) {
        if !self.config.enable_pan {
            return;
        }

        // Scale pan by distance to surface
        let height = camera.position.length() - self.ellipsoid.maximum_radius();
        let pan_scale = height.abs().max(1000.0) * 0.001 * self.config.pan_speed;

        let move_right = camera.right * (-delta_x * pan_scale);
        let move_up = camera.up * (delta_y * pan_scale);

        camera.position += move_right + move_up;
    }

    /// Zooms the camera in or out.
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `delta` - Zoom amount (positive = zoom in, negative = zoom out)
    pub fn zoom(&self, camera: &mut Camera, delta: f64) {
        if !self.config.enable_zoom {
            return;
        }

        // Scale zoom by distance to surface
        let height = camera.position.length() - self.ellipsoid.maximum_radius();
        let zoom_amount = height.abs().max(1000.0) * 0.1 * delta * self.config.zoom_speed;

        let movement = camera.direction * zoom_amount;
        let new_position = camera.position + movement;

        // Collision detection
        if self.config.enable_collision_detection {
            let new_height = new_position.length() - self.ellipsoid.maximum_radius();
            if new_height < self.config.minimum_zoom_distance {
                return; // Don't zoom below minimum distance
            }
        }

        camera.position = new_position;
    }

    /// Tilts the camera (changes pitch while looking at a target).
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `target` - The point to look at (ECEF)
    /// * `delta_pitch` - Change in pitch (radians)
    pub fn tilt(&self, camera: &mut Camera, target: DVec3, delta_pitch: f64) {
        let offset = camera.position - target;
        let range = offset.length();

        // Rotate offset around the right axis
        let surface_normal = target.normalize();
        let right = offset.cross(surface_normal).normalize();
        let rotated = rotate_around_axis(offset.normalize(), right, delta_pitch * self.config.rotation_speed);

        camera.position = target + rotated * range;
        camera.direction = (target - camera.position).normalize();
        camera.right = camera.direction.cross(DVec3::Y).normalize();
        camera.up = camera.right.cross(camera.direction).normalize();
    }

    /// Spins (rotates) the camera about the ellipsoid center: the position and
    /// the orientation rotate together, so the globe appears to turn under the
    /// viewer.
    ///
    /// Maps to CesiumJS `rotate3D`/`spin3D` — blueprint L1963-1968
    /// (`camera.rotate_right(delta_phi)` then `camera.rotate_up(delta_theta)`).
    /// The pixel→radian scaling is the adapter's responsibility; this method
    /// takes signed angles in radians.
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `delta_heading` - Rotation about the camera up axis (radians)
    /// * `delta_pitch` - Rotation about the camera right axis (radians)
    pub fn spin(&self, camera: &mut Camera, delta_heading: f64, delta_pitch: f64) {
        if !self.config.enable_rotation {
            return;
        }
        let heading = delta_heading * self.config.rotation_speed;
        let pitch = delta_pitch * self.config.rotation_speed;
        camera.rotate_right(heading);
        camera.rotate_up(pitch);
    }

    /// Looks around in place: rotates the orientation (direction/up) without
    /// moving the position.
    ///
    /// Maps to CesiumJS `look3D` — blueprint L2814-2851 (horizontal
    /// `camera.look_left(angle)` then a vertical `look` about the right axis).
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `delta_heading` - Yaw angle (radians)
    /// * `delta_pitch` - Pitch angle (radians)
    pub fn look(&self, camera: &mut Camera, delta_heading: f64, delta_pitch: f64) {
        if !self.config.enable_rotation {
            return;
        }
        let heading = delta_heading * self.config.rotation_speed;
        let pitch = delta_pitch * self.config.rotation_speed;
        camera.look_left(Some(heading));
        camera.look_up(Some(pitch));
    }

    /// Twists (rolls) the camera about its own view direction.
    ///
    /// Maps to CesiumJS `twist2D` — blueprint L1178 (`camera.twist_right(theta)`).
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `delta_angle` - Roll angle (radians, positive = clockwise)
    pub fn twist(&self, camera: &mut Camera, delta_angle: f64) {
        if !self.config.enable_rotation {
            return;
        }
        camera.twist_right(delta_angle * self.config.rotation_speed);
    }

    // ========================================================================
    // Two-finger (pinch) touch gestures — M2.6
    // ========================================================================
    //
    // These map the decomposed two-finger metrics produced by
    // [`crate::event_aggregator::CameraEventAggregator`] (`pinch_distance_delta`,
    // `pinch_angle_delta`, `pinch_midpoint_delta`) onto camera transforms:
    //
    // | Gesture | Metric | Camera action |
    // |---------|--------|---------------|
    // | pinch (fingers spread/close) | distance Δ (px) | [`Self::zoom`] |
    // | rotate (fingers turn about midpoint) | angle Δ (rad) | [`Self::spin`] (heading) |
    // | drag (fingers translate together) | midpoint Δ (px) | [`Self::pan`] |
    //
    // Per the M2.6 gesture model the two-finger **rotate maps to spin** (the
    // globe turns under the viewer), not to CesiumJS's `twist2D` roll; the
    // roll mapping remains available via [`Self::twist`] +
    // `pinch_twist_pixels` for applications that prefer it.
    //
    // Pixel→world magnitude scaling stays at the adapter boundary: the pixel
    // deltas are passed through with an adapter-supplied `scale` (mirroring
    // [`Self::coast_inertia`]), so the domain owns only the gesture→action
    // *semantics* (which component drives which motion, and its sign) and
    // stays resolution-independent and free of render-unit concerns.

    /// Two-finger **pinch → zoom**.
    ///
    /// `distance_delta` is the change in finger separation in pixels
    /// (positive = fingers spreading apart = zoom **in**, matching the
    /// conventional pinch-to-zoom feel). `scale` converts pixels to the
    /// unitless zoom delta consumed by [`Self::zoom`] (adapter-supplied, e.g.
    /// a sensitivity over the canvas height). Honors `enable_zoom`.
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `distance_delta` - Finger-separation change this frame (pixels)
    /// * `scale` - Pixel→zoom-delta scale supplied by the adapter boundary
    pub fn pinch_zoom(&self, camera: &mut Camera, distance_delta: f64, scale: f64) {
        // Spreading fingers (positive delta) zoom in (positive zoom delta).
        self.zoom(camera, distance_delta * scale);
    }

    /// Two-finger **rotate → spin** (heading).
    ///
    /// `angle_delta` is the rotation of the finger-connecting line in radians
    /// (positive = counter-clockwise on screen, from `atan2`). It is already an
    /// angle, so no pixel scaling is applied; it is mapped directly to the
    /// heading component of [`Self::spin`] (pitch = 0). Honors `enable_rotation`.
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `angle_delta` - Finger-line rotation this frame (radians)
    pub fn pinch_rotate(&self, camera: &mut Camera, angle_delta: f64) {
        self.spin(camera, angle_delta, 0.0);
    }

    /// Two-finger **drag → translate** (pan).
    ///
    /// `midpoint_delta` is the common translation of both fingers in pixels
    /// (the change in the two-finger midpoint). `scale` converts pixels to the
    /// normalized pan delta consumed by [`Self::pan`] (adapter-supplied); the
    /// existing `pan` sign convention (`-delta_x` along right, `+delta_y` along
    /// up) gives the grab-the-globe feel. Honors `enable_pan`.
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `midpoint_delta` - Common finger translation this frame (pixels)
    /// * `scale` - Pixel→pan-delta scale supplied by the adapter boundary
    pub fn pinch_translate(&self, camera: &mut Camera, midpoint_delta: DVec2, scale: f64) {
        self.pan(camera, midpoint_delta.x * scale, midpoint_delta.y * scale);
    }

    /// Applies one inertial coasting step for `slot`, driving the matching
    /// camera motion with the decayed motion returned by
    /// [`InertiaController::maintain`].
    ///
    /// This is the domain-level integration of CesiumJS `maintainInertia`
    /// (blueprint L796-875): the stored pixel motion is tapered by the
    /// [`crate::inertia::decay`] exponential and fed to `spin`/`zoom`/`pan`/
    /// `tilt`. `scale` converts the pixel delta into radians (for spin/tilt) or
    /// meters (for zoom/pan); supplying it here keeps the pixel→world
    /// conversion — including any render-unit scaling — at the adapter boundary.
    ///
    /// # Returns
    /// `true` while the camera is still coasting, `false` once the inertia has
    /// stopped (released, held too long, or decayed below the stop distance).
    ///
    /// # Arguments
    /// * `camera` - The camera to update
    /// * `inertia` - The inertia controller holding the captured motion
    /// * `slot` - Which inertia state to coast
    /// * `target` - The pivot point for tilt inertia (ECEF)
    /// * `sample` - Per-frame timing + decay coefficient
    /// * `scale` - Pixel→radian/meter scale applied to the decayed delta
    pub fn coast_inertia(
        &self,
        camera: &mut Camera,
        inertia: &mut InertiaController,
        slot: InertiaState,
        target: DVec3,
        sample: &InertiaSample,
        scale: f64,
    ) -> bool {
        let delta = match inertia.maintain(slot, sample) {
            Some(delta) => delta,
            None => return false,
        };
        match slot {
            InertiaState::Spin => self.spin(camera, delta.x * scale, delta.y * scale),
            InertiaState::Zoom => self.zoom(camera, -delta.y * scale),
            InertiaState::Translate => self.pan(camera, delta.x * scale, delta.y * scale),
            InertiaState::Tilt => self.tilt(camera, target, delta.y * scale),
        }
        true
    }

    /// Ensures the camera is not below the ellipsoid surface.
    pub fn enforce_collision(&self, camera: &mut Camera) {
        if !self.config.enable_collision_detection {
            return;
        }

        let height = camera.position.length() - self.ellipsoid.maximum_radius();
        if height < self.config.minimum_zoom_distance {
            let normal = camera.position.normalize();
            camera.position = normal * (self.ellipsoid.maximum_radius() + self.config.minimum_zoom_distance);
        }
    }
}

/// Rotates a vector around an axis by an angle (Rodrigues' formula).
fn rotate_around_axis(v: DVec3, axis: DVec3, angle: f64) -> DVec3 {
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    v * cos_a + axis.cross(v) * sin_a + axis * axis.dot(v) * (1.0 - cos_a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_camera() -> Camera {
        // Camera above the equator looking down
        Camera::new(
            DVec3::new(6378137.0 * 2.0, 0.0, 0.0),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        )
    }

    #[test]
    fn test_camera_controller_creation() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        assert!(controller.config.enable_rotation);
        assert!(controller.config.enable_pan);
        assert!(controller.config.enable_zoom);
    }

    #[test]
    fn test_zoom_in() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial_distance = camera.position.length();

        controller.zoom(&mut camera, 1.0); // Zoom in

        assert!(camera.position.length() < initial_distance);
    }

    #[test]
    fn test_zoom_out() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial_distance = camera.position.length();

        controller.zoom(&mut camera, -1.0); // Zoom out

        assert!(camera.position.length() > initial_distance);
    }

    #[test]
    fn test_zoom_disabled() {
        let mut controller = CameraController::new(Ellipsoid::WGS84);
        controller.config.enable_zoom = false;
        let mut camera = create_test_camera();
        let initial_pos = camera.position;

        controller.zoom(&mut camera, 1.0);

        assert_eq!(camera.position, initial_pos);
    }

    #[test]
    fn test_pan() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial_pos = camera.position;

        controller.pan(&mut camera, 1.0, 0.0);

        assert_ne!(camera.position, initial_pos);
    }

    #[test]
    fn test_orbit() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let target = DVec3::ZERO;
        let initial_distance = (camera.position - target).length();

        controller.orbit(&mut camera, target, 0.1, 0.0, 0.0);

        // Distance should be preserved during pure rotation
        let new_distance = (camera.position - target).length();
        assert!((new_distance - initial_distance).abs() / initial_distance < 0.01);
    }

    #[test]
    fn test_collision_detection() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = Camera::new(
            DVec3::new(6378137.0 + 0.5, 0.0, 0.0), // Very close to surface
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        );

        controller.enforce_collision(&mut camera);

        let height = camera.position.length() - Ellipsoid::WGS84.maximum_radius();
        assert!(height >= controller.config.minimum_zoom_distance);
    }

    #[test]
    fn test_rotate_around_axis() {
        let v = DVec3::new(1.0, 0.0, 0.0);
        let axis = DVec3::new(0.0, 0.0, 1.0);
        let angle = std::f64::consts::FRAC_PI_2;

        let rotated = rotate_around_axis(v, axis, angle);

        // 90 degrees around Z: X → Y
        assert!((rotated.x).abs() < 1e-10);
        assert!((rotated.y - 1.0).abs() < 1e-10);
        assert!((rotated.z).abs() < 1e-10);
    }

    #[test]
    fn test_spin_rotates_about_center_preserving_distance() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial_distance = camera.position.length();
        let initial_pos = camera.position;

        controller.spin(&mut camera, 0.1, 0.0);

        // Position moved but stayed on the same sphere about the center.
        assert!((camera.position - initial_pos).length() > 1.0);
        assert!((camera.position.length() - initial_distance).abs() / initial_distance < 1e-9);
        // Orientation stays orthonormal.
        assert!((camera.direction.length() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_spin_disabled_no_change() {
        let mut controller = CameraController::new(Ellipsoid::WGS84);
        controller.config.enable_rotation = false;
        let mut camera = create_test_camera();
        let initial_pos = camera.position;

        controller.spin(&mut camera, 0.5, 0.5);

        assert_eq!(camera.position, initial_pos);
    }

    #[test]
    fn test_look_rotates_orientation_only() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial_pos = camera.position;
        let initial_dir = camera.direction;

        controller.look(&mut camera, 0.2, 0.0);

        // Position is untouched; direction changes.
        assert_eq!(camera.position, initial_pos);
        assert!(camera.direction.dot(initial_dir) < 0.999999);
        assert!((camera.direction.length() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_look_disabled_no_change() {
        let mut controller = CameraController::new(Ellipsoid::WGS84);
        controller.config.enable_rotation = false;
        let mut camera = create_test_camera();
        let initial_dir = camera.direction;

        controller.look(&mut camera, 0.3, 0.3);

        assert!((camera.direction - initial_dir).length() < 1e-12);
    }

    #[test]
    fn test_twist_rolls_about_view_direction() {
        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial_pos = camera.position;
        let initial_dir = camera.direction;
        let initial_up = camera.up;

        controller.twist(&mut camera, 0.3);

        // Rolling keeps position and view direction, but rotates `up`.
        assert_eq!(camera.position, initial_pos);
        assert!(camera.direction.dot(initial_dir) > 0.999999);
        assert!(camera.up.dot(initial_up) < 0.999999);
        assert!((camera.up.length() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_coast_inertia_spin_moves_and_decays() {
        use crate::inertia::{InertiaController, InertiaSample, InertiaState};
        use glam::DVec2;

        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let mut inertia = InertiaController::new();
        inertia.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(2000.0, 0.0));

        // pixel→radian scale supplied by the adapter boundary.
        let scale = 1e-4;
        let target = DVec3::ZERO;

        let s1 = InertiaSample::new(0.9, 0.0, 0.0, 16.0);
        let pos0 = camera.position;
        assert!(controller.coast_inertia(&mut camera, &mut inertia, InertiaState::Spin, target, &s1, scale));
        let step1 = (camera.position - pos0).length();
        assert!(step1 > 0.0);

        // A later frame has decayed further → a smaller step.
        let s2 = InertiaSample::new(0.9, 0.0, 0.0, 600.0);
        let pos1 = camera.position;
        assert!(controller.coast_inertia(&mut camera, &mut inertia, InertiaState::Spin, target, &s2, scale));
        let step2 = (camera.position - pos1).length();
        assert!(step2 < step1, "inertia step must decay: {step2} !< {step1}");
    }

    #[test]
    fn test_coast_inertia_stops_when_disabled() {
        use crate::inertia::{InertiaController, InertiaSample, InertiaState};
        use glam::DVec2;

        let controller = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let mut inertia = InertiaController::new();
        inertia.capture(InertiaState::Translate, DVec2::ZERO, DVec2::new(800.0, 0.0));
        inertia.deactivate(InertiaState::Translate);

        let sample = InertiaSample::new(0.9, 0.0, 0.0, 16.0);
        let initial = camera.position;
        let coasting = controller.coast_inertia(
            &mut camera,
            &mut inertia,
            InertiaState::Translate,
            DVec3::ZERO,
            &sample,
            1.0,
        );
        assert!(!coasting);
        assert_eq!(camera.position, initial);
    }

    // ========================================================================
    // M2.6 two-finger (pinch) touch-gesture sequences
    // ========================================================================

    use crate::event_aggregator::CameraEventAggregator;

    /// Advance one frame of a two-finger gesture. The aggregator seeds `start`
    /// on the first `pinch_move` of a frame and extends `end` on the second
    /// (reproducing CesiumJS's per-frame pinch), so feeding the frame's opening
    /// pair `(a1,a2)` and closing pair `(b1,b2)` yields a per-frame delta of
    /// `metrics(b) - metrics(a)`.
    fn pinch_frame(
        agg: &mut CameraEventAggregator,
        t: f64,
        a1: DVec2,
        a2: DVec2,
        b1: DVec2,
        b2: DVec2,
    ) {
        agg.reset(t);
        agg.pinch_move(a1, a2);
        agg.pinch_move(b1, b2);
    }

    /// Apply a full two-finger frame (zoom + rotate + translate) to the camera.
    /// `a`/`b` are the frame's opening/closing finger pairs; `scales` is
    /// `(zoom_scale, translate_scale)`.
    fn apply_pinch_frame(
        agg: &mut CameraEventAggregator,
        ctrl: &CameraController,
        camera: &mut Camera,
        t: f64,
        a: (DVec2, DVec2),
        b: (DVec2, DVec2),
        scales: (f64, f64),
    ) {
        pinch_frame(agg, t, a.0, a.1, b.0, b.1);
        ctrl.pinch_zoom(camera, agg.pinch_distance_delta(), scales.0);
        ctrl.pinch_rotate(camera, agg.pinch_angle_delta());
        ctrl.pinch_translate(camera, agg.pinch_midpoint_delta(), scales.1);
    }

    #[test]
    fn pinch_spread_zooms_in() {
        let ctrl = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial = camera.position.length();
        // Fingers spread apart: separation 100 → 200 px (distance_delta = +100).
        ctrl.pinch_zoom(&mut camera, 100.0, 0.01);
        assert!(
            camera.position.length() < initial,
            "spreading fingers must zoom in"
        );
    }

    #[test]
    fn pinch_close_zooms_out() {
        let ctrl = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial = camera.position.length();
        // Fingers pinch together: separation 200 → 100 px (distance_delta = -100).
        ctrl.pinch_zoom(&mut camera, -100.0, 0.01);
        assert!(
            camera.position.length() > initial,
            "closing fingers must zoom out"
        );
    }

    #[test]
    fn pinch_rotate_maps_angle_to_spin_preserving_distance() {
        let ctrl = CameraController::new(Ellipsoid::WGS84);
        let mut camera = create_test_camera();
        let initial = camera.position.length();
        let pos0 = camera.position;
        // A counter-clockwise two-finger rotation of +0.2 rad → heading spin.
        ctrl.pinch_rotate(&mut camera, 0.2);
        // Spin rotates about the center: distance preserved, position moved.
        assert!((camera.position.length() - initial).abs() / initial < 1e-9);
        assert!((camera.position - pos0).length() > 1.0);
        assert!((camera.direction.length() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn pinch_rotate_sign_follows_angle_direction() {
        let ctrl = CameraController::new(Ellipsoid::WGS84);
        // CCW (+) and CW (−) rotations must move the camera in opposite senses.
        let mut ccw = create_test_camera();
        let mut cw = create_test_camera();
        let pos0 = ccw.position;
        ctrl.pinch_rotate(&mut ccw, 0.2);
        ctrl.pinch_rotate(&mut cw, -0.2);
        let d_ccw = ccw.position - pos0;
        let d_cw = cw.position - pos0;
        // Opposite rotation directions → displacements point opposite ways.
        assert!(
            d_ccw.dot(d_cw) < 0.0,
            "CCW and CW pinch-rotate must spin oppositely"
        );
    }

    #[test]
    fn pinch_drag_translates_opposite_directions() {
        let ctrl = CameraController::new(Ellipsoid::WGS84);
        // +x and −x midpoint drags must translate the camera oppositely.
        let mut right = create_test_camera();
        let mut left = create_test_camera();
        let pos0 = right.position;
        ctrl.pinch_translate(&mut right, DVec2::new(100.0, 0.0), 0.001);
        ctrl.pinch_translate(&mut left, DVec2::new(-100.0, 0.0), 0.001);
        let d_right = right.position - pos0;
        let d_left = left.position - pos0;
        assert!(d_right.length() > 0.0, "drag must translate the camera");
        assert!(
            d_right.dot(d_left) < 0.0,
            "opposite drags must translate oppositely"
        );

        // Same check for the vertical axis.
        let mut up = create_test_camera();
        let mut down = create_test_camera();
        let pos1 = up.position;
        ctrl.pinch_translate(&mut up, DVec2::new(0.0, 100.0), 0.001);
        ctrl.pinch_translate(&mut down, DVec2::new(0.0, -100.0), 0.001);
        assert!((up.position - pos1).dot(down.position - pos1) < 0.0);
    }

    #[test]
    fn pinch_zoom_disabled_honors_flag() {
        let mut ctrl = CameraController::new(Ellipsoid::WGS84);
        ctrl.config.enable_zoom = false;
        let mut camera = create_test_camera();
        let pos0 = camera.position;
        ctrl.pinch_zoom(&mut camera, 200.0, 0.01);
        assert_eq!(camera.position, pos0);
    }

    #[test]
    fn pinch_rotate_disabled_honors_flag() {
        let mut ctrl = CameraController::new(Ellipsoid::WGS84);
        ctrl.config.enable_rotation = false;
        let mut camera = create_test_camera();
        let pos0 = camera.position;
        ctrl.pinch_rotate(&mut camera, 0.5);
        assert_eq!(camera.position, pos0);
    }

    #[test]
    fn pinch_translate_disabled_honors_flag() {
        let mut ctrl = CameraController::new(Ellipsoid::WGS84);
        ctrl.config.enable_pan = false;
        let mut camera = create_test_camera();
        let pos0 = camera.position;
        ctrl.pinch_translate(&mut camera, DVec2::new(120.0, 80.0), 0.001);
        assert_eq!(camera.position, pos0);
    }

    /// A three-frame pinch-open sequence: each frame spreads the fingers a bit
    /// more, so every frame's `distance_delta` is positive and the camera keeps
    /// zooming in monotonically.
    #[test]
    fn multi_frame_pinch_open_zooms_in_monotonically() {
        let ctrl = CameraController::new(Ellipsoid::WGS84);
        let mut agg = CameraEventAggregator::new();
        let mut camera = create_test_camera();
        agg.pinch_start(DVec2::new(-50.0, 0.0), DVec2::new(50.0, 0.0));

        let mut prev_len = camera.position.length();
        // Frame k opens the separation from w_k to w_{k+1} about a fixed midpoint.
        let widths = [50.0, 80.0, 120.0, 170.0];
        for k in 0..3 {
            let (a, b) = (widths[k], widths[k + 1]);
            apply_pinch_frame(
                &mut agg,
                &ctrl,
                &mut camera,
                k as f64 / 60.0,
                (DVec2::new(-a, 0.0), DVec2::new(a, 0.0)),
                (DVec2::new(-b, 0.0), DVec2::new(b, 0.0)),
                (0.002, 0.0),
            );
            let len = camera.position.length();
            assert!(
                len < prev_len,
                "frame {k}: pinch-open must keep zooming in ({len} !< {prev_len})"
            );
            prev_len = len;
        }
    }

    /// A combined gesture over two frames: fingers spread (zoom in), rotate CCW
    /// (spin), and drift right (translate) simultaneously. All three camera
    /// effects must be present.
    #[test]
    fn combined_pinch_applies_zoom_spin_and_translate() {
        let ctrl = CameraController::new(Ellipsoid::WGS84);
        let mut agg = CameraEventAggregator::new();
        let mut camera = create_test_camera();
        let pos0 = camera.position;
        let len0 = pos0.length();

        agg.pinch_start(DVec2::new(-50.0, 0.0), DVec2::new(50.0, 0.0));
        // Frame 1: spread 50→90, rotate the finger line slightly CCW, drift the
        // midpoint right by 40 px.
        apply_pinch_frame(
            &mut agg,
            &ctrl,
            &mut camera,
            0.0,
            (DVec2::new(-50.0, 0.0), DVec2::new(50.0, 0.0)),
            (DVec2::new(-45.0, 20.0), DVec2::new(85.0, 20.0)),
            (0.002, 0.001),
        );

        // Zoom-in component: distance to the center shrank.
        assert!(camera.position.length() < len0, "combined gesture must zoom in");
        // Spin + translate component: the position moved off the pure-zoom ray.
        let radial = camera.position.normalize();
        let tangential = camera.position - pos0;
        // Not purely radial → spin/translate contributed.
        let perp = tangential - radial * tangential.dot(radial);
        assert!(perp.length() > 1.0, "combined gesture must spin/translate");
    }

    /// After a two-finger rotate is released, the spin velocity is captured
    /// into the [`InertiaController`] and coasts to a stop (inertia handoff).
    #[test]
    fn pinch_rotate_release_hands_off_to_spin_inertia() {
        use crate::inertia::{InertiaController, InertiaSample, InertiaState};

        let ctrl = CameraController::new(Ellipsoid::WGS84);
        let mut agg = CameraEventAggregator::new();
        let mut camera = create_test_camera();
        agg.pinch_start(DVec2::new(-50.0, 0.0), DVec2::new(50.0, 0.0));

        // One rotating frame: finger line turns CCW; capture the angle delta.
        pinch_frame(
            &mut agg,
            0.0,
            DVec2::new(-50.0, 0.0),
            DVec2::new(50.0, 0.0),
            DVec2::new(-40.0, 30.0),
            DVec2::new(40.0, -30.0),
        );
        let angle_delta = agg.pinch_angle_delta();
        assert!(angle_delta.abs() > 1e-6, "gesture must produce a rotation");
        ctrl.pinch_rotate(&mut camera, angle_delta);

        // On release the adapter captures the spin velocity (radians → pixels at
        // the boundary; here a representative pixel motion) into the inertia.
        let mut inertia = InertiaController::new();
        inertia.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(angle_delta * 2e4, 0.0));
        inertia.activate(Some(InertiaState::Spin));

        // Coasting continues the spin and decays frame over frame.
        let scale = 1e-4;
        let s1 = InertiaSample::new(0.9, 0.0, 0.0, 16.0);
        let p0 = camera.position;
        assert!(ctrl.coast_inertia(&mut camera, &mut inertia, InertiaState::Spin, DVec3::ZERO, &s1, scale));
        let step1 = (camera.position - p0).length();
        assert!(step1 > 0.0, "inertia must keep the globe spinning after release");

        let s2 = InertiaSample::new(0.9, 0.0, 0.0, 600.0);
        let p1 = camera.position;
        assert!(ctrl.coast_inertia(&mut camera, &mut inertia, InertiaState::Spin, DVec3::ZERO, &s2, scale));
        let step2 = (camera.position - p1).length();
        assert!(step2 < step1, "spin inertia must decay: {step2} !< {step1}");
    }
}
