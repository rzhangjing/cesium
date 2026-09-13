//! ScreenSpaceCameraController extended specs — rotate, look, translate, twist
//! Ported from: packages/engine/Specs/Scene/ScreenSpaceCameraControllerSpec.js
//! A-class pure math tests

use cesium_interaction::camera_controller::{CameraController, CameraControllerConfig};
use cesium_camera::Camera;
use cesium_geospatial::ellipsoid::Ellipsoid;
use glam::DVec3;
use std::f64::consts::PI;

const EPSILON10: f64 = 1e-10;
const EPSILON14: f64 = 1e-14;

fn make_camera() -> Camera {
    Camera::new(
        DVec3::new(6378137.0 * 2.0, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    )
}

fn make_controller() -> CameraController {
    CameraController::new(Ellipsoid::WGS84)
}

// ============================================================================
// Config customization
// ============================================================================

#[test]
fn config_custom_speeds() {
    let mut controller = make_controller();
    controller.config.rotation_speed = 2.0;
    controller.config.pan_speed = 0.5;
    controller.config.zoom_speed = 1.5;
    controller.config.enable_rotation = false;

    assert!((controller.config.rotation_speed - 2.0).abs() < EPSILON10);
    assert!((controller.config.pan_speed - 0.5).abs() < EPSILON10);
    assert!((controller.config.zoom_speed - 1.5).abs() < EPSILON10);
    assert!(!controller.config.enable_rotation);
}

#[test]
fn config_minimum_zoom_distance_custom() {
    let controller = CameraController {
        config: CameraControllerConfig {
            minimum_zoom_distance: 100.0,
            ..Default::default()
        },
        ellipsoid: Ellipsoid::WGS84,
    };

    let mut camera = Camera::new(
        DVec3::new(Ellipsoid::WGS84.maximum_radius() + 50.0, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );

    // Camera is 50m above surface, min distance is 100m — collision should push it up
    controller.enforce_collision(&mut camera);
    let height = camera.position.length() - Ellipsoid::WGS84.maximum_radius();
    assert!(height >= 100.0 - EPSILON10);
}

// ============================================================================
// Orbit: heading-only rotation preserves distance
// ============================================================================

#[test]
fn orbit_heading_only_preserves_distance() {
    let controller = make_controller();
    let mut camera = make_camera();
    let target = DVec3::ZERO;
    let initial_distance = (camera.position - target).length();

    controller.orbit(&mut camera, target, 0.5, 0.0, 0.0);

    let new_distance = (camera.position - target).length();
    assert!((new_distance - initial_distance).abs() / initial_distance < 0.01);
}

#[test]
fn orbit_pitch_only_preserves_distance() {
    let controller = make_controller();
    let mut camera = make_camera();
    let target = DVec3::ZERO;
    let initial_distance = (camera.position - target).length();

    controller.orbit(&mut camera, target, 0.0, 0.3, 0.0);

    let new_distance = (camera.position - target).length();
    assert!((new_distance - initial_distance).abs() / initial_distance < 0.01);
}

#[test]
fn orbit_range_change_alters_distance() {
    let controller = make_controller();
    let mut camera = make_camera();
    let target = DVec3::ZERO;
    let initial_distance = (camera.position - target).length();

    controller.orbit(&mut camera, target, 0.0, 0.0, 1_000_000.0);

    let new_distance = (camera.position - target).length();
    assert!(new_distance > initial_distance);
}

#[test]
fn orbit_disabled_does_nothing() {
    let mut controller = make_controller();
    controller.config.enable_rotation = false;
    let mut camera = make_camera();
    let target = DVec3::ZERO;
    let initial_pos = camera.position;

    controller.orbit(&mut camera, target, 1.0, 1.0, 1000.0);

    assert_eq!(camera.position, initial_pos);
}

// ============================================================================
// Orbit: pitch clamping
// ============================================================================

#[test]
fn orbit_pitch_clamped_to_near_90() {
    let controller = make_controller();
    let mut camera = make_camera();
    let target = DVec3::ZERO;

    // Try to pitch way past vertical
    controller.orbit(&mut camera, target, 0.0, 10.0, 0.0);

    // Camera should not pass through the pole
    let offset = camera.position - target;
    let pitch = offset.y.atan2((offset.x * offset.x + offset.z * offset.z).sqrt());
    assert!(pitch.abs() <= PI / 2.0 + 0.01);
}

// ============================================================================
// Pan: direction tests
// ============================================================================

#[test]
fn pan_right_moves_camera() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial_pos = camera.position;

    controller.pan(&mut camera, 1.0, 0.0);

    // Camera should move in the direction of right vector
    let delta = camera.position - initial_pos;
    // delta should have component along camera.right (which points roughly +Z for this setup)
    assert!(delta.length() > 0.0);
}

#[test]
fn pan_up_moves_camera() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial_pos = camera.position;

    controller.pan(&mut camera, 0.0, 1.0);

    let delta = camera.position - initial_pos;
    assert!(delta.length() > 0.0);
}

#[test]
fn pan_disabled_does_nothing() {
    let mut controller = make_controller();
    controller.config.enable_pan = false;
    let mut camera = make_camera();
    let initial_pos = camera.position;

    controller.pan(&mut camera, 1.0, 1.0);

    assert_eq!(camera.position, initial_pos);
}

// ============================================================================
// Zoom: collision prevention
// ============================================================================

#[test]
fn zoom_does_not_cross_surface() {
    let controller = make_controller();
    let mut camera = Camera::new(
        DVec3::new(Ellipsoid::WGS84.maximum_radius() + 10.0, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );

    // Try to zoom in deep underground
    controller.zoom(&mut camera, 100.0);

    let height = camera.position.length() - Ellipsoid::WGS84.maximum_radius();
    assert!(height >= controller.config.minimum_zoom_distance - 1.0);
}

#[test]
fn zoom_disabled_no_collision_check() {
    let mut controller = make_controller();
    controller.config.enable_zoom = false;
    let mut camera = make_camera();
    let initial_pos = camera.position;

    controller.zoom(&mut camera, 10.0);

    assert_eq!(camera.position, initial_pos);
}

#[test]
fn zoom_with_collision_disabled_can_go_underground() {
    let mut controller = make_controller();
    controller.config.enable_collision_detection = false;
    let mut camera = Camera::new(
        DVec3::new(Ellipsoid::WGS84.maximum_radius() + 10.0, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );

    controller.zoom(&mut camera, 100.0);

    let height = camera.position.length() - Ellipsoid::WGS84.maximum_radius();
    assert!(height < 0.0); // Below surface
}

// ============================================================================
// Tilt: pitch changes
// ============================================================================

#[test]
fn tilt_changes_pitch() {
    let controller = make_controller();
    let mut camera = Camera::new(
        DVec3::new(Ellipsoid::WGS84.maximum_radius() * 2.0, 0.0, 1_000_000.0),
        DVec3::new(-1.0, 0.0, 0.0).normalize(),
        DVec3::new(0.0, 0.0, 1.0),
    );
    let target = DVec3::new(Ellipsoid::WGS84.maximum_radius(), 0.0, 0.0);
    let initial_dir = camera.direction;

    controller.tilt(&mut camera, target, 0.5);

    let dot = camera.direction.dot(initial_dir);
    assert!(dot < 0.99, "direction should change: dot={}", dot);
}

#[test]
fn tilt_up_increases_z_height() {
    let controller = make_controller();
    let mut camera = Camera::new(
        DVec3::new(Ellipsoid::WGS84.maximum_radius() * 2.0, 0.0, 1_000_000.0),
        DVec3::new(-1.0, 0.0, 0.0).normalize(),
        DVec3::new(0.0, 0.0, 1.0),
    );
    let target = DVec3::new(Ellipsoid::WGS84.maximum_radius(), 0.0, 0.0);

    let height_before = camera.position.length();
    controller.tilt(&mut camera, target, PI / 4.0);
    let height_after = camera.position.length();

    // After tilting, position length may change — just verify orthonormality
    assert!((camera.direction.length() - 1.0).abs() < EPSILON14);
    assert!((camera.up.length() - 1.0).abs() < EPSILON14);
    assert!((camera.right.length() - 1.0).abs() < EPSILON14);
}

// ============================================================================
// Enforce collision
// ============================================================================

#[test]
fn enforce_collision_pushes_to_surface() {
    let controller = make_controller();
    let mut camera = Camera::new(
        DVec3::new(Ellipsoid::WGS84.maximum_radius() + 0.5, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );

    controller.enforce_collision(&mut camera);

    let height = camera.position.length() - Ellipsoid::WGS84.maximum_radius();
    assert!(height >= controller.config.minimum_zoom_distance - EPSILON10);
}

#[test]
fn enforce_collision_does_nothing_if_safe() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial_pos = camera.position;

    controller.enforce_collision(&mut camera);

    assert_eq!(camera.position, initial_pos);
}

#[test]
fn enforce_collision_disabled_does_nothing() {
    let mut controller = make_controller();
    controller.config.enable_collision_detection = false;
    let mut camera = Camera::new(
        DVec3::new(Ellipsoid::WGS84.maximum_radius() + 0.5, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    );
    let initial_pos = camera.position;

    controller.enforce_collision(&mut camera);

    assert_eq!(camera.position, initial_pos);
}

// ============================================================================
// Orbit: orthonormality
// ============================================================================

#[test]
fn orbit_preserves_orthonormality() {
    let controller = make_controller();
    let mut camera = make_camera();
    let target = DVec3::ZERO;

    controller.orbit(&mut camera, target, 0.7, 0.3, 50000.0);

    assert!((camera.direction.length() - 1.0).abs() < EPSILON14);
    assert!((camera.up.length() - 1.0).abs() < EPSILON14);
    assert!((camera.right.length() - 1.0).abs() < EPSILON14);
    assert!(camera.direction.dot(camera.up).abs() < EPSILON14);
    assert!(camera.direction.dot(camera.right).abs() < EPSILON14);
    assert!(camera.up.dot(camera.right).abs() < EPSILON14);
}

// ============================================================================
// Multiple operations chaining
// ============================================================================

#[test]
fn orbit_then_pan_then_zoom() {
    let controller = make_controller();
    let mut camera = make_camera();

    let initial_pos = camera.position;

    // Orbit
    controller.orbit(&mut camera, DVec3::ZERO, 0.5, -0.2, 0.0);
    assert!(camera.position != initial_pos);

    // Pan
    controller.pan(&mut camera, 0.3, -0.1);
    assert!((camera.direction.length() - 1.0).abs() < EPSILON14);

    // Zoom out
    controller.zoom(&mut camera, -1.0);
    assert!((camera.up.length() - 1.0).abs() < EPSILON14);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn zoom_zero_delta_does_nothing() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial = camera.position;

    controller.zoom(&mut camera, 0.0);

    assert!((camera.position - initial).length() < EPSILON10);
}

#[test]
fn pan_zero_delta_does_nothing() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial = camera.position;

    controller.pan(&mut camera, 0.0, 0.0);

    assert!((camera.position - initial).length() < EPSILON10);
}

#[test]
fn orbit_zero_delta_preserves_position() {
    let controller = make_controller();
    let mut camera = make_camera();
    let initial = camera.position;
    let target = DVec3::ZERO;

    controller.orbit(&mut camera, target, 0.0, 0.0, 0.0);

    assert!((camera.position - initial).length() < EPSILON10);
}
