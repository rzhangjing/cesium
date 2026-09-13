use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;

use crate::camera::components::{CameraInputState, CesiumCamera};
use crate::METERS_PER_RENDER_UNIT;

/// Screen-space camera controller: orbit, zoom, pan via mouse and touch.
pub fn camera_controller_system(
    mut cameras: Query<&mut CesiumCamera>,
    mut input_state: ResMut<CameraInputState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    time: Res<Time>,
) {
    let dt = time.delta_secs() as f64;

    // --- Track mouse button state ---
    input_state.left_mouse_down = mouse_buttons.pressed(MouseButton::Left);
    input_state.right_mouse_down = mouse_buttons.pressed(MouseButton::Right);
    input_state.middle_mouse_down = mouse_buttons.pressed(MouseButton::Middle);

    // --- Accumulate mouse delta ---
    let mut total_delta = Vec2::ZERO;
    for ev in mouse_motion.read() {
        total_delta += ev.delta;
    }

    // --- Accumulate scroll ---
    let mut scroll_lines = 0.0_f32;
    let mut scroll_pixels = 0.0_f32;
    for ev in scroll_events.read() {
        match ev.unit {
            MouseScrollUnit::Line => scroll_lines += ev.y,
            MouseScrollUnit::Pixel => scroll_pixels += ev.y,
        }
    }

    let any_input = total_delta != Vec2::ZERO || scroll_lines != 0.0 || scroll_pixels != 0.0;
    if !any_input {
        return;
    }

    for mut cesium_cam in cameras.iter_mut() {
        let enable_collision = cesium_cam.enable_collision_detection;
        let min_dist = cesium_cam.minimum_zoom_distance;
        let max_dist = cesium_cam.maximum_zoom_distance;

        let cam = &mut cesium_cam.camera;

        // --- Orbit (left mouse drag) ---
        if input_state.left_mouse_down && total_delta != Vec2::ZERO {
            let orbit_speed = input_state.orbit_sensitivity as f64 * 0.005;
            let heading_delta = total_delta.x as f64 * orbit_speed;
            let pitch_delta = -total_delta.y as f64 * orbit_speed;

            cam.rotate_right(heading_delta);
            cam.rotate_up(pitch_delta);
        }

        // --- Zoom (right mouse drag + scroll wheel) ---
        if input_state.right_mouse_down && total_delta.y != 0.0 {
            let zoom_speed = input_state.zoom_sensitivity as f64 * 0.01;
            let zoom_amount = total_delta.y as f64 * zoom_speed * cam.position.length();
            cam.move_along(cam.direction, zoom_amount);
        }

        // Scroll wheel zoom
        let scroll_amount = scroll_lines as f64 * 100.0 + scroll_pixels as f64;
        if scroll_amount != 0.0 {
            let zoom_speed = input_state.zoom_sensitivity as f64 * 1000.0;
            let distance = scroll_amount * zoom_speed * (dt * 60.0).min(5.0);
            cam.move_along(cam.direction, distance);
        }

        // --- Pan (middle mouse drag) ---
        if input_state.middle_mouse_down && total_delta != Vec2::ZERO {
            let pan_speed = input_state.pan_sensitivity as f64 * 0.1;
            let pixel_size = cam.position.length() * 1e-6;
            let x_offset = -total_delta.x as f64 * pan_speed * pixel_size;
            let y_offset = total_delta.y as f64 * pan_speed * pixel_size;

            cam.position += cam.right * x_offset + cam.up * y_offset;
        }

        // --- Collision detection ---
        if enable_collision {
            let dist = cam.position.length();

            if dist < METERS_PER_RENDER_UNIT + min_dist {
                let clamped =
                    cam.position.normalize() * (METERS_PER_RENDER_UNIT + min_dist);
                cam.position = clamped;
            }
            if dist > max_dist {
                let clamped = cam.position.normalize() * max_dist;
                cam.position = clamped;
            }
        }
    }
}
