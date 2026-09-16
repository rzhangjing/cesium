//! Two-finger touch → camera gesture adapter (M2.6, adapter half).
//!
//! Bridges Bevy [`TouchInput`] events to the domain two-finger pinch API built
//! in M2.6 phase 1 ([`CameraEventAggregator::pinch_move`] +
//! [`CameraController::pinch_zoom`] / [`CameraController::pinch_rotate`] /
//! [`CameraController::pinch_translate`]). All camera math lives in the domain;
//! this system only
//!
//! 1. tracks the two active fingers from the raw touch stream,
//! 2. feeds the domain aggregator once per frame (after `agg.reset(t)`), and
//! 3. computes the pixel→world scale factors at the adapter boundary, mirroring
//!    the focal-length conversion in [`super::controller_system`] so the touch
//!    path shares the mouse path's grab-the-globe magnitude.
//!
//! # Gesture mapping (CesiumJS `ScreenSpaceCameraController`)
//! | Two-finger gesture | Domain metric | Camera action |
//! |--------------------|---------------|---------------|
//! | pinch (spread/close) | `pinch_distance_delta` (px) | `pinch_zoom` |
//! | rotate (turn about midpoint) | `pinch_angle_delta` (rad) | `pinch_rotate` → spin |
//! | drag (translate together) | `pinch_midpoint_delta` (px) | `pinch_translate` → pan |
//!
//! # Boundary scales (resolution independence stays in the domain)
//! * `zoom_scale = 1 / window_height` — finger-separation pixels become a
//!   fraction of the window height, the same unit the right-drag zoom path
//!   feeds [`CameraController::zoom`].
//! * `translate_scale = 1 / focal`, `focal = (H/2) / tan(fov/2)` — midpoint
//!   pixels become the pan delta the middle-drag path feeds
//!   [`CameraController::pan`].
//! * `pinch_rotate` takes radians directly (no pixel scale).
//!
//! The system is inert without touch input, so registering it does not perturb
//! the keyboard/mouse path.

use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use cesium_camera::Frustum;
use cesium_geospatial::Ellipsoid;
use cesium_interaction::{CameraController, CameraControllerConfig, CameraEventAggregator};
use glam::DVec2;

use crate::camera::components::{CameraInputState, CesiumCamera};

/// Per-frame bridge between Bevy [`TouchInput`] events and the domain
/// [`CameraEventAggregator`].
///
/// Holds the aggregator plus the minimal finger bookkeeping needed to produce a
/// per-frame delta: the aggregator re-seeds its pinch *start* on the first
/// `pinch_move` after a `reset`, so we seed it with the previous frame's finger
/// pair and extend it with the current pair, making the reported deltas equal
/// exactly this frame's two-finger motion.
#[derive(Resource)]
pub struct TouchCameraState {
    /// Domain aggregator accumulating the two-finger pinch metrics.
    pub agg: CameraEventAggregator,
    /// The (up to two) actively tracked fingers as `(id, position)` in screen
    /// pixels, kept in ascending-id order for a stable finger-line direction.
    fingers: Vec<(u64, Vec2)>,
    /// The previous frame's finger pair, used to seed the aggregator's
    /// per-frame start.
    prev_pair: Option<(DVec2, DVec2)>,
}

impl Default for TouchCameraState {
    fn default() -> Self {
        Self {
            agg: CameraEventAggregator::new(),
            fingers: Vec::new(),
            prev_pair: None,
        }
    }
}

/// Two-finger touch camera control: pinch → zoom, rotate → spin, drag → pan.
///
/// Reads the raw [`TouchInput`] event stream, maintains the two-finger pinch in
/// the domain [`CameraEventAggregator`], and delegates every camera transform to
/// the domain [`CameraController`]. The pixel→world conversion is computed here
/// at the adapter boundary (see the module docs).
pub fn camera_touch_system(
    mut cameras: Query<&mut CesiumCamera>,
    input_state: Res<CameraInputState>,
    mut touch_state: ResMut<TouchCameraState>,
    mut touch_events: EventReader<TouchInput>,
    time: Res<Time>,
    windows: Query<&Window>,
) {
    // --- 1. Per-frame reset: the aggregator re-seeds its pinch start. ---
    touch_state.agg.reset(time.elapsed_secs_f64());

    // --- 2. Fold this frame's raw touch events into the tracked finger set. ---
    for ev in touch_events.read() {
        match ev.phase {
            TouchPhase::Started => {
                if !touch_state.fingers.iter().any(|(id, _)| *id == ev.id) {
                    touch_state.fingers.push((ev.id, ev.position));
                }
            }
            TouchPhase::Moved => {
                if let Some(slot) = touch_state
                    .fingers
                    .iter_mut()
                    .find(|(id, _)| *id == ev.id)
                {
                    slot.1 = ev.position;
                }
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                touch_state.fingers.retain(|(id, _)| *id != ev.id);
            }
        }
    }
    // Stable order (by id) so the finger-line angle never flips between frames;
    // only the first two fingers drive the pinch.
    touch_state.fingers.sort_by_key(|(id, _)| *id);
    touch_state.fingers.truncate(2);

    // Snapshot the current + previous pairs as owned locals so the aggregator
    // borrow below stays disjoint from the finger bookkeeping.
    let pair = two_fingers(&touch_state.fingers);
    let prev = touch_state.prev_pair;

    // --- 3. Drive the aggregator: pinch lifecycle, then seed-with-prev and
    //        extend-with-current so the deltas are this frame's motion. ---
    let mut new_prev: Option<(DVec2, DVec2)> = None;
    {
        let agg = &mut touch_state.agg;
        match pair {
            Some((f1, f2)) => {
                if !agg.is_pinching() {
                    agg.pinch_start(f1, f2);
                }
                if let Some((p1, p2)) = prev {
                    agg.pinch_move(p1, p2);
                }
                agg.pinch_move(f1, f2);
                new_prev = Some((f1, f2));
            }
            None => {
                if agg.is_pinching() {
                    agg.pinch_end();
                }
            }
        }
    }
    touch_state.prev_pair = new_prev;

    // A two-finger pinch must be active to move the camera.
    let Some((_, _)) = pair else {
        return;
    };

    // --- 4. Per-frame gesture deltas (domain-owned semantics). ---
    let distance_delta = touch_state.agg.pinch_distance_delta();
    let angle_delta = touch_state.agg.pinch_angle_delta();
    let midpoint_delta = touch_state.agg.pinch_midpoint_delta();

    // --- 5. Adapter-boundary pixel→world scales (mirror controller_system). ---
    let win_h = windows
        .get_single()
        .map(|w| w.height() as f64)
        .unwrap_or(720.0)
        .max(1.0);
    // Finger-separation px → fraction of window height (right-drag zoom unit).
    let zoom_scale = 1.0 / win_h;

    let orbit_sens = sens_or(input_state.orbit_sensitivity);
    let zoom_sens = sens_or(input_state.zoom_sensitivity);
    let pan_sens = sens_or(input_state.pan_sensitivity);

    for mut cesium_cam in cameras.iter_mut() {
        // Extract config values before borrowing `camera` mutably.
        let enable_collision = cesium_cam.enable_collision_detection;
        let min_zoom_dist = cesium_cam.minimum_zoom_distance;
        let max_zoom_dist = cesium_cam.maximum_zoom_distance;
        let cam = &mut cesium_cam.camera;

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

        // Focal length in px: f = (H/2) / tan(fov/2) (grab-the-globe scale).
        let fov = match &cam.frustum {
            Frustum::Perspective(f) => f.fov,
            Frustum::Orthographic(_) => std::f64::consts::FRAC_PI_3,
        };
        let focal = ((win_h * 0.5) / (fov * 0.5).tan()).max(1.0);
        // Midpoint px → pan delta (middle-drag pan unit).
        let translate_scale = 1.0 / focal;

        ctrl.pinch_zoom(cam, distance_delta, zoom_scale);
        ctrl.pinch_rotate(cam, angle_delta);
        ctrl.pinch_translate(cam, midpoint_delta, translate_scale);
        ctrl.enforce_collision(cam);
    }
}

/// Converts the two tracked fingers (screen px, id-ordered) to f64 `DVec2`s,
/// or `None` when fewer than two fingers are down.
fn two_fingers(fingers: &[(u64, Vec2)]) -> Option<(DVec2, DVec2)> {
    if fingers.len() < 2 {
        return None;
    }
    let a = fingers[0].1;
    let b = fingers[1].1;
    Some((
        DVec2::new(a.x as f64, a.y as f64),
        DVec2::new(b.x as f64, b.y as f64),
    ))
}

/// Sensitivity multiplier: falls back to `1.0` when the resource is
/// zero-initialized (mirrors `controller_system`'s `non_zero_or`).
#[inline]
fn sens_or(value: f32) -> f64 {
    if value != 0.0 {
        value as f64
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::camera_update_system;
    use cesium_camera::Camera;
    use cesium_scene_mode::SceneMode;

    /// WGS84 maximum radius (meters); the test camera sits one radius above it.
    const R: f64 = 6378137.0;

    /// Builds a headless app: `MinimalPlugins` (provides `Time`), the touch
    /// event + state, [`camera_touch_system`] in `Update` and the Transform
    /// writer in `PostUpdate`, one [`Window`], and one [`CesiumCamera`] entity
    /// positioned one radius above the equator looking at the center.
    fn touch_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_event::<TouchInput>()
            .init_resource::<CameraInputState>()
            .init_resource::<TouchCameraState>()
            .add_systems(Update, camera_touch_system)
            .add_systems(PostUpdate, camera_update_system);
        app.world_mut().spawn(Window::default());
        let cam = Camera::new(
            glam::DVec3::new(R * 2.0, 0.0, 0.0),
            glam::DVec3::new(-1.0, 0.0, 0.0),
            glam::DVec3::new(0.0, 0.0, 1.0),
        );
        let cam_e = app
            .world_mut()
            .spawn((
                CesiumCamera::new(cam, SceneMode::Scene3D),
                Transform::default(),
                Projection::default(),
            ))
            .id();
        // Prime the Transform from the domain camera (no touch this frame).
        app.update();
        (app, cam_e)
    }

    /// Sends one [`TouchInput`] event into the app's event queue.
    fn send_touch(app: &mut App, phase: TouchPhase, id: u64, x: f32, y: f32) {
        app.world_mut()
            .resource_mut::<Events<TouchInput>>()
            .send(TouchInput {
                phase,
                position: Vec2::new(x, y),
                window: Entity::PLACEHOLDER,
                force: None,
                id,
            });
    }

    fn cesium_position(app: &App, e: Entity) -> glam::DVec3 {
        app.world().get::<CesiumCamera>(e).unwrap().camera.position
    }

    fn transform_len(app: &App, e: Entity) -> f32 {
        app.world()
            .get::<Transform>(e)
            .unwrap()
            .translation
            .length()
    }

    /// A two-finger spread (separation 100 → 200 px) zooms the camera in: the
    /// Bevy `Transform` translation shrinks toward the ellipsoid center.
    #[test]
    fn two_finger_spread_zooms_camera_in() {
        let (mut app, e) = touch_app();
        let initial = transform_len(&app, e);
        // Frame 1: two fingers land at separation 100 px (pinch seeds, no move).
        send_touch(&mut app, TouchPhase::Started, 1, -50.0, 0.0);
        send_touch(&mut app, TouchPhase::Started, 2, 50.0, 0.0);
        app.update();
        // Frame 2: fingers spread to separation 200 px → distance_delta = +100.
        send_touch(&mut app, TouchPhase::Moved, 1, -100.0, 0.0);
        send_touch(&mut app, TouchPhase::Moved, 2, 100.0, 0.0);
        app.update();

        let after = transform_len(&app, e);
        assert!(after < initial, "pinch-spread must zoom in: {after} !< {initial}");
        // Boundary magnitude: Δseparation 100 px / 720 → Δheight ≈ 88 km.
        let dropped = R * 2.0 - cesium_position(&app, e).length();
        assert!(
            (50_000.0..150_000.0).contains(&dropped),
            "zoom magnitude out of range: {dropped} m"
        );
    }

    /// A two-finger pinch-close (separation 200 → 100 px) zooms the camera out.
    #[test]
    fn two_finger_close_zooms_camera_out() {
        let (mut app, e) = touch_app();
        send_touch(&mut app, TouchPhase::Started, 1, -100.0, 0.0);
        send_touch(&mut app, TouchPhase::Started, 2, 100.0, 0.0);
        app.update();
        let initial = transform_len(&app, e);
        send_touch(&mut app, TouchPhase::Moved, 1, -50.0, 0.0);
        send_touch(&mut app, TouchPhase::Moved, 2, 50.0, 0.0);
        app.update();

        let after = transform_len(&app, e);
        assert!(after > initial, "pinch-close must zoom out: {after} !> {initial}");
    }

    /// A two-finger rotation (finger line 0 → 90° CCW, separation and midpoint
    /// fixed) spins the camera about the center: distance preserved, position
    /// moved — proving `pinch_angle_delta` reaches `pinch_rotate` → `spin`.
    #[test]
    fn two_finger_rotate_spins_camera_preserving_distance() {
        let (mut app, e) = touch_app();
        let pos0 = cesium_position(&app, e);
        let len0 = pos0.length();
        send_touch(&mut app, TouchPhase::Started, 1, -50.0, 0.0);
        send_touch(&mut app, TouchPhase::Started, 2, 50.0, 0.0);
        app.update();
        // Rotate the finger line 90° CCW; separation (100) and midpoint (0,0)
        // are unchanged, so this is a pure spin.
        send_touch(&mut app, TouchPhase::Moved, 1, 0.0, -50.0);
        send_touch(&mut app, TouchPhase::Moved, 2, 0.0, 50.0);
        app.update();

        let pos1 = cesium_position(&app, e);
        assert!(
            (pos1.length() - len0).abs() / len0 < 1e-9,
            "spin must preserve distance: {} vs {len0}",
            pos1.length()
        );
        assert!((pos1 - pos0).length() > 1.0, "spin must move the camera");
    }

    /// A two-finger drag (both fingers drift right 60 px, separation and angle
    /// fixed) translates the camera tangentially — proving `pinch_midpoint_delta`
    /// reaches `pinch_translate` → `pan` with the focal-length boundary scale.
    #[test]
    fn two_finger_drag_translates_camera_tangentially() {
        let (mut app, e) = touch_app();
        let pos0 = cesium_position(&app, e);
        send_touch(&mut app, TouchPhase::Started, 1, -50.0, 0.0);
        send_touch(&mut app, TouchPhase::Started, 2, 50.0, 0.0);
        app.update();
        // Midpoint (0,0) → (60,0); separation (100) and angle (0) unchanged.
        send_touch(&mut app, TouchPhase::Moved, 1, 10.0, 0.0);
        send_touch(&mut app, TouchPhase::Moved, 2, 110.0, 0.0);
        app.update();

        let pos1 = cesium_position(&app, e);
        let moved = pos1 - pos0;
        assert!(moved.length() > 1.0, "drag must translate the camera");
        // Pan runs along the view plane → mostly perpendicular to the radius.
        let radial = pos0.normalize();
        let perp = moved - radial * moved.dot(radial);
        assert!(
            perp.length() > 0.5 * moved.length(),
            "translate must be tangential: perp {} of {}",
            perp.length(),
            moved.length()
        );
    }

    /// A combined gesture (spread + rotate + drift at once) produces all three
    /// effects: zoom-in plus a tangential (spin/translate) component.
    #[test]
    fn combined_gesture_zooms_spins_and_translates() {
        let (mut app, e) = touch_app();
        let pos0 = cesium_position(&app, e);
        let len0 = pos0.length();
        send_touch(&mut app, TouchPhase::Started, 1, -50.0, 0.0);
        send_touch(&mut app, TouchPhase::Started, 2, 50.0, 0.0);
        app.update();
        send_touch(&mut app, TouchPhase::Moved, 1, -30.0, 40.0);
        send_touch(&mut app, TouchPhase::Moved, 2, 110.0, 20.0);
        app.update();

        let pos1 = cesium_position(&app, e);
        assert!(pos1.length() < len0, "combined gesture must zoom in");
        let radial = pos1.normalize();
        let tangential = pos1 - pos0;
        let perp = tangential - radial * tangential.dot(radial);
        assert!(perp.length() > 1.0, "combined gesture must spin/translate");
    }

    /// A multi-frame spread zooms in monotonically (per-frame re-seeding works
    /// across many frames, mirroring the domain sequence test).
    #[test]
    fn multi_frame_spread_zooms_in_monotonically() {
        let (mut app, e) = touch_app();
        send_touch(&mut app, TouchPhase::Started, 1, -20.0, 0.0);
        send_touch(&mut app, TouchPhase::Started, 2, 20.0, 0.0);
        app.update();
        let mut prev_len = cesium_position(&app, e).length();
        for half in [40.0_f32, 70.0, 110.0, 160.0] {
            send_touch(&mut app, TouchPhase::Moved, 1, -half, 0.0);
            send_touch(&mut app, TouchPhase::Moved, 2, half, 0.0);
            app.update();
            let len = cesium_position(&app, e).length();
            assert!(len < prev_len, "spread to {half} must zoom in: {len} !< {prev_len}");
            prev_len = len;
        }
    }

    /// Lifting one finger ends the pinch; a subsequent single-finger move must
    /// not zoom (the two-finger scope is released cleanly).
    #[test]
    fn releasing_a_finger_ends_the_pinch() {
        let (mut app, e) = touch_app();
        send_touch(&mut app, TouchPhase::Started, 1, -50.0, 0.0);
        send_touch(&mut app, TouchPhase::Started, 2, 50.0, 0.0);
        app.update();
        send_touch(&mut app, TouchPhase::Moved, 1, -100.0, 0.0);
        send_touch(&mut app, TouchPhase::Moved, 2, 100.0, 0.0);
        app.update();
        let after_zoom = cesium_position(&app, e).length();

        // Lift finger 2 → pinch ends.
        send_touch(&mut app, TouchPhase::Ended, 2, 100.0, 0.0);
        app.update();
        // A lone finger 1 move must be ignored by the two-finger path.
        send_touch(&mut app, TouchPhase::Moved, 1, -300.0, 0.0);
        app.update();

        let after_release = cesium_position(&app, e).length();
        assert!(
            (after_release - after_zoom).abs() < 1e-6,
            "after release the pinch must stop: {after_zoom} → {after_release}"
        );
        assert!(
            !app.world().resource::<TouchCameraState>().agg.is_pinching(),
            "aggregator must report not-pinching after release"
        );
    }

    /// With zero touch events the system is inert: the camera is byte-for-byte
    /// unchanged, proving the keyboard/mouse default path is not perturbed.
    #[test]
    fn no_touch_leaves_camera_unchanged() {
        let (mut app, e) = touch_app();
        let pos0 = cesium_position(&app, e);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(pos0, cesium_position(&app, e), "touch system must be inert");
    }
}
