//! Camera flight animations (flyTo, lookAt).
//!
//! Maps to CesiumJS `Scene/Camera.js` flight methods:
//! - `Camera.flyTo`
//! - `Camera.flyToBoundingSphere`
//! - `Camera.flyHome`
//! - `Camera.lookAt`
//! - `Camera.setView`

use cesium_camera::{Camera, EasingFunction};
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::{BoundingSphere, HeadingPitchRange};
use glam::DVec3;

/// Options for a camera flight.
#[derive(Debug, Clone)]
pub struct FlightOptions {
    /// Target position (ECEF).
    pub destination: DVec3,
    /// Target heading in radians.
    pub heading: Option<f64>,
    /// Target pitch in radians.
    pub pitch: Option<f64>,
    /// Target roll in radians.
    pub roll: Option<f64>,
    /// Target direction (overrides heading/pitch).
    pub direction: Option<DVec3>,
    /// Target up vector.
    pub up: Option<DVec3>,
    /// Flight duration in seconds.
    pub duration: f64,
    /// Easing function.
    pub easing: EasingFunction,
}

impl Default for FlightOptions {
    fn default() -> Self {
        Self {
            destination: DVec3::ZERO,
            heading: None,
            pitch: None,
            roll: None,
            direction: None,
            up: None,
            duration: 3.0,
            easing: EasingFunction::SinusoidalInOut,
        }
    }
}

/// A camera flight path animation.
#[derive(Debug, Clone)]
pub struct CameraFlight {
    /// Start position.
    pub start_position: DVec3,
    /// End position.
    pub end_position: DVec3,
    /// Start direction.
    pub start_direction: DVec3,
    /// End direction.
    pub end_direction: DVec3,
    /// Start up vector.
    pub start_up: DVec3,
    /// End up vector.
    pub end_up: DVec3,
    /// Total duration in seconds.
    pub duration: f64,
    /// Elapsed time in seconds.
    pub elapsed: f64,
    /// Whether the flight is complete.
    pub complete: bool,
    /// Easing function for the flight.
    pub easing: EasingFunction,
}

impl CameraFlight {
    /// Creates a flyTo animation.
    ///
    /// # Arguments
    /// * `camera` - Current camera state
    /// * `destination` - Target position (ECEF)
    /// * `direction` - Target look direction (optional)
    /// * `duration` - Flight duration in seconds
    pub fn fly_to(
        camera: &Camera,
        destination: DVec3,
        direction: Option<DVec3>,
        up: Option<DVec3>,
        duration: f64,
    ) -> Self {
        let end_direction = direction.unwrap_or_else(|| {
            // Default: look at the center of the Earth from destination
            -destination.normalize()
        });
        let end_up = up.unwrap_or(DVec3::Z);

        Self {
            start_position: camera.position,
            end_position: destination,
            start_direction: camera.direction,
            end_direction: end_direction.normalize(),
            start_up: camera.up,
            end_up: end_up.normalize(),
            duration: duration.max(0.001),
            elapsed: 0.0,
            complete: false,
            easing: EasingFunction::SinusoidalInOut,
        }
    }

    /// Creates a flyTo from cartographic coordinates.
    pub fn fly_to_cartographic(
        camera: &Camera,
        destination: &Cartographic,
        ellipsoid: &Ellipsoid,
        duration: f64,
    ) -> Self {
        let ecef = ellipsoid.cartographic_to_cartesian(destination);
        let direction = -ecef.normalize(); // Look down
        Self::fly_to(camera, ecef, Some(direction), Some(DVec3::Z), duration)
    }

    /// Updates the flight by a time delta and returns the interpolated camera state.
    ///
    /// # Arguments
    /// * `dt` - Time delta in seconds
    ///
    /// # Returns
    /// The interpolated camera position, direction, and up vector
    pub fn update(&mut self, dt: f64) -> Option<(DVec3, DVec3, DVec3)> {
        if self.complete {
            return None;
        }

        self.elapsed += dt;
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);

        if t >= 1.0 {
            self.complete = true;
        }

        // Apply easing function
        let t_eased = self.easing.evaluate(t);

        let position = self.start_position.lerp(self.end_position, t_eased);
        let direction = self.start_direction.lerp(self.end_direction, t_eased).normalize();
        let up = self.start_up.lerp(self.end_up, t_eased).normalize();

        Some((position, direction, up))
    }

    /// Applies the current flight state to a camera.
    pub fn apply_to_camera(&mut self, camera: &mut Camera, dt: f64) -> bool {
        if let Some((position, direction, up)) = self.update(dt) {
            camera.position = position;
            camera.direction = direction;
            camera.right = direction.cross(up).normalize();
            camera.up = camera.right.cross(direction).normalize();
            !self.complete
        } else {
            false
        }
    }

    /// Returns the progress (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Creates a flight from full options.
    /// Maps to `Camera.flyTo` with full options
    pub fn fly_to_with_options(camera: &Camera, options: &FlightOptions) -> Self {
        let end_direction = if let Some(dir) = options.direction {
            dir.normalize()
        } else {
            // Compute from heading/pitch or default to looking at center
            -options.destination.normalize()
        };
        let end_up = options.up.unwrap_or(DVec3::Z).normalize();

        Self {
            start_position: camera.position,
            end_position: options.destination,
            start_direction: camera.direction,
            end_direction,
            start_up: camera.up,
            end_up,
            duration: options.duration.max(0.001),
            elapsed: 0.0,
            complete: false,
            easing: options.easing,
        }
    }

    /// Creates a flight to view a bounding sphere.
    /// Maps to `Camera.flyToBoundingSphere`
    pub fn fly_to_bounding_sphere(
        camera: &Camera,
        sphere: &BoundingSphere,
        offset: Option<&HeadingPitchRange>,
        duration: f64,
    ) -> Self {
        let default_offset = HeadingPitchRange::new(0.0, -std::f64::consts::FRAC_PI_4, 0.0);
        let offset = offset.unwrap_or(&default_offset);

        // Compute range if not specified
        let range = if offset.range > 0.0 {
            offset.range
        } else {
            // Default: compute from sphere radius and FOV
            let fov = match &camera.frustum {
                cesium_camera::Frustum::Perspective(f) => f.fov,
                cesium_camera::Frustum::Orthographic(_) => std::f64::consts::FRAC_PI_3,
            };
            sphere.radius / (fov * 0.5).sin().max(0.001)
        };

        // Compute destination from sphere center + offset
        let cos_pitch = offset.pitch.cos();
        let dest_offset = DVec3::new(
            range * cos_pitch * offset.heading.cos(),
            range * cos_pitch * offset.heading.sin(),
            range * offset.pitch.sin(),
        );
        let destination = sphere.center + dest_offset;
        let direction = (sphere.center - destination).normalize();

        Self {
            start_position: camera.position,
            end_position: destination,
            start_direction: camera.direction,
            end_direction: direction,
            start_up: camera.up,
            end_up: DVec3::Z,
            duration: duration.max(0.001),
            elapsed: 0.0,
            complete: false,
            easing: EasingFunction::SinusoidalInOut,
        }
    }

    /// Creates a flight to the default home view.
    /// Maps to `Camera.flyHome`
    pub fn fly_home(camera: &Camera, ellipsoid: &Ellipsoid, duration: f64) -> Self {
        let destination = Camera::default_home_position(ellipsoid);
        let direction = -destination.normalize();
        Self::fly_to(camera, destination, Some(direction), Some(DVec3::Z), duration)
    }
}

/// Computes a "lookAt" camera orientation.
///
/// Positions the camera to look at a target from a given offset.
///
/// # Arguments
/// * `target` - The point to look at (ECEF)
/// * `offset` - Offset from target (in local ENU or world coordinates)
///
/// # Returns
/// Camera position, direction, and up vector
pub fn compute_look_at(target: DVec3, offset: DVec3) -> (DVec3, DVec3, DVec3) {
    let position = target + offset;
    let direction = (target - position).normalize();

    // Choose up vector that's not parallel to direction
    let world_up = if direction.dot(DVec3::Z).abs() > 0.99 {
        DVec3::Y
    } else {
        DVec3::Z
    };

    let right = direction.cross(world_up).normalize();
    let up = right.cross(direction).normalize();

    (position, direction, up)
}

/// Computes a camera view looking down at a cartographic position from a given height.
///
/// # Arguments
/// * `cartographic` - The position to look at
/// * `height` - Height above the surface (meters)
/// * `heading` - Camera heading (radians)
/// * `pitch` - Camera pitch (radians, negative = looking down)
/// * `ellipsoid` - The ellipsoid
///
/// # Returns
/// Camera position, direction, and up vector
pub fn compute_set_view(
    cartographic: &Cartographic,
    height: f64,
    heading: f64,
    pitch: f64,
    ellipsoid: &Ellipsoid,
) -> (DVec3, DVec3, DVec3) {
    // Position above the target
    let target_carto = Cartographic::from_radians(
        cartographic.longitude,
        cartographic.latitude,
        height,
    );
    let position = ellipsoid.cartographic_to_cartesian(&target_carto);

    // Surface normal at the target
    let surface_normal = position.normalize();

    // Compute direction from pitch and heading
    // pitch = -PI/2 means looking straight down
    let pitch_from_nadir = pitch + std::f64::consts::FRAC_PI_2;

    // Direction: rotate surface normal by pitch
    let east = DVec3::Z.cross(surface_normal).normalize();
    let north = surface_normal.cross(east).normalize();

    // Apply heading rotation to get the tilt plane
    let tilt_dir = north * heading.cos() + east * heading.sin();

    // Direction is a combination of looking down and tilting
    let direction = (-surface_normal * pitch_from_nadir.cos() + tilt_dir * pitch_from_nadir.sin())
        .normalize();

    let right = direction.cross(surface_normal).normalize();
    let up = right.cross(direction).normalize();

    (position, direction, up)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smooth step interpolation (Hermite) - test helper.
    fn smoothstep(t: f64) -> f64 {
        t * t * (3.0 - 2.0 * t)
    }

    fn create_test_camera() -> Camera {
        Camera::new(
            DVec3::new(6378137.0 * 3.0, 0.0, 0.0),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
        )
    }

    #[test]
    fn test_camera_flight_creation() {
        let camera = create_test_camera();
        let destination = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);

        let flight = CameraFlight::fly_to(&camera, destination, None, None, 2.0);

        assert_eq!(flight.start_position, camera.position);
        assert_eq!(flight.end_position, destination);
        assert_eq!(flight.duration, 2.0);
        assert!(!flight.complete);
    }

    #[test]
    fn test_camera_flight_update() {
        let camera = create_test_camera();
        let destination = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);

        let mut flight = CameraFlight::fly_to(&camera, destination, None, None, 2.0);

        // At t=0
        let (pos, _, _) = flight.update(0.0).unwrap();
        assert!((pos - camera.position).length() < 1.0);

        // At t=1 (halfway)
        let (pos, _, _) = flight.update(1.0).unwrap();
        let midpoint = (camera.position + destination) / 2.0;
        assert!((pos - midpoint).length() / midpoint.length() < 0.01);

        // At t=2 (end)
        let (pos, _, _) = flight.update(1.0).unwrap();
        assert!((pos - destination).length() < 1.0);
        assert!(flight.complete);
    }

    #[test]
    fn test_camera_flight_progress() {
        let camera = create_test_camera();
        let destination = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);

        let mut flight = CameraFlight::fly_to(&camera, destination, None, None, 4.0);

        flight.update(1.0);
        assert!((flight.progress() - 0.25).abs() < 1e-10);

        flight.update(1.0);
        assert!((flight.progress() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_camera_flight_apply() {
        let camera = create_test_camera();
        let destination = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);

        let mut flight = CameraFlight::fly_to(&camera, destination, None, None, 1.0);
        let mut camera = camera;

        // Apply full duration
        let still_flying = flight.apply_to_camera(&mut camera, 1.0);
        assert!(!still_flying); // Flight complete
        assert!((camera.position - destination).length() < 1.0);
    }

    #[test]
    fn test_compute_look_at() {
        let target = DVec3::new(6378137.0, 0.0, 0.0);
        let offset = DVec3::new(1000000.0, 0.0, 0.0);

        let (position, direction, up) = compute_look_at(target, offset);

        // Position should be target + offset
        assert!((position - (target + offset)).length() < 1e-6);

        // Direction should point from position to target
        let expected_dir = (target - position).normalize();
        assert!((direction - expected_dir).length() < 1e-10);

        // Up should be perpendicular to direction
        assert!(direction.dot(up).abs() < 1e-10);
    }

    #[test]
    fn test_compute_set_view() {
        let carto = Cartographic::from_radians(0.0, 0.0, 0.0);
        let height = 1000000.0;

        let (position, direction, _up) = compute_set_view(
            &carto,
            height,
            0.0,
            -std::f64::consts::FRAC_PI_2, // Looking straight down
            &Ellipsoid::WGS84,
        );

        // Position should be at the given height above the equator/prime meridian
        let pos_height = position.length() - Ellipsoid::WGS84.maximum_radius();
        assert!((pos_height - height).abs() / height < 0.01);

        // Direction should be roughly towards the center (looking down)
        let to_center = -position.normalize();
        assert!(direction.dot(to_center) > 0.9);
    }

    #[test]
    fn test_smoothstep() {
        assert!((smoothstep(0.0)).abs() < 1e-10);
        assert!((smoothstep(1.0) - 1.0).abs() < 1e-10);
        assert!((smoothstep(0.5) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_fly_to_cartographic() {
        let camera = create_test_camera();
        let dest = Cartographic::from_radians(0.5, 0.3, 10000.0);

        let flight = CameraFlight::fly_to_cartographic(&camera, &dest, &Ellipsoid::WGS84, 3.0);

        assert_eq!(flight.duration, 3.0);
        assert!(!flight.complete);
        // End position should be on the ellipsoid at the given cartographic
        let expected_pos = Ellipsoid::WGS84.cartographic_to_cartesian(&dest);
        assert!((flight.end_position - expected_pos).length() < 1.0);
    }
}
