//! Camera controller for orbit, pan, and zoom interactions.
//!
//! Maps to CesiumJS `Scene/ScreenSpaceCameraController.js`

use cesium_camera::Camera;
use cesium_geospatial::ellipsoid::Ellipsoid;
use glam::DVec3;

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
}
