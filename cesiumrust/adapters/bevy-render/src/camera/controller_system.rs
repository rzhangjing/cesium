//! Screen-space camera controller system.
//!
//! Converts Bevy input events into pixel deltas and delegates **all** camera
//! math to the domain [`CameraController`]. The pixel→radian conversion is
//! computed here at the adapter boundary using the window's focal length,
//! keeping the domain resolution-independent and free of render-unit concerns.
//!
//! Zoom is based on **surface height** (`position.length() - ellipsoid.maximum_radius()`),
//! matching the domain algorithm and orbit_camera's proven feel.

use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use cesium_camera::Frustum;
use cesium_geospatial::Ellipsoid;
use cesium_interaction::{CameraController, CameraControllerConfig};

use crate::camera::components::{CameraInputState, CesiumCamera};

/// Screen-space camera controller: orbit, zoom, pan via mouse and touch.
///
/// All computation is delegated to the domain [`CameraController`]; this system
/// only converts Bevy input events into pixel deltas and computes the
/// pixel→radian scale factor at the adapter boundary.
pub fn camera_controller_system(
    mut cameras: Query<&mut CesiumCamera>,
    mut input_state: ResMut<CameraInputState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    windows: Query<&Window>,
) {
    // --- Track mouse button state ---
    input_state.left_mouse_down = mouse_buttons.pressed(MouseButton::Left);
    input_state.right_mouse_down = mouse_buttons.pressed(MouseButton::Right);
    input_state.middle_mouse_down = mouse_buttons.pressed(MouseButton::Middle);

    // --- Accumulate mouse delta (pixels) ---
    let mut total_delta = Vec2::ZERO;
    for ev in mouse_motion.read() {
        total_delta += ev.delta;
    }

    // --- Accumulate scroll (normalized to notches) ---
    let mut scroll_notches = 0.0_f64;
    for ev in scroll_events.read() {
        match ev.unit {
            MouseScrollUnit::Line => scroll_notches += ev.y as f64,
            MouseScrollUnit::Pixel => scroll_notches += (ev.y as f64) / 100.0,
        }
    }

    let any_input = total_delta != Vec2::ZERO || scroll_notches != 0.0;
    if !any_input {
        return;
    }

    // Window height for the pixel→radian focal-length conversion.
    let win_h = windows
        .get_single()
        .map(|w| w.height() as f64)
        .unwrap_or(720.0);

    // Sensitivity multipliers (default to 1.0 when the resource is zero-initialized).
    let orbit_sens = non_zero_or(input_state.orbit_sensitivity, 1.0);
    let zoom_sens = non_zero_or(input_state.zoom_sensitivity, 1.0);
    let pan_sens = non_zero_or(input_state.pan_sensitivity, 1.0);

    for mut cesium_cam in cameras.iter_mut() {
        // Extract config values before borrowing `camera` mutably.
        let enable_collision = cesium_cam.enable_collision_detection;
        let min_zoom_dist = cesium_cam.minimum_zoom_distance;
        let max_zoom_dist = cesium_cam.maximum_zoom_distance;
        let cam = &mut cesium_cam.camera;

        // Build a domain controller from the component's configuration.
        let config = CameraControllerConfig {
            minimum_zoom_distance: min_zoom_dist,
            maximum_zoom_distance: max_zoom_dist,
            rotation_speed: orbit_sens,
            pan_speed: pan_sens,
            zoom_speed: zoom_sens,
            enable_rotation: true,
            enable_pan: true,
            enable_zoom: true,
            enable_collision_detection: enable_collision,
        };
        let ctrl = CameraController {
            config,
            ellipsoid: Ellipsoid::WGS84,
        };

        // Surface height (meters above the ellipsoid) — the correct reference
        // for pixel→radian conversion and zoom/pan scaling.
        let surface_height = (cam.position.length() - Ellipsoid::WGS84.maximum_radius())
            .abs()
            .max(1.0);

        // Focal length in pixels: f = (H/2) / tan(fov/2).
        let fov = match &cam.frustum {
            Frustum::Perspective(f) => f.fov,
            Frustum::Orthographic(_) => std::f64::consts::FRAC_PI_3,
        };
        let focal = (win_h * 0.5) / (fov * 0.5).tan();

        // Pixel → radian at the surface (grab-the-globe scale factor).
        let pixel_to_radian = surface_height / focal;

        // --- Orbit / Spin (left mouse drag) ---
        if input_state.left_mouse_down && total_delta != Vec2::ZERO {
            let heading = total_delta.x as f64 * pixel_to_radian;
            let pitch = -total_delta.y as f64 * pixel_to_radian;
            ctrl.spin(cam, heading, pitch);
        }

        // --- Zoom (right mouse drag): vertical pixels → fraction of window ---
        if input_state.right_mouse_down && total_delta.y != 0.0 {
            let zoom_delta = total_delta.y as f64 / win_h;
            ctrl.zoom(cam, zoom_delta);
        }

        // --- Zoom (scroll wheel): one notch = one zoom unit ---
        // At surface height 1e6 m with zoom_speed=1: Δheight = -1e5 m per notch.
        if scroll_notches != 0.0 {
            ctrl.zoom(cam, scroll_notches);
        }

        // --- Pan (middle mouse drag) ---
        if input_state.middle_mouse_down && total_delta != Vec2::ZERO {
            let pan_x = total_delta.x as f64 / focal;
            let pan_y = -total_delta.y as f64 / focal;
            ctrl.pan(cam, pan_x, pan_y);
        }

        // --- Collision detection (delegated to domain) ---
        ctrl.enforce_collision(cam);
    }
}

/// Returns `value` as f64, or `fallback` when the value is zero.
#[inline]
fn non_zero_or(value: f32, fallback: f64) -> f64 {
    if value != 0.0 {
        value as f64
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_zero_or_returns_value_when_nonzero() {
        assert!((non_zero_or(2.5, 1.0) - 2.5).abs() < 1e-12);
    }

    #[test]
    fn non_zero_or_returns_fallback_when_zero() {
        assert!((non_zero_or(0.0, 1.0) - 1.0).abs() < 1e-12);
    }

    /// Verification: at surface height 1e6 m, one scroll notch (delta=1.0)
    /// with zoom_speed=1.0 produces zoom_amount = height * 0.1 = 1e5 m.
    #[test]
    fn zoom_at_1e6_height_produces_1e5_per_notch() {
        let config = CameraControllerConfig {
            zoom_speed: 1.0,
            ..Default::default()
        };
        let ctrl = CameraController {
            config,
            ellipsoid: Ellipsoid::WGS84,
        };
        let mut cam = cesium_camera::Camera::new(
            glam::DVec3::new(6378137.0 + 1_000_000.0, 0.0, 0.0),
            glam::DVec3::new(-1.0, 0.0, 0.0),
            glam::DVec3::new(0.0, 0.0, 1.0),
        );
        let initial_height = cam.position.length() - Ellipsoid::WGS84.maximum_radius();
        // Zoom in (positive delta = move toward target).
        ctrl.zoom(&mut cam, 1.0);
        let new_height = cam.position.length() - Ellipsoid::WGS84.maximum_radius();
        let delta_h = new_height - initial_height;
        // Should be approximately -1e5 (moved 1e5 closer to the surface).
        assert!(
            (delta_h - (-100_000.0)).abs() < 1.0,
            "expected Δheight ≈ -1e5, got {delta_h}"
        );
    }
}
