//! Scene/ScreenSpaceCameraController → Rust integration tests.
//!
//! Maps to CesiumJS:
//! - Scene/ScreenSpaceCameraController.js (orbit, pan, zoom, tilt, collision)
//!
//! A-class tests: CameraController orbit/pan/zoom/tilt/enforce_collision,
//! CameraControllerConfig defaults, rotate_around_axis (Rodrigues).
//! C-class omitted: DOM events, pointer events, touch gestures, canvas.

use cesium_interaction::camera_controller::{CameraController, CameraControllerConfig};
use cesium_camera::Camera;
use cesium_geospatial::ellipsoid::Ellipsoid;
use glam::DVec3;

fn make_camera() -> Camera {
    // Camera at 2x Earth radius on +X axis, looking towards origin
    Camera::new(
        DVec3::new(6378137.0 * 2.0, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    )
}

fn make_controller() -> CameraController {
    CameraController::new(Ellipsoid::WGS84)
}

// === Config ===

#[test]
fn config_defaults() {
    let config = CameraControllerConfig::default();
    assert!((config.minimum_zoom_distance - 1.0).abs() < 1e-10);
    assert!(config.maximum_zoom_distance.is_infinite());
    assert!((config.rotation_speed - 1.0).abs() < 1e-10);
    assert!((config.pan_speed - 1.0).abs() < 1e-10);
    assert!((config.zoom_speed - 1.0).abs() < 1e-10);
    assert!(config.enable_rotation);
    assert!(config.enable_pan);
    assert!(config.enable_zoom);
    assert!(config.enable_collision_detection);
}

#[test]
fn controller_creation() {
    let controller = make_controller();
    assert!(controller.config.enable_rotation);
    assert!(controller.config.enable_pan);
    assert!(controller.config.enable_zoom);
}

// === Zoom ===

#[test]
fn zoom_in_decreases_distance() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial = camera.position.length();
    controller.zoom(&mut camera, 1.0);
    assert!(camera.position.length() < initial);
}

#[test]
fn zoom_out_increases_distance() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial = camera.position.length();
    controller.zoom(&mut camera, -1.0);
    assert!(camera.position.length() > initial);
}

#[test]
fn zoom_disabled_no_change() {
    let mut controller = make_controller();
    controller.config.enable_zoom = false;
    let mut camera = make_camera();
    let initial = camera.position;
    controller.zoom(&mut camera, 1.0);
    assert_eq!(camera.position, initial);
}

#[test]
fn zoom_collision_prevents_underground() {
    let controller = make_controller();
    // Camera very close to surface
    let mut camera = Camera::new(
        DVec3::new(6378137.0 + 5.0, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );
    // Zoom in aggressively
    controller.zoom(&mut camera, 100.0);
    let height = camera.position.length() - Ellipsoid::WGS84.maximum_radius();
    // Should not go below minimum_zoom_distance
    assert!(height >= controller.config.minimum_zoom_distance - 1.0);
}

// === Pan ===

#[test]
fn pan_moves_camera() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial = camera.position;
    controller.pan(&mut camera, 1.0, 0.0);
    assert_ne!(camera.position, initial);
}

#[test]
fn pan_disabled_no_change() {
    let mut controller = make_controller();
    controller.config.enable_pan = false;
    let mut camera = make_camera();
    let initial = camera.position;
    controller.pan(&mut camera, 1.0, 1.0);
    assert_eq!(camera.position, initial);
}

#[test]
fn pan_vertical_moves_differently() {
    let controller = make_controller();
    let mut cam_h = make_camera();
    let mut cam_v = make_camera();
    controller.pan(&mut cam_h, 1.0, 0.0);
    controller.pan(&mut cam_v, 0.0, 1.0);
    // Horizontal and vertical pan should produce different positions
    assert_ne!(cam_h.position, cam_v.position);
}

// === Orbit ===

#[test]
fn orbit_preserves_distance() {
    let controller = make_controller();
    let mut camera = make_camera();
    let target = DVec3::ZERO;
    let initial_distance = (camera.position - target).length();
    controller.orbit(&mut camera, target, 0.1, 0.0, 0.0);
    let new_distance = (camera.position - target).length();
    assert!((new_distance - initial_distance).abs() / initial_distance < 0.01);
}

#[test]
fn orbit_disabled_no_change() {
    let mut controller = make_controller();
    controller.config.enable_rotation = false;
    let mut camera = make_camera();
    let initial = camera.position;
    controller.orbit(&mut camera, DVec3::ZERO, 0.5, 0.5, 0.0);
    assert_eq!(camera.position, initial);
}

#[test]
fn orbit_changes_heading() {
    let controller = make_controller();
    let mut camera = make_camera();
    let target = DVec3::ZERO;
    let initial_pos = camera.position;
    controller.orbit(&mut camera, target, 0.3, 0.0, 0.0);
    // Position should change (heading rotation)
    assert!((camera.position - initial_pos).length() > 1.0);
}

#[test]
fn orbit_zoom_changes_range() {
    let controller = make_controller();
    let mut camera = make_camera();
    let target = DVec3::ZERO;
    let initial_distance = (camera.position - target).length();
    controller.orbit(&mut camera, target, 0.0, 0.0, 1000.0);
    let new_distance = (camera.position - target).length();
    // Positive delta_range = zoom out
    assert!(new_distance > initial_distance);
}

// === Tilt ===

#[test]
fn tilt_changes_position() {
    let controller = make_controller();
    // Camera offset from target must not be parallel to surface normal
    let mut camera = Camera::new(
        DVec3::new(6378137.0 * 1.5, 6378137.0 * 0.5, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );
    let target = DVec3::new(6378137.0, 0.0, 0.0); // Surface point
    let initial = camera.position;
    controller.tilt(&mut camera, target, 0.2);
    assert!((camera.position - initial).length() > 1.0);
}

// === Collision ===

#[test]
fn enforce_collision_pushes_up() {
    let controller = make_controller();
    let mut camera = Camera::new(
        DVec3::new(6378137.0 + 0.5, 0.0, 0.0), // Below min distance
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );
    controller.enforce_collision(&mut camera);
    let height = camera.position.length() - Ellipsoid::WGS84.maximum_radius();
    assert!(height >= controller.config.minimum_zoom_distance - 0.01);
}

#[test]
fn enforce_collision_disabled_no_change() {
    let mut controller = make_controller();
    controller.config.enable_collision_detection = false;
    let mut camera = Camera::new(
        DVec3::new(6378137.0 + 0.5, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );
    let initial = camera.position;
    controller.enforce_collision(&mut camera);
    assert_eq!(camera.position, initial);
}

#[test]
fn enforce_collision_high_altitude_no_change() {
    let controller = make_controller();
    let mut camera = make_camera(); // At 2x radius, well above surface
    let initial = camera.position;
    controller.enforce_collision(&mut camera);
    assert_eq!(camera.position, initial);
}
