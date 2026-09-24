//! Orbit camera controller — mouse drag to rotate, scroll to zoom.
//!
//! Mimics CesiumJS default ScreenSpaceCameraController behavior:
//! left-drag orbits around the globe, wheel zooms in/out.
//!
//! The globe is in ECEF orientation (north pole at +Z, equator in the XY
//! plane), so the camera orbits around the Z (polar) axis with Z as "up".
//!
//! ## M2.4 thin-shell delegation
//!
//! Rotation inertia and flight interpolation are delegated to the domain layer
//! (`cesium_interaction::{InertiaController, InertiaSample, decay, CameraFlight,
//! compute_flight_duration, select_flight_easing}`). The grab-the-globe tracking
//! formulas and zoom inertia glide are preserved byte-for-byte from the proven
//! M0 implementation.

use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use glam::{DQuat, DVec2, DVec3};
use cesium_interaction::{
    CameraFlight, InertiaController, InertiaSample, InertiaState,
    INERTIA_MAX_CLICK_TIME_THRESHOLD, compute_flight_duration, select_flight_easing,
};

use crate::feature_flags::{postprocess_builtin_enabled, postprocess_enabled};

/// Camera vertical field of view (radians). Kept in sync between the spawned
/// projection and the drag math so the grab-the-globe tracking is exact.
pub const CAMERA_FOV_Y: f32 = std::f32::consts::FRAC_PI_3; // 60 degrees
/// Near clip plane — kept below the camera's closest `min_distance` altitude
/// (≈760 m) so the ground stays visible when fully zoomed in to inspect the
/// finest tiles. Reversed-Z (wgpu default) tolerates the resulting near:far
/// ratio without surface z-fighting (same-level tiles never overlap).
const CAMERA_NEAR: f32 = 0.00005;
/// Far clip plane — large enough for the starfield (radius ~50).
const CAMERA_FAR: f32 = 200.0;
/// Globe (equatorial) radius in render units.
const GLOBE_RADIUS: f32 = 1.0;

/// WGS84 ellipsoid semi-axes in render units. The tile renderer tessellates the
/// ground on the WGS84 ellipsoid — equatorial `ELLIPSOID_A`, polar
/// `ELLIPSOID_B`, polar axis = local +Z (see `tile_mesh::create_tile_mesh_uv`,
/// whose vertex `z` carries the `(1 - e²)` factor). The grab-the-globe drag pick
/// MUST intersect this SAME surface: at deep zoom the camera sits only ~1e-3
/// render units above the ground, so the ~0.2% radial gap between a unit sphere
/// and the ellipsoid (largest at mid/high latitude) otherwise inflates into a
/// large screen-space gain error — the measured ~0.58× "不跟手" undershoot.
const ELLIPSOID_A: f64 = 1.0; // = GLOBE_RADIUS (EARTH_RADIUS / METERS_PER_RENDER_UNIT)
const ELLIPSOID_B: f64 = 6356752.314245 / 6378137.0; // ≈ 0.99664719 (polar / equatorial)

/// Absolute geocentric-angle cap (radians) per drag frame — a zoom-blind
/// backstop only. A pure radian cap is far too tight at deep zoom: 0.1 rad at
/// `min_distance` corresponds to ≲ 20 px of screen pan, so a ordinary fast
/// drag bound against it and the map lagged the cursor by 10-20 px every
/// frame (the measured "不跟手"). The real comfort limit is expressed in
/// pixels per frame (`MAX_PAN_PX_PER_FRAME`) and converted to a zoom-aware
/// angle; this radian value only keeps the two caps ordered (px cap ≤ rad
/// cap at the closest zoom) and guards the raw-motion path from a runaway.
const MAX_DRAG_STEP_RAD: f32 = 0.5;

/// Screen-pan cap (pixels / frame) for EVERY drag step — both the anchored
/// grab-the-globe correction and the raw-motion fallback, in-window or not.
/// A radian cap is zoom-blind — 0.1 rad at max zoom is only ~20 px of pan,
/// which throttled fast drags at the surface and made the map trail the
/// cursor — so each frame's rotation is additionally bounded to never pan
/// more than this many pixels (then converted to a zoom-aware angle via
/// `px · surface_dist / focal`). Comfortably above any real one-frame mouse
/// motion (a very fast flick ≈ 150 px/frame), so normal drags stay exact
/// 1:1 at every zoom; only pathological spikes (coalesced input bursts,
/// near-pole grabs where meridians converge) get reeled in over a couple of
/// frames instead of teleporting the view or flinging the camera past the
/// globe.
const MAX_PAN_PX_PER_FRAME: f32 = 600.0;

/// Default inertia decay coefficient for rotation coasting (CesiumJS
/// `inertiaSpin` default ≈ 0.9).
const INERTIA_SPIN_COEFFICIENT: f64 = 0.9;

/// Marker component for the orbit-controlled camera.
#[derive(Component)]
pub struct OrbitCamera;

/// Resource holding the orbit state (spherical coordinates around target).
#[derive(Resource)]
pub struct OrbitState {
    /// Azimuth angle in radians (rotation around the globe's Z/polar axis).
    pub heading: f32,
    /// Elevation angle in radians above the equatorial (XY) plane.
    /// Positive = north of the equator, negative = south.
    pub pitch: f32,
    /// Distance from target in render units.
    pub distance: f32,
    /// Wheel input moves this instantly; `distance` glides toward it each
    /// frame (exponential easing), giving CesiumJS-style zoom inertia
    /// instead of a hard 30% jump per wheel notch.
    pub target_distance: f32,
    /// Orbit target (world space, globe center).
    pub target: Vec3,
    /// Overall rotation sensitivity multiplier (1.0 = exact 1:1 surface
    /// tracking derived from the camera geometry).
    pub rotate_speed: f32,
    /// Zoom sensitivity: fractional change in height-above-surface per wheel
    /// unit (0.3 = each notch moves 30% closer/farther from the surface).
    pub zoom_speed: f32,
    /// Min zoom distance (just above the surface so you can inspect detail).
    pub min_distance: f32,
    /// Max zoom distance.
    pub max_distance: f32,
}

impl Default for OrbitState {
    fn default() -> Self {
        Self {
            heading: 0.0,
            pitch: 0.4, // ~23 deg north of the equator
            distance: 3.0,
            target_distance: 3.0,
            target: Vec3::ZERO,
            rotate_speed: 1.0, // exact geometric tracking by default
            zoom_speed: 0.3,
            min_distance: 1.00012, // descend to ~760 m altitude -> level ~17 tiles
            max_distance: 20.0,
        }
    }
}

// ── M2.4 Rotation Inertia State ─────────────────────────────────────────────

/// Resource tracking rotation inertia for the orbit camera.
///
/// When a left-drag is released after a quick flick (< [`INERTIA_MAX_CLICK_TIME_THRESHOLD`]
/// seconds), the last frame's heading/pitch velocity is captured into the
/// domain [`InertiaController`] and coasted with exponential decay each frame.
#[derive(Resource)]
pub struct OrbitInertiaState {
    /// Domain inertia controller (pure f64 math, no Bevy dependency).
    pub controller: InertiaController,
    /// Whether the left button was down on the previous frame.
    was_dragging: bool,
    /// Previous frame's heading for delta computation.
    prev_heading: f32,
    /// Previous frame's pitch for delta computation.
    prev_pitch: f32,
    /// Elapsed time in milliseconds (monotonic clock for inertia timing).
    now_ms: f64,
    /// Timestamp (ms) when the current drag started.
    press_time_ms: f64,
    /// Timestamp (ms) when the current drag was released.
    release_time_ms: f64,
    /// Whether inertia coasting is active.
    coasting: bool,
    /// Pixels-per-radian heading scale captured at release time. The domain
    /// [`InertiaController`] coasts in pixel space, so the coasted pixel delta
    /// is converted back to radians with the *same* scale used on capture,
    /// giving an exact exponential decay of the original radian velocity.
    capture_scale_h: f32,
    /// Pixels-per-radian pitch scale captured at release time (see above).
    capture_scale_p: f32,
}

impl Default for OrbitInertiaState {
    fn default() -> Self {
        Self {
            controller: InertiaController::new(),
            was_dragging: false,
            prev_heading: 0.0,
            prev_pitch: 0.4,
            now_ms: 0.0,
            press_time_ms: 0.0,
            release_time_ms: 0.0,
            coasting: false,
            capture_scale_h: 1.0,
            capture_scale_p: 1.0,
        }
    }
}

// ── M2.4 Flight State ───────────────────────────────────────────────────────

/// Resource holding an active great-arc camera flight for the orbit camera.
///
/// When a flight is active, the orbit state is driven by the domain
/// [`CameraFlight`] slerp interpolation instead of mouse input.
#[derive(Resource, Default)]
pub struct OrbitFlightState {
    /// The active flight, if any.
    pub flight: Option<CameraFlight>,
}

/// Event requesting the orbit camera to fly to an ECEF destination (meters).
///
/// Send this event to trigger a great-arc flight with automatic duration and
/// easing derived from the domain's [`compute_flight_duration`] and
/// [`select_flight_easing`].
#[derive(Event)]
pub struct OrbitFlyToRequest {
    /// Target position in ECEF meters.
    pub destination_ecef: DVec3,
}

// ── FIX-ARCBALL: live trackball orientation ─────────────────────────────────

/// Live arcball (trackball) orientation for the interactive camera.
///
/// `engaged` flips to `true` the first time the user drags the left mouse
/// button in a windowed session and then stays `true` so the pose — including
/// roll and over-the-pole views — persists between frames. Every deterministic
/// capture path (`FIXED_CAMERA`, `--camera-script`, the M2.4 neutrality test,
/// the v0 baselines) never feeds a mouse, so `engaged` stays `false` and
/// [`orbit_camera_system`] keeps driving the camera through the pure
/// [`compute_camera_transform`] spherical path → byte-for-byte unchanged.
#[derive(Resource)]
struct Arcball {
    /// Rotation of the camera rig about the target (globe centre). Held in
    /// f64: at max zoom the per-frame grab-the-globe correction is ~1e-7 rad,
    /// which f32 annihilates (catastrophic cancellation between two O(1) unit
    /// vectors that differ below ε → a headless probe showed the whole drag
    /// sweeping 0.0 rad), freezing the drag. f64 keeps that tiny residual
    /// meaningful; it is narrowed to f32 only for the render pose.
    orientation: DQuat,
    /// Whether a real drag has taken over from the spherical path.
    engaged: bool,
    /// Geographic anchor: unit vector (target → surface) of the point grabbed
    /// under the cursor on the current drag. Each frame the rig is rotated so
    /// this point stays glued to the cursor → exact pointer tracking at any
    /// grab location / zoom. `None` until a point is picked under the cursor.
    anchor: Option<DVec3>,
    /// Left-button state on the previous frame, to detect a fresh press.
    was_pressed: bool,
}

impl Default for Arcball {
    fn default() -> Self {
        Self {
            orientation: DQuat::IDENTITY,
            engaged: false,
            anchor: None,
            was_pressed: false,
        }
    }
}

// ── M0.1 Camera Seed from Environment ─────────────────────────────────────
// This is the minimal precursor to M3.3 FIXED_CAMERA; M3.3 will formalize
// the interface with a proper config struct and validation. For now we read
// individual env vars so the capture harness can position the camera without
// touching main.rs or any rendering logic.
//
// Supported env vars (all optional; unset = pixel-neutral default):
//   CESIUM_CAM_LON      — longitude in degrees (camera position azimuth)
//   CESIUM_CAM_LAT      — latitude in degrees (camera position elevation)
//   CESIUM_CAM_HEIGHT   — height above surface in render units (default globe R=1)
//   CESIUM_CAM_HEADING  — alias for LON (takes precedence if both set)
//   CESIUM_CAM_PITCH    — alias for LAT (takes precedence if both set)
//   CESIUM_CAM_DISTANCE — direct orbit distance from center (overrides HEIGHT)
//
// When NONE of these are set the returned state is `OrbitState::default()`,
// guaranteeing binary-identical output to the unmodified codebase.

/// Read camera seed from env vars. Returns `OrbitState::default()` when no
/// seed vars are present (pixel-neutral path).
pub(crate) fn orbit_state_from_env() -> OrbitState {
    let mut state = OrbitState::default();
    let mut any_set = false;

    // Helper: parse f32 from env var
    let read_f32 = |name: &str| -> Option<f32> {
        std::env::var(name).ok().and_then(|v| v.trim().parse::<f32>().ok())
    };

    // Heading: CESIUM_CAM_HEADING takes precedence over CESIUM_CAM_LON
    if let Some(h) = read_f32("CESIUM_CAM_HEADING").or(read_f32("CESIUM_CAM_LON")) {
        state.heading = h.to_radians();
        any_set = true;
    }

    // Pitch: CESIUM_CAM_PITCH takes precedence over CESIUM_CAM_LAT
    if let Some(p) = read_f32("CESIUM_CAM_PITCH").or(read_f32("CESIUM_CAM_LAT")) {
        state.pitch = p.to_radians();
        any_set = true;
    }

    // Distance: CESIUM_CAM_DISTANCE overrides HEIGHT
    if let Some(d) = read_f32("CESIUM_CAM_DISTANCE") {
        state.distance = d;
        state.target_distance = d;
        any_set = true;
    } else if let Some(h) = read_f32("CESIUM_CAM_HEIGHT") {
        let d = GLOBE_RADIUS + h;
        state.distance = d;
        state.target_distance = d;
        any_set = true;
    }

    if any_set {
        info!(
            "[camera-seed] env override: heading={:.4} pitch={:.4} distance={:.4}",
            state.heading, state.pitch, state.distance
        );
    }
    state
}

/// Plugin that sets up the orbit camera.
pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        // M0.1: seed initial camera from env (pixel-neutral when unset)
        let initial_state = orbit_state_from_env();
        app.insert_resource(initial_state)
            .init_resource::<OrbitInertiaState>()
            .init_resource::<OrbitFlightState>()
            .init_resource::<Arcball>()
            .add_event::<OrbitFlyToRequest>()
            .add_systems(Startup, spawn_orbit_camera)
            .add_systems(
                Update,
                (orbit_camera_system, orbit_inertia_system, orbit_flight_system).chain(),
            );
    }
}

fn spawn_orbit_camera(mut commands: Commands, state: Res<OrbitState>) {
    let transform = compute_camera_transform(&state);
    // Custom perspective projection: a small near plane lets the camera get
    // very close to the surface for inspecting imagery detail, while the far
    // plane still reaches the starfield.
    let projection = PerspectiveProjection {
        fov: CAMERA_FOV_Y,
        near: CAMERA_NEAR,
        far: CAMERA_FAR,
        ..default()
    };

    // M4.2: when the built-in post-process gate is ON, enable HDR rendering
    // with ACES Fitted tonemapping + natural bloom. The HDR pipeline computes
    // lighting in linear space, tonemaps to LDR, then sRGB-encodes for display.
    // When OFF (default), Tonemapping::None preserves the v0 baseline exactly
    // (CesiumJS displays imagery as-is; TonyMcMapFace requires the
    // `tonemapping_luts` feature which is disabled in this workspace).
    //
    // M5-E1: FXAA lives on a separate gate (CESIUM_ENABLE_POSTPROCESS). When ON,
    // the camera is tagged with `CesiumFxaa` so the render-graph FXAA node runs
    // after tonemapping. The two gates are independent: FXAA can be enabled with
    // or without HDR/tonemapping (it operates on whatever LDR image precedes it).
    let mut cam = if postprocess_builtin_enabled() {
        commands.spawn((
            Camera3d::default(),
            Camera {
                hdr: true,
                ..default()
            },
            Tonemapping::AcesFitted,
            Bloom::NATURAL,
            OrbitCamera,
            Projection::Perspective(projection),
            transform,
        ))
    } else {
        commands.spawn((
            Camera3d::default(),
            Tonemapping::None,
            OrbitCamera,
            Projection::Perspective(projection),
            transform,
        ))
    };

    // M5-E1: attach the FXAA trigger component when the post-process gate is ON.
    // `fxaa_system` (adapters/bevy-render effects/post_process.rs) keeps
    // `.enabled` synced with `PostProcessConfig.fxaa_enabled` each frame; the
    // marker is extracted to the render world and read by `FxaaNode::run`.
    if postprocess_enabled() {
        cam.insert(cesium_bevy_render::effects::CesiumFxaa { enabled: true });
    }
}

/// System: read mouse input and update camera transform.
///
/// This is the **original M0 system** — grab-the-globe formulas and zoom
/// inertia glide are preserved byte-for-byte. Rotation inertia coasting is
/// handled by [`orbit_inertia_system`] which runs after this.
#[allow(clippy::too_many_arguments)] // Bevy system: one param per resource/event/query
fn orbit_camera_system(
    mut state: ResMut<OrbitState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion_events: EventReader<MouseMotion>,
    mut wheel_events: EventReader<MouseWheel>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<OrbitCamera>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    mut arcball: ResMut<Arcball>,
) {
    // Rotation: left mouse drag — cursor-anchored "grab the globe". On press
    // we pick the geographic point under the pointer (ray → sphere), then each
    // frame rotate the rig about the target so that same point stays glued to
    // the cursor as it moves. This tracks the pointer EXACTLY at any grab
    // location and zoom — the previous per-pixel tangent gain only matched the
    // single screen-centre point, so grabbing elsewhere felt detached in both
    // axes. A geometric fallback runs when the OS cursor position is
    // unavailable, so the feel degrades gracefully instead of locking up.
    let pressed = mouse_buttons.pressed(MouseButton::Left);
    if pressed {
        // Fresh press (new grab): reset the anchor so we re-pick under the
        // cursor, and latch the spherical pose if this is the first ever drag.
        if !arcball.was_pressed {
            if !arcball.engaged {
                arcball.orientation = arcball_quat_from_spherical(&state).as_dquat();
                arcball.engaged = true;
            }
            arcball.anchor = None;
        }

        let win = windows.get_single().ok();
        let win_h = win.map(|w| w.height()).unwrap_or(720.0);
        let focal = (win_h * 0.5) / (CAMERA_FOV_Y * 0.5).tan();
        let surface_dist = (state.distance - GLOBE_RADIUS).max(0.001);

        // World-space pick ray through the cursor, then the surface direction
        // it hits. PREFER the camera's *real* projection (`viewport_to_world`,
        // i.e. Bevy's actual `clip_from_view` + rendered `GlobalTransform`) so
        // "the point under the cursor" is by construction the point the user
        // SEES under the cursor — this removes every hand-model assumption
        // (frustum math, transform conventions, up-axis). The hand-rolled
        // `cursor_ray_world` survives only as the fallback for frames where
        // the camera isn't queryable yet. Both widen to f64 downstream.
        let bevy_ray = win.and_then(|w| {
            cameras
                .get_single()
                .ok()
                .and_then(|(cam, ct)| cursor_ray_bevy(cam, ct, w))
        });
        let picked = bevy_ray
            .or_else(|| {
                win.and_then(|w| {
                    cursor_ray_world(w, arcball.orientation, state.distance, state.target)
                })
            })
            .and_then(|(o, d)| pick_surface_dir_ellipsoid(o, d, state.target.as_dvec3()));

        match (arcball.anchor, picked) {
            (Some(anchor), Some(bdir)) => {
                // Rotate the rig so the grabbed point comes back under the
                // cursor (exact 1:1). Deltas are irrelevant (absolute cursor).
                // NOTE: deliberately NOT `Quat::from_rotation_arc` — it
                // early-outs to `IDENTITY` when `dot > 1 − ε`, and at max zoom
                // the anchor and cursor surface directions collapse to within
                // ~3e-5 rad so their f32 dot rounds to 1.0 → the drag froze at
                // 0 rotation. `rotation_from_unit_dir` recovers that tiny angle.
                //
                // The correction is then CLAMPED per frame — but the cap is
                // ZOOM-AWARE: `MAX_PAN_PX_PER_FRAME` pixels of screen pan
                // converted to a geocentric angle at the current altitude
                // (`px · surface_dist / focal`), never exceeding the global
                // radian backstop. A fixed radian cap looked safe at whole-
                // globe zoom yet throttled deep zoom into the ground: 0.1 rad
                // at `min_distance` ≈ 20 px/frame, so every fast drag left a
                // 10-20 px lag (the "不跟手"). Normal
                // drags never bind the pixel cap at any zoom (mid-globe chord
                // ≲ 0.03 rad, max-zoom steps ≲ 1e-4 → still exact 1:1), while
                // a large single-frame cursor delta (a flick / coalesced input
                // burst) or a near-pole grab — where meridians converge and a
                // horizontal mouse move maps to a huge geocentric swing — is
                // reeled in over a few frames instead of teleporting the view
                // or flinging the camera past the globe ("地球没了").
                let step_cap =
                    (f64::from(MAX_PAN_PX_PER_FRAME) * f64::from(surface_dist / focal))
                        .min(f64::from(MAX_DRAG_STEP_RAD));
                let r = clamp_rotation_angle(rotation_from_unit_dir(bdir, anchor), step_cap as f32);
                arcball.orientation = (r * arcball.orientation).normalize();
                motion_events.clear();
            }
            (maybe_anchor, Some(bdir)) => {
                // First picked frame (or cursor re-entered the globe): latch the
                // anchor to the point under the cursor and DO NOT rotate this
                // frame. Applying the geometric gain here too would fight the
                // next frame's pick correction (it rotates the just-latched
                // point back under the cursor), producing a start-of-drag
                // teleport. Tracking begins cleanly from the following frame.
                let _ = maybe_anchor;
                arcball.anchor = Some(bdir);
                motion_events.clear();
            }
            (_, None) => {
                // No surface point under the cursor. Two very different cases
                // must NOT be treated the same way:
                // Whether the pointer is still inside the window decides only
                // how HARD we bound the step, not whether we rotate. The
                // absolute-anchor path only ever consumed motion on the frames
                // it took the (None) branch, so a stray backlog could flush on
                // the first cursor-less frame and fling the globe (the old
                // "鼠标一出窗口就飞速旋转"). Freezing outright was the wrong
                // cure: at max zoom the cursor reaches the window edge almost
                // immediately, so a freeze read as "the map won't follow".
                // Instead KEEP dragging CesiumJS-style from the raw per-frame
                // mouse motion at the same 1:1 surface gain the anchored path
                // uses, but bound each frame's pan.
                arcball.anchor = None;
                let lat_factor = state.pitch.cos().max(0.15);
                // Convert the pixel cap to a zoom-aware angle (a cap expressed
                // in radians is meaningless at the surface: 0.1 rad at max zoom
                // is ~100k px of pan) and never exceed the global radian cap.
                // Same zoom-aware cap whether or not the pointer is in the
                // window: the pixel bound already tracks the current altitude,
                // and the radian backstop keeps a near-pole swing sane.
                let step_cap =
                    (f64::from(MAX_PAN_PX_PER_FRAME) * f64::from(surface_dist / focal))
                        .min(f64::from(MAX_DRAG_STEP_RAD)) as f32;
                for ev in motion_events.read() {
                    let up = (arcball.orientation * DVec3::Y).normalize();
                    let right = (arcball.orientation * DVec3::X).normalize();
                    let d_yaw = -ev.delta.x
                        * state.rotate_speed
                        * surface_dist
                        / (lat_factor * focal);
                    let d_tilt = ev.delta.y * state.rotate_speed * surface_dist / focal;
                    let q = DQuat::from_axis_angle(up, f64::from(d_yaw))
                        * DQuat::from_axis_angle(right, f64::from(d_tilt));
                    arcball.orientation =
                        (clamp_rotation_angle(q, step_cap) * arcball.orientation).normalize();
                }
            }
        }

        // Re-derive heading/pitch (roll intentionally dropped) so the spherical
        // consumers — globe LOD sub-camera point, inertia capture — stay live.
        let (h, p) = orbit_from_orientation(arcball.orientation);
        state.heading = h;
        state.pitch = p;
    } else {
        // Consume events even when not dragging to avoid accumulation
        motion_events.clear();
    }
    arcball.was_pressed = pressed;

    // Zoom: mouse wheel — scale the height ABOVE THE SURFACE multiplicatively,
    // not the distance from the center. Near the ground, distance-from-center
    // is ~= R, so a fixed ratio of it is a huge ratio of the small height
    // above the surface (one notch would slam into the ground), while pulling
    // back out feels sluggish. Scaling the height-above-surface instead gives
    // a consistent perceived zoom at any altitude: gentle when skimming the
    // ground, fast when approaching from afar.
    for ev in wheel_events.read() {
        let min_surf = state.min_distance - GLOBE_RADIUS;
        let max_surf = state.max_distance - GLOBE_RADIUS;
        let surface_dist = (state.target_distance - GLOBE_RADIUS).clamp(min_surf, max_surf);
        // ev.y > 0 (scroll up) = zoom in -> shrink the height above the surface.
        let zoom_factor = 1.0 - ev.y * state.zoom_speed;
        let new_surf = (surface_dist * zoom_factor).clamp(min_surf, max_surf);
        state.target_distance = GLOBE_RADIUS + new_surf;
    }

    // Zoom inertia: glide `distance` toward the wheel-set target so the
    // scene scales continuously (CesiumJS eases zoom the same way; a hard
    // per-notch jump reads as tile "wobble").
    let k = 1.0 - (-10.0f32 * time.delta_secs()).exp();
    state.distance += (state.target_distance - state.distance) * k;
    if (state.target_distance - state.distance).abs() < 1.0e-5 {
        state.distance = state.target_distance;
    }

    // Apply transform: trackball pose while engaged, otherwise the pure
    // spherical north-up path (byte-identical to the pre-arcball baseline).
    if let Ok(mut transform) = query.get_single_mut() {
        *transform = if arcball.engaged {
            transform_from_arcball(arcball.orientation.as_quat(), state.distance, state.target)
        } else {
            compute_camera_transform(&state)
        };
    }
}

/// M2.4: Rotation inertia coasting system (delegated to domain
/// [`InertiaController`]).
///
/// Runs AFTER [`orbit_camera_system`]. Tracks heading/pitch deltas between
/// frames; on a quick flick release, captures the velocity and coasts with
/// exponential decay. Suppressed while a flight is active.
#[allow(clippy::too_many_arguments)] // Bevy system: one param per resource/event/query
fn orbit_inertia_system(
    mut state: ResMut<OrbitState>,
    mut inertia: ResMut<OrbitInertiaState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    flight_state: Res<OrbitFlightState>,
    mut query: Query<&mut Transform, With<OrbitCamera>>,
    windows: Query<&Window>,
    mut arcball: ResMut<Arcball>,
) {
    // Advance the monotonic clock for inertia timing.
    inertia.now_ms += time.delta_secs() as f64 * 1000.0;

    let is_dragging = mouse_buttons.pressed(MouseButton::Left);
    let flight_active = flight_state.flight.is_some();

    // Window height for the radian↔pixel boundary conversion (the domain
    // InertiaController coasts in pixel space, see `inertia_pixel_scale`).
    let win_h = windows
        .get_single()
        .map(|w| w.height())
        .unwrap_or(720.0);

    // ── Detect drag start ───────────────────────────────────────────────
    if is_dragging && !inertia.was_dragging {
        inertia.coasting = false;
        inertia.controller.deactivate(InertiaState::Spin);
        inertia.press_time_ms = inertia.now_ms;
    }

    // ── Detect drag release → capture inertia (radian → pixel) ──────────
    if inertia.was_dragging && !is_dragging && !flight_active {
        inertia.release_time_ms = inertia.now_ms;
        let hold_secs = (inertia.release_time_ms - inertia.press_time_ms) / 1000.0;
        let heading_delta = (state.heading - inertia.prev_heading) as f64;
        let pitch_delta = (state.pitch - inertia.prev_pitch) as f64;

        if hold_secs < INERTIA_MAX_CLICK_TIME_THRESHOLD
            && (heading_delta.abs() > 1e-8 || pitch_delta.abs() > 1e-8)
        {
            // Convert the radian velocity into pixel space: the domain coasts
            // in pixels and its `INERTIA_STOP_DISTANCE` guard (0.5 px) is
            // meaningless on raw radians (which are ~0.01). The scale mirrors
            // the grab-the-globe projection so the round-trip is exact.
            let (scale_h, scale_p) = inertia_pixel_scale(&state, win_h);
            inertia.capture_scale_h = scale_h;
            inertia.capture_scale_p = scale_p;
            // capture stores motion = (end - start) * 0.5, so pass end = 2×delta.
            // `heading` is re-derived via atan2 while trackball-engaged and can
            // wrap ±π across a pole crossing; normalise the per-frame delta so a
            // wrap doesn't masquerade as a huge velocity (→ runaway coast).
            let motion_px = DVec2::new(
                wrap_pi(heading_delta as f32) as f64 * scale_h as f64,
                pitch_delta * scale_p as f64,
            );
            inertia
                .controller
                .capture(InertiaState::Spin, DVec2::ZERO, motion_px * 2.0);
            inertia.controller.activate(Some(InertiaState::Spin));
            inertia.coasting = true;
        } else {
            inertia.coasting = false;
        }
    }

    inertia.was_dragging = is_dragging;

    // ── Coast with exponential decay (pixel → radian) ───────────────────
    if inertia.coasting && !is_dragging && !flight_active {
        let sample = InertiaSample::new(
            INERTIA_SPIN_COEFFICIENT,
            inertia.press_time_ms,
            inertia.release_time_ms,
            inertia.now_ms,
        );
        // Snapshot the capture-time scale before borrowing `inertia` mutably.
        let scale_h = inertia.capture_scale_h as f64;
        let scale_p = inertia.capture_scale_p as f64;
        match inertia.controller.maintain(InertiaState::Spin, &sample) {
            Some(delta_px) => {
                let d_heading = (delta_px.x / scale_h) as f32;
                let d_pitch = (delta_px.y / scale_p) as f32;
                if arcball.engaged {
                    // Coast the live trackball the same way the drag drove it:
                    // yaw about the camera up, tilt about the camera right.
                    let up = (arcball.orientation * DVec3::Y).normalize();
                    let right = (arcball.orientation * DVec3::X).normalize();
                    let q = DQuat::from_axis_angle(up, f64::from(d_heading))
                        * DQuat::from_axis_angle(right, f64::from(d_pitch));
                    arcball.orientation = (q * arcball.orientation).normalize();
                    let (h, p) = orbit_from_orientation(arcball.orientation);
                    state.heading = h;
                    state.pitch = p;
                    if let Ok(mut transform) = query.get_single_mut() {
                        *transform = transform_from_arcball(
                            arcball.orientation.as_quat(),
                            state.distance,
                            state.target,
                        );
                    }
                } else {
                    state.heading += d_heading;
                    state.pitch = (state.pitch + d_pitch).clamp(-1.5, 1.5);
                    if let Ok(mut transform) = query.get_single_mut() {
                        *transform = compute_camera_transform(&state);
                    }
                }
            }
            None => {
                inertia.coasting = false;
            }
        }
    }

    // Store current heading/pitch for next frame's delta computation.
    inertia.prev_heading = state.heading;
    inertia.prev_pitch = state.pitch;
}

/// Pixels-per-radian scale factors `(heading, pitch)` at the current orbit state.
///
/// The domain [`InertiaController`] coasts in **pixel space** — its
/// `INERTIA_STOP_DISTANCE` guard is 0.5 px — so the app boundary converts
/// rotation deltas from radians to pixels before capture and back to radians
/// after [`InertiaController::maintain`]. The factors are the exact inverse of
/// the geometric grab gain: `focal = (H/2)/tan(fov/2)`,
/// `surface_dist = distance - R` (the same inverse used by the drag
/// fallback). Using the same scale for capture and coast makes the
/// pixel round-trip lossless, so the coasted motion is a clean exponential
/// decay of the released radian velocity.
fn inertia_pixel_scale(state: &OrbitState, win_h: f32) -> (f32, f32) {
    let focal = (win_h * 0.5) / (CAMERA_FOV_Y * 0.5).tan();
    let surface_dist = (state.distance - GLOBE_RADIUS).max(0.001);
    let s = focal / surface_dist;
    (s, s)
}

/// Build the world-space pick ray (origin + unit direction) through the OS
/// cursor for the arcball camera rig. `None` when the cursor position is
/// unavailable (e.g. pointer outside the window). Uses the custom frustum
/// (`CAMERA_FOV_Y`) and the window aspect; the camera looks along its local
/// -Z, sitting at `target + orientation·(Ẑ · distance)`.
fn cursor_ray_world(
    window: &Window,
    orientation: DQuat,
    distance: f32,
    target: Vec3,
) -> Option<(DVec3, DVec3)> {
    let cursor = window.cursor_position()?;
    let w = window.width().max(1.0);
    let h = window.height().max(1.0);
    let aspect = w / h;
    let x_ndc = (cursor.x / w) * 2.0 - 1.0;
    let y_ndc = 1.0 - (cursor.y / h) * 2.0;
    let tan_y = (CAMERA_FOV_Y * 0.5).tan();
    let tan_x = tan_y * aspect;
    // Everything downstream (anchor pick + rotation) runs in f64 so the tiny
    // max-zoom residuals survive; the pixel/FOV inputs are widened exactly.
    let dir_cam = DVec3::new(
        f64::from(x_ndc * tan_x),
        f64::from(y_ndc * tan_y),
        -1.0,
    )
    .normalize();
    let origin = target.as_dvec3() + orientation * (DVec3::Z * f64::from(distance));
    let dir = (orientation * dir_cam).normalize();
    Some((origin, dir))
}

/// Build the world-space pick ray through the OS cursor using the camera's
/// REAL projection — `Camera::viewport_to_world` composes the actual
/// `clip_from_view` matrix and the rendered `GlobalTransform`, so the ray it
/// returns is exactly the line of sight the frame on screen was drawn along.
/// This is the non-circular replacement for [`cursor_ray_world`]: instead of
/// assuming our hand-derived frustum matches the renderer, we ask the renderer.
/// The near-plane `origin` + unit `direction` are widened to f64 immediately
/// so the downstream anchor rotation keeps the max-zoom tiny-angle precision
/// (the f32 ray direction carries ~1e-7 rad, well under the ~1e-5 rad signal).
fn cursor_ray_bevy(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
) -> Option<(DVec3, DVec3)> {
    let cursor = window.cursor_position()?;
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    let origin = ray.origin.as_dvec3();
    let dir = Vec3::from(ray.direction).as_dvec3();
    Some((origin, dir))
}

/// Intersect a world ray with the WGS84 ellipsoid (centred at `center`,
/// equatorial radius `ELLIPSOID_A` in x/y, polar radius `ELLIPSOID_B` in z —
/// the SAME surface the tile meshes are tessellated on) and return the unit
/// direction from the centre to the near hit. `None` on a miss or when the only
/// intersection is behind the camera. This replaces the old unit-sphere pick
/// for the grab-the-globe drag so the anchor is the point
/// the user actually SEES on the ground, not a sphere floating above it — the
/// radial gap between the two is what made deep-zoom drags undershoot.
fn pick_surface_dir_ellipsoid(origin: DVec3, dir: DVec3, center: DVec3) -> Option<DVec3> {
    // Anisotropically scale to unit-sphere space, solve |o + t·d|² = 1, scale
    // the near hit back to world. `dir` is unit in world but not after scaling,
    // so keep the general quadratic (a ≠ 1).
    let s = DVec3::new(1.0 / ELLIPSOID_A, 1.0 / ELLIPSOID_A, 1.0 / ELLIPSOID_B);
    let os = (origin - center) * s;
    let ds = dir * s;
    let a = ds.dot(ds);
    let half_b = os.dot(ds);
    let c = os.dot(os) - 1.0;
    let disc = half_b * half_b - a * c;
    if disc <= 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let t0 = (-half_b - sq) / a;
    let t = if t0 > 0.0 { t0 } else { (-half_b + sq) / a };
    if t <= 0.0 {
        return None;
    }
    let hit = center + (os + ds * t) / s;
    (hit - center).try_normalize()
}

/// Shortest-arc rotation taking unit direction `from` onto unit direction `to`,
/// computed as `axis = from × to`, `angle = atan2(|axis|, from·to)`.
///
/// This replaces `Quat::from_rotation_arc`, which is unusable for the grab-the-
/// globe anchor: it bails out to `IDENTITY` whenever `dot > 1 − ε`. At max zoom
/// (camera ~765 m above the surface) the two geocentric surface directions — the
/// latched anchor and the current cursor pick — differ by only ~3e-5 rad, so
/// their f32 dot rounds to 1.0 and the anchored rotation degenerated to the
/// identity, freezing the drag at 0 px. The cross-product magnitude (~3e-5) is
/// still orders of magnitude above the f32 subnormal floor, so `atan2` recovers
/// the true tiny angle and the grabbed point tracks the cursor 1:1 right down to
/// the surface — while the same absolute-anchor math stays exact (never
/// over-spins) at wide/whole-globe zooms, unlike a per-pixel gain that scales
/// with camera height.
fn rotation_from_unit_dir(from: DVec3, to: DVec3) -> DQuat {
    let axis = from.cross(to);
    let sin = axis.length();
    let cos = from.dot(to);
    if sin < 1.0e-12 {
        // (Anti)parallel: no meaningful rotation axis.
        return if cos < 0.0 {
            // 180° about any unit axis perpendicular to `from`.
            let perp = if from.x.abs() < from.y.abs() { DVec3::X } else { DVec3::Y };
            from.cross(perp)
                .try_normalize()
                .map(|a| DQuat::from_axis_angle(a, std::f64::consts::PI))
                .unwrap_or(DQuat::IDENTITY)
        } else {
            DQuat::IDENTITY
        };
    }
    DQuat::from_axis_angle(axis / sin, sin.atan2(cos))
}

/// Soft-cap a rotation's angle to at most `max_angle` radians, preserving its
/// axis. Used to bound the per-frame "grab the globe" correction: a huge
/// single-frame residual (fast flick, coalesced input burst, or a near-pole
/// grab where meridians converge) would otherwise teleport the view; clamping
/// lets it converge smoothly over a few frames instead. A near-identity or
/// already-small rotation is returned unchanged, so ordinary drags stay
/// byte-identical (exact 1:1) — only oversized lurches are softened.
fn clamp_rotation_angle(q: DQuat, max_angle: f32) -> DQuat {
    let (axis, angle) = q.to_axis_angle();
    let max_angle = f64::from(max_angle);
    if angle <= max_angle || axis.length_squared() < 1.0e-12 {
        return q;
    }
    DQuat::from_axis_angle(axis, max_angle)
}

/// Normalise an angle to the (-π, π] interval. Guards the inertia capture
/// against a ±π azimuth wrap when the trackball crosses a pole.
fn wrap_pi(a: f32) -> f32 {
    let two_pi = 2.0 * std::f32::consts::PI;
    let mut x = (a + std::f32::consts::PI) % two_pi;
    if x < 0.0 {
        x += two_pi;
    }
    x - std::f32::consts::PI
}

/// M2.4: Great-arc flight system (delegated to domain [`CameraFlight`]).
///
/// Runs LAST so the flight has final say over the orbit state. Uses the
/// domain's slerp great-arc interpolation with automatic duration/easing
/// from [`compute_flight_duration`] and [`select_flight_easing`].
fn orbit_flight_system(
    mut flight_state: ResMut<OrbitFlightState>,
    mut state: ResMut<OrbitState>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<OrbitCamera>>,
    mut fly_requests: EventReader<OrbitFlyToRequest>,
    mut arcball: ResMut<Arcball>,
) {
    // Process new fly-to requests. A fly-to hands control back to the
    // deterministic north-up spherical path, so drop the live trackball.
    for request in fly_requests.read() {
        arcball.engaged = false;
        arcball.anchor = None;
        orbit_fly_to(&state, request.destination_ecef, &mut flight_state);
    }

    let flight = match flight_state.flight.as_mut() {
        Some(f) if !f.complete => f,
        _ => return,
    };

    let dt = time.delta_secs() as f64;
    if let Some((position, _direction, _up)) = flight.update(dt) {
        let meters_per_render_unit = 6378137.0_f64;
        let (heading, pitch, distance) = ecef_to_orbit(position, meters_per_render_unit);
        state.heading = heading;
        state.pitch = pitch.clamp(-1.5, 1.5);
        state.distance = distance;
        state.target_distance = distance;
        if let Ok(mut transform) = query.get_single_mut() {
            *transform = compute_camera_transform(&state);
        }
    }

    if flight.complete {
        flight_state.flight = None;
    }
}

/// Initiates a great-arc flight to the given ECEF destination (meters).
///
/// Duration and easing are derived automatically from the distance using the
/// domain's [`compute_flight_duration`] and [`select_flight_easing`].
pub(crate) fn orbit_fly_to(
    state: &OrbitState,
    destination_ecef: DVec3,
    flight_state: &mut OrbitFlightState,
) {
    let meters_per_render_unit = 6378137.0_f64;
    let position = orbit_position_to_ecef(state, meters_per_render_unit);
    let direction = -position.normalize();
    let cam = cesium_camera::Camera::new(position, direction, DVec3::Z);

    let distance = (destination_ecef - position).length();
    let duration = compute_flight_duration(distance);
    let mut flight = CameraFlight::fly_to(&cam, destination_ecef, None, None, duration);
    flight.easing = select_flight_easing(distance);
    flight_state.flight = Some(flight);
}

/// Converts orbit state to an ECEF position in meters.
fn orbit_position_to_ecef(state: &OrbitState, meters_per_render_unit: f64) -> DVec3 {
    let d = state.distance as f64 * meters_per_render_unit;
    let cos_pitch = (state.pitch as f64).cos();
    let sin_pitch = (state.pitch as f64).sin();
    let heading = state.heading as f64;
    DVec3::new(
        d * cos_pitch * heading.cos(),
        d * cos_pitch * heading.sin(),
        d * sin_pitch,
    )
}

/// Converts an ECEF position (meters) back to orbit spherical coordinates.
fn ecef_to_orbit(position: DVec3, meters_per_render_unit: f64) -> (f32, f32, f32) {
    let r = position.length();
    let distance = (r / meters_per_render_unit) as f32;
    let pitch =
        position.z.atan2((position.x * position.x + position.y * position.y).sqrt()) as f32;
    let heading = position.y.atan2(position.x) as f32;
    (heading, pitch, distance)
}

/// Compute camera Transform from spherical orbit state.
///
/// The globe is ECEF: north pole at +Z, equator in the XY plane. The camera
/// position is expressed in spherical coordinates around the Z (polar) axis:
///   x = distance * cos(pitch) * cos(heading)
///   y = distance * cos(pitch) * sin(heading)
///   z = distance * sin(pitch)
/// and the camera's "up" is the globe's +Z axis, so north is always up.
fn compute_camera_transform(state: &OrbitState) -> Transform {
    let cos_pitch = state.pitch.cos();
    let sin_pitch = state.pitch.sin();

    let offset = Vec3::new(
        state.distance * cos_pitch * state.heading.cos(),
        state.distance * cos_pitch * state.heading.sin(),
        state.distance * sin_pitch,
    );

    let position = state.target + offset;
    Transform::from_translation(position).looking_at(state.target, Vec3::Z)
}

// ── FIX-ARCBALL helpers ──────────────────────────────────────────────────────

/// Build the arcball orientation that reproduces the legacy north-up polar
/// pose of the current spherical state. Used to latch the trackball onto the
/// existing view the instant a drag begins, so engagement is seamless.
fn arcball_quat_from_spherical(state: &OrbitState) -> Quat {
    compute_camera_transform(state).rotation
}

/// Camera `Transform` from an arcball orientation: the camera sits at
/// `target + orientation·(Ẑ · distance)` and looks back along `-orientation·Ẑ`.
/// For the orientation produced by [`arcball_quat_from_spherical`] this is
/// identical to [`compute_camera_transform`] (the rig's +Z axis points from the
/// target to the camera, so `-Z` — Bevy's camera forward — aims at the target).
fn transform_from_arcball(orientation: Quat, distance: f32, target: Vec3) -> Transform {
    let position = target + orientation * (Vec3::Z * distance);
    Transform::from_translation(position).with_rotation(orientation)
}

/// Derive `(heading, pitch)` — the view direction only, roll intentionally
/// dropped — from an arcball orientation, so the spherical consumers (globe
/// LOD sub-camera point, inertia) stay populated while the trackball is live.
fn orbit_from_orientation(orientation: DQuat) -> (f32, f32) {
    let dir = orientation * DVec3::Z; // normalize(position - target)
    (
        dir.y.atan2(dir.x) as f32,
        dir.z.clamp(-1.0, 1.0).asin() as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_state_default_is_pixel_neutral() {
        let state = OrbitState::default();
        assert_eq!(state.heading, 0.0);
        assert!((state.pitch - 0.4).abs() < 1e-6);
        assert!((state.distance - 3.0).abs() < 1e-6);
    }

    #[test]
    fn clamp_rotation_angle_bounds_large_steps_only() {
        // A small rotation (an ordinary drag step) passes through byte-identical.
        let small = DQuat::from_axis_angle(DVec3::Z, 0.02);
        assert_eq!(clamp_rotation_angle(small, MAX_DRAG_STEP_RAD), small);

        // A huge rotation (a teleport-sized lurch) is capped to the limit while
        // keeping its axis, so the view glides instead of jumping.
        let big = DQuat::from_axis_angle(DVec3::X, 1.2);
        let (axis, angle) = clamp_rotation_angle(big, MAX_DRAG_STEP_RAD).to_axis_angle();
        assert!((angle - f64::from(MAX_DRAG_STEP_RAD)).abs() < 1e-5);
        assert!((axis.abs() - DVec3::X.abs()).length() < 1e-5);
    }

    #[test]
    fn clamp_rotation_angle_preserves_1to1_tracking_scale() {
        // At whole-globe zoom a normal fast drag step (~0.03 rad) must NOT be
        // clamped, so the grabbed point still lands exactly under the cursor.
        let typical = DQuat::from_axis_angle(DVec3::Y, 0.03);
        assert_eq!(clamp_rotation_angle(typical, MAX_DRAG_STEP_RAD), typical);
    }

    #[test]
    fn compute_transform_produces_correct_position() {
        let state = OrbitState {
            heading: 0.0,
            pitch: 0.0,
            distance: 3.0,
            ..Default::default()
        };
        let t = compute_camera_transform(&state);
        // At heading=0, pitch=0: position = (3, 0, 0)
        assert!((t.translation.x - 3.0).abs() < 1e-5);
        assert!((t.translation.y).abs() < 1e-5);
        assert!((t.translation.z).abs() < 1e-5);
    }

    #[test]
    fn ecef_to_orbit_roundtrip() {
        let mpru = 6378137.0_f64;
        let state = OrbitState {
            heading: 0.5,
            pitch: 0.3,
            distance: 3.0,
            ..Default::default()
        };
        let ecef = orbit_position_to_ecef(&state, mpru);
        let (h, p, d) = ecef_to_orbit(ecef, mpru);
        assert!((h - state.heading).abs() < 1e-5);
        assert!((p - state.pitch).abs() < 1e-5);
        assert!((d - state.distance).abs() < 1e-4);
    }

    #[test]
    fn flight_duration_and_easing_from_domain() {
        // Short hop → quintic, 1s minimum.
        assert!((compute_flight_duration(500_000.0) - 1.0).abs() < 1e-12);
        assert_eq!(
            select_flight_easing(500_000.0),
            cesium_camera::EasingFunction::QuinticInOut
        );
        // Long hop → cubic, capped at 5s.
        assert!((compute_flight_duration(10_000_000.0) - 5.0).abs() < 1e-12);
        assert_eq!(
            select_flight_easing(2_000_000.0),
            cesium_camera::EasingFunction::CubicInOut
        );
    }

    #[test]
    fn orbit_fly_to_creates_valid_flight() {
        let state = OrbitState::default();
        let mut fs = OrbitFlightState::default();
        let dest = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);
        orbit_fly_to(&state, dest, &mut fs);
        assert!(fs.flight.is_some());
        let f = fs.flight.as_ref().unwrap();
        assert!(!f.complete);
        assert!(f.duration >= 1.0 && f.duration <= 5.0);
    }

    #[test]
    fn inertia_coasts_in_pixel_space_via_boundary_conversion() {
        // The domain InertiaController coasts in PIXEL space (0.5 px stop
        // guard), so the app converts a radian velocity → pixels on capture
        // and back to radians on maintain. Feeding raw radians (~0.01) would
        // fall straight through the stop guard and never coast.
        let state = OrbitState {
            heading: 0.0,
            pitch: 0.4,
            distance: 3.0,
            ..Default::default()
        };
        let (scale_h, scale_p) = inertia_pixel_scale(&state, 720.0);
        // At distance=3 (surface_dist=2), focal≈623.5: scales are >>1 px/rad,
        // so a 0.02 rad flick is a multi-pixel motion that clears the guard.
        assert!(scale_h > 100.0 && scale_p > 100.0);

        // A realistic flick: ~0.02 rad heading / 0.01 rad pitch in one frame.
        let heading_delta = 0.02_f64;
        let pitch_delta = 0.01_f64;
        let motion_px = DVec2::new(heading_delta * scale_h as f64, pitch_delta * scale_p as f64);

        let mut ctrl = InertiaController::new();
        ctrl.capture(InertiaState::Spin, DVec2::ZERO, motion_px * 2.0);
        ctrl.activate(Some(InertiaState::Spin));

        // First coasting frame (16 ms after release): still above the guard.
        let sample = InertiaSample::new(INERTIA_SPIN_COEFFICIENT, 0.0, 0.0, 16.0);
        let delta_px = ctrl.maintain(InertiaState::Spin, &sample).expect("coasting");

        // Convert back to radians: decay(0.016s, 0.9) = exp(-2.5*0.016) ≈ 0.9608,
        // so the coasted radian velocity is just under the released velocity.
        let heading_back = delta_px.x / scale_h as f64;
        let pitch_back = delta_px.y / scale_p as f64;
        assert!(heading_back > 0.0 && heading_back <= heading_delta);
        assert!(
            (heading_back - heading_delta * 0.9608).abs() < 1e-3,
            "heading coast {heading_back} should ≈ {}",
            heading_delta * 0.9608
        );
        assert!(pitch_back > 0.0 && pitch_back <= pitch_delta);
    }

    #[test]
    fn inertia_pixel_scale_matches_grab_the_globe_gain() {
        // The scale must be the exact inverse of the grab-the-globe gain so the
        // radian→pixel→radian round-trip is lossless. Both axes use the plain
        // geometric gain (focal/surface_dist); the old 1/cos(pitch) meridian-
        // convergence factor belongs to the legacy ECEF-around-Z model and was
        // dropped when the arcball (camera-relative) tracking took over.
        let state = OrbitState {
            pitch: 0.3,
            distance: 4.0,
            ..Default::default()
        };
        let win_h = 900.0_f32;
        let (scale_h, scale_p) = inertia_pixel_scale(&state, win_h);
        let focal = (win_h * 0.5) / (CAMERA_FOV_Y * 0.5).tan();
        let surface_dist = state.distance - GLOBE_RADIUS;
        assert!((scale_h - focal / surface_dist).abs() < 1e-3);
        assert!((scale_p - focal / surface_dist).abs() < 1e-3);

        // Round-trip: a radian delta → pixels → radians is identity.
        let d_heading = 0.05_f64;
        let px = d_heading * scale_h as f64;
        assert!((px / scale_h as f64 - d_heading).abs() < 1e-9);
    }

    #[test]
    fn inertia_stops_after_threshold() {
        let mut ctrl = InertiaController::new();
        ctrl.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(0.02, 0.0));
        // Held for 0.5s ≥ INERTIA_MAX_CLICK_TIME_THRESHOLD → no coasting.
        let sample = InertiaSample::new(INERTIA_SPIN_COEFFICIENT, 0.0, 500.0, 516.0);
        assert!(ctrl.maintain(InertiaState::Spin, &sample).is_none());
    }

    /// M2.4 verification gate: headless keyframe playback neutrality.
    ///
    /// Reproduces the `--headless --camera-script` path exactly:
    /// `camera_script_system` overwrites `OrbitState` each frame and there is
    /// **no** mouse / wheel / fly-to input, so the delegated rotation-inertia
    /// and great-arc-flight systems must contribute exactly zero. The resulting
    /// `Transform` therefore equals the pure grab-the-globe transform of the
    /// scripted state — bit-identical to the legacy M0 build (pos/quat diff
    /// `0.0 < 1e-4` render units) for every scripted pose.
    ///
    /// Ten distinct scripted poses stand in for the ten gesture scripts; the
    /// neutrality argument is per-frame and script-independent, so this covers
    /// the whole family. Runs on `MinimalPlugins` (no GPU / render backend).
    #[test]
    fn headless_keyframe_playback_matches_legacy_within_1e4() {
        // A 10-pose scripted trajectory (heading sweeps a full turn, pitch and
        // distance vary) — the deterministic equivalent of the capture scripts.
        let scripted: Vec<(f32, f32, f32)> = (0..10)
            .map(|i| {
                let t = i as f32 / 9.0;
                (
                    t * std::f32::consts::TAU,
                    0.4 + 0.15 * t,
                    3.0 - 0.75 * t,
                )
            })
            .collect();

        let mut max_pos_err = 0.0_f32;
        let mut max_quat_err = 0.0_f32;

        for &(heading, pitch, distance) in &scripted {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .add_event::<MouseMotion>()
                .add_event::<MouseWheel>()
                .add_event::<OrbitFlyToRequest>()
                .init_resource::<ButtonInput<MouseButton>>()
                .init_resource::<OrbitInertiaState>()
                .init_resource::<OrbitFlightState>()
                .init_resource::<Arcball>()
                .insert_resource(OrbitState {
                    heading,
                    pitch,
                    distance,
                    target_distance: distance,
                    ..Default::default()
                })
                .add_systems(
                    Update,
                    (orbit_camera_system, orbit_inertia_system, orbit_flight_system).chain(),
                );
            let cam = app.world_mut().spawn((OrbitCamera, Transform::IDENTITY)).id();

            // One headless frame with no input events (mirrors playback).
            app.update();

            let got = *app.world().get::<Transform>(cam).expect("camera transform");
            // The legacy transform is the pure function of the scripted state
            // (grab-the-globe body is byte-preserved; glide is a no-op since
            // target_distance == distance).
            let expected = compute_camera_transform(&OrbitState {
                heading,
                pitch,
                distance,
                target_distance: distance,
                ..Default::default()
            });

            max_pos_err = max_pos_err.max(got.translation.distance(expected.translation));
            let quat_err = got
                .rotation
                .to_array()
                .iter()
                .zip(expected.rotation.to_array().iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f32, f32::max);
            max_quat_err = max_quat_err.max(quat_err);
        }

        assert!(
            max_pos_err < 1e-4,
            "playback pos drift {max_pos_err} render units must be < 1e-4"
        );
        assert!(
            max_quat_err < 1e-4,
            "playback quat drift {max_quat_err} must be < 1e-4"
        );
    }
}
