//! M2.5 integration test — the `CameraControl` driving port.
//!
//! Exercises the port implementation (`cesium-bevy-render`'s
//! [`CameraControlImpl`] / [`CameraControlPort`]) that exposes the domain camera
//! algorithms as a programmable control surface. Two levels are covered:
//!
//! 1. **Port object** (ECS-free): `set_view` / `fly_to` / `look_at` / `zoom_*` /
//!    `get_camera_state` are driven directly and asserted in ECEF meters, with
//!    the `fly_to` landing error checked against the M2 gate of `< 1e-4` render
//!    units (`METERS_PER_RENDER_UNIT = 6378137`).
//! 2. **Bevy bridge**: a minimal app registers [`CameraControlPort`] +
//!    [`camera_control_port_system`] with a single [`CesiumCamera`] entity and
//!    proves a port command reaches the live entity (both an instantaneous
//!    `set_view` and a multi-frame great-arc `fly_to`).

use std::f64::consts::FRAC_PI_2;
use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

use cesium_bevy_render::camera::camera_control_port_system;
use cesium_bevy_render::{
    CameraControlImpl, CameraControlPort, CesiumCamera, METERS_PER_RENDER_UNIT,
};
use cesium_camera::Camera;
use cesium_geospatial::{Cartographic, Ellipsoid};
use cesium_ports_driving::CameraControl;
use cesium_scene_mode::SceneMode;
use glam::DVec3;

/// WGS84 maximum radius (meters) == `METERS_PER_RENDER_UNIT`.
const R: f64 = 6378137.0;

/// A camera one radius above the equator on the +X axis, looking at the center.
fn equator_camera() -> Camera {
    Camera::new(
        DVec3::new(R * 2.0, 0.0, 0.0),
        DVec3::new(-1.0, 0.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
    )
}

fn new_control() -> CameraControlImpl {
    CameraControlImpl::new(equator_camera(), Ellipsoid::WGS84)
}

/// Drives `control`'s active flight to completion with fixed `dt` steps,
/// bounded so a non-completing flight fails loudly rather than hanging.
fn run_flight_to_completion(control: &mut CameraControlImpl, dt: f64) {
    for _ in 0..10_000 {
        if !control.update(dt) {
            return;
        }
    }
    panic!("flight did not complete within 10_000 steps");
}

// ── Port object: fly_to ────────────────────────────────────────────────────

/// `fly_to` follows a great arc and lands exactly on the destination ECEF; the
/// residual is asserted against the M2 gate (< 1e-4 render units) and printed.
#[test]
fn fly_to_lands_on_destination_within_gate() {
    let mut control = new_control();
    let dest = Cartographic::from_degrees(-75.0, 40.0, 1_000_000.0);

    // duration_secs = 0.0 → the port derives duration/easing from the distance
    // (compute_flight_duration / select_flight_easing) and slerps the great arc.
    control.fly_to(dest, None, None, None, 0.0);
    assert!(control.is_flying(), "fly_to must start a flight");

    run_flight_to_completion(&mut control, 0.05);
    assert!(!control.is_flying(), "flight must be finished");

    let expected = Ellipsoid::WGS84.cartographic_to_cartesian(&dest);
    let state = control.get_camera_state();
    let err_m = (state.position - expected).length();
    let err_ru = err_m / METERS_PER_RENDER_UNIT;
    println!(
        "[fly_to] measured pos = ({:.6}, {:.6}, {:.6}) m; expected = ({:.6}, {:.6}, {:.6}) m; \
         err = {:.6e} m = {:.6e} render units",
        state.position.x, state.position.y, state.position.z,
        expected.x, expected.y, expected.z,
        err_m, err_ru,
    );
    assert!(
        err_ru < 1e-4,
        "fly_to landing error {err_ru} render units exceeds the 1e-4 gate ({err_m} m)"
    );
}

/// An explicit `duration_secs` is honored and the flight still lands on target.
#[test]
fn fly_to_honors_explicit_duration() {
    let mut control = new_control();
    let dest = Cartographic::from_degrees(139.0, 35.0, 2_000_000.0);
    control.fly_to(dest, Some(0.0), Some(-FRAC_PI_2), None, 2.0);

    // Half-way through a 2 s flight the camera must be strictly between the
    // endpoints (proves the duration is real, not an instant snap).
    control.update(1.0);
    let mid = control.get_camera_state().position;
    let start = DVec3::new(R * 2.0, 0.0, 0.0);
    let end = Ellipsoid::WGS84.cartographic_to_cartesian(&dest);
    assert!(control.is_flying(), "still flying at t=1s of a 2s flight");
    assert!((mid - start).length() > 1.0 && (mid - end).length() > 1.0, "mid must be in-flight");

    run_flight_to_completion(&mut control, 0.05);
    let err_ru = (control.get_camera_state().position - end).length() / METERS_PER_RENDER_UNIT;
    assert!(err_ru < 1e-4, "explicit-duration fly_to error {err_ru} ru");
}

// ── Port object: set_view / get_camera_state ───────────────────────────────

/// `set_view` places the camera at the cartographic position and reports the
/// pose back through `get_camera_state` (position + a downward pitch).
#[test]
fn set_view_positions_camera_and_state_reports_it() {
    let mut control = new_control();
    let carto = Cartographic::from_degrees(10.0, 20.0, 500_000.0);

    control.set_view(carto, 0.0, -FRAC_PI_2, 0.0);

    let expected = Ellipsoid::WGS84.cartographic_to_cartesian(&carto);
    let state = control.get_camera_state();
    let err_ru = (state.position - expected).length() / METERS_PER_RENDER_UNIT;
    assert!(err_ru < 1e-4, "set_view position error {err_ru} ru");
    // Looking straight down ⇒ pitch ≈ -π/2 and the direction opposes the normal.
    assert!(
        (state.pitch + FRAC_PI_2).abs() < 1e-6,
        "set_view pitch {} should be -π/2",
        state.pitch
    );
    let n = expected.normalize();
    assert!(
        state.direction.dot(-n) > 1.0 - 1e-9,
        "set_view must look straight down"
    );
    // A level camera has ~zero roll.
    assert!(state.roll.abs() < 1e-6, "set_view roll {}", state.roll);
}

// ── Port object: look_at ───────────────────────────────────────────────────

/// `look_at` holds the requested range and aims the camera at the target.
#[test]
fn look_at_holds_range_and_aims_at_target() {
    let mut control = new_control();
    let target = Cartographic::from_degrees(0.0, 0.0, 0.0);
    let range = 2_000_000.0;

    control.look_at(target, 0.0, -FRAC_PI_2, range);

    let target_ecef = Ellipsoid::WGS84.cartographic_to_cartesian(&target);
    let state = control.get_camera_state();
    let dist = (state.position - target_ecef).length();
    assert!(
        (dist - range).abs() / range < 1e-9,
        "look_at range {dist} != {range}"
    );
    let aim = (target_ecef - state.position).normalize();
    assert!(
        aim.dot(state.direction) > 1.0 - 1e-9,
        "look_at must aim at the target"
    );
}

// ── Port object: zoom ──────────────────────────────────────────────────────

/// `zoom_in` moves the camera forward along its view by the metric amount and
/// `zoom_out` moves it back, both measured against the surface distance.
#[test]
fn zoom_in_and_out_move_along_view() {
    let mut control = new_control();
    let len0 = control.get_camera_state().position.length();

    control.zoom_in(Some(100_000.0));
    let len1 = control.get_camera_state().position.length();
    assert!(
        (len0 - len1 - 100_000.0).abs() < 1.0,
        "zoom_in: {len0} → {len1} (expected -100000)"
    );

    control.zoom_out(Some(250_000.0));
    let len2 = control.get_camera_state().position.length();
    assert!(
        (len2 - len1 - 250_000.0).abs() < 1.0,
        "zoom_out: {len1} → {len2} (expected +250000)"
    );
}

// ── Bevy bridge ────────────────────────────────────────────────────────────

/// Builds a minimal app: `MinimalPlugins` (for `Time`), the [`CameraControlPort`]
/// resource, [`camera_control_port_system`] in `PostUpdate`, and one
/// [`CesiumCamera`] entity. No input plugins are needed because only the port
/// bridge is exercised.
fn bridge_app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<CameraControlPort>()
        .add_systems(PostUpdate, camera_control_port_system);
    let e = app
        .world_mut()
        .spawn(CesiumCamera::new(equator_camera(), SceneMode::Scene3D))
        .id();
    (app, e)
}

fn entity_position(app: &App, e: Entity) -> DVec3 {
    app.world().get::<CesiumCamera>(e).unwrap().camera.position
}

/// An instantaneous `set_view` through the port resource reaches the live
/// `CesiumCamera` entity after one `app.update()`.
#[test]
fn bridge_set_view_updates_cesium_camera_entity() {
    let (mut app, e) = bridge_app();
    let carto = Cartographic::from_degrees(-30.0, 15.0, 800_000.0);

    app.world_mut()
        .resource_mut::<CameraControlPort>()
        .set_view(carto, 0.0, -FRAC_PI_2, 0.0);
    app.update();

    let expected = Ellipsoid::WGS84.cartographic_to_cartesian(&carto);
    let err_ru = (entity_position(&app, e) - expected).length() / METERS_PER_RENDER_UNIT;
    assert!(
        err_ru < 1e-4,
        "bridge set_view error {err_ru} ru: {:?}",
        entity_position(&app, e)
    );
}

/// A great-arc `fly_to` driven through the port resource advances across frames
/// (fixed dt via `TimeUpdateStrategy`) and lands the `CesiumCamera` entity on the
/// destination within the 1e-4 render-unit gate.
#[test]
fn bridge_fly_to_flies_cesium_camera_entity_to_destination() {
    let (mut app, e) = bridge_app();
    // Fixed 50 ms frames so the flight completes deterministically.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        Duration::from_secs_f64(0.05),
    ));

    let dest = Cartographic::from_degrees(2.0, -33.0, 1_500_000.0);
    app.world_mut()
        .resource_mut::<CameraControlPort>()
        .fly_to(dest, None, None, None, 1.0);

    // 1 s flight at 50 ms/frame → 20 frames; run 40 for margin.
    for _ in 0..40 {
        app.update();
    }

    let expected = Ellipsoid::WGS84.cartographic_to_cartesian(&dest);
    let got = entity_position(&app, e);
    let err_m = (got - expected).length();
    let err_ru = err_m / METERS_PER_RENDER_UNIT;
    println!(
        "[bridge fly_to] entity pos = ({:.6}, {:.6}, {:.6}); err = {:.6e} m = {:.6e} ru",
        got.x, got.y, got.z, err_m, err_ru
    );
    assert!(
        err_ru < 1e-4,
        "bridge fly_to error {err_ru} render units exceeds the 1e-4 gate"
    );
    assert!(
        !app.world()
            .resource::<CameraControlPort>()
            .control()
            .is_flying(),
        "flight must have completed"
    );
}

/// With no command and no flight the bridge is inert: the entity's own pose is
/// preserved (the port tracks it rather than overwriting it).
#[test]
fn bridge_is_inert_without_commands() {
    let (mut app, e) = bridge_app();
    let before = entity_position(&app, e);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(before, entity_position(&app, e), "idle bridge must not move the camera");
}
