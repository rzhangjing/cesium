//! Orbit camera controller — mouse drag to rotate, scroll to zoom.
//!
//! Mimics CesiumJS default ScreenSpaceCameraController behavior:
//! left-drag orbits around the globe, wheel zooms in/out.
//!
//! The globe is in ECEF orientation (north pole at +Z, equator in the XY
//! plane), so the camera orbits around the Z (polar) axis with Z as "up".

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// Camera vertical field of view (radians). Kept in sync between the spawned
/// projection and the drag math so the grab-the-globe tracking is exact.
pub const CAMERA_FOV_Y: f32 = std::f32::consts::FRAC_PI_3; // 60 degrees
/// Near clip plane — small enough to see the surface when zoomed in close.
const CAMERA_NEAR: f32 = 0.002;
/// Far clip plane — large enough for the starfield (radius ~50).
const CAMERA_FAR: f32 = 200.0;
/// Globe (equatorial) radius in render units.
const GLOBE_RADIUS: f32 = 1.0;

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
            target: Vec3::ZERO,
            rotate_speed: 1.0, // exact geometric tracking by default
            zoom_speed: 0.3,
            min_distance: 1.005, // hover just above the surface
            max_distance: 20.0,
        }
    }
}

/// Plugin that sets up the orbit camera.
pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OrbitState>()
            .add_systems(Startup, spawn_orbit_camera)
            .add_systems(Update, orbit_camera_system);
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
    commands.spawn((
        Camera3d::default(),
        // CesiumJS displays imagery as-is without tonemapping; the default
        // TonyMcMapFace also requires the `tonemapping_luts` feature which is
        // disabled in this workspace (missing LUT renders everything magenta).
        Tonemapping::None,
        OrbitCamera,
        Projection::Perspective(projection),
        transform,
    ));
}

/// System: read mouse input and update camera transform.
fn orbit_camera_system(
    mut state: ResMut<OrbitState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion_events: EventReader<MouseMotion>,
    mut wheel_events: EventReader<MouseWheel>,
    mut query: Query<&mut Transform, With<OrbitCamera>>,
    windows: Query<&Window>,
) {
    // Rotation: left mouse drag
    if mouse_buttons.pressed(MouseButton::Left) {
        // Exact grab-the-globe tracking from the real camera geometry:
        //   focal length f = (H/2) / tan(fov/2)   (pixels per radian)
        //   surface_dist  = distance - R           (camera -> surface at center)
        // A pitch rotation dPitch moves the surface point R*dPitch world units,
        // which projects to R*dPitch*f/surface_dist pixels; solving for the
        // rotation that matches a drag of dy pixels gives dPitch = dy*dist/f.
        // Heading is the same but divided by cos(pitch) because meridians
        // converge toward the poles (clamped to avoid runaway spin there).
        let win_h = windows
            .get_single()
            .map(|w| w.height())
            .unwrap_or(720.0);
        let focal = (win_h * 0.5) / (CAMERA_FOV_Y * 0.5).tan();
        let surface_dist = (state.distance - GLOBE_RADIUS).max(0.001);
        let lat_factor = state.pitch.cos().max(0.15);

        for ev in motion_events.read() {
            // Horizontal drag -> orbit around the globe's polar (Z) axis.
            // Sign chosen for a "grab the globe" feel: dragging right spins
            // the surface right, i.e. the camera azimuth decreases.
            state.heading -=
                ev.delta.x * state.rotate_speed * surface_dist / (lat_factor * focal);
            // Vertical drag -> move north/south (change elevation). Dragging
            // down pulls the surface down, revealing the north (pitch rises).
            state.pitch += ev.delta.y * state.rotate_speed * surface_dist / focal;
            // Clamp elevation to avoid gimbal lock directly over the poles
            // (keep the view direction off the Z axis by ~4 degrees).
            state.pitch = state.pitch.clamp(-1.5, 1.5);
        }
    } else {
        // Consume events even when not dragging to avoid accumulation
        motion_events.clear();
    }

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
        let surface_dist = (state.distance - GLOBE_RADIUS).clamp(min_surf, max_surf);
        // ev.y > 0 (scroll up) = zoom in -> shrink the height above the surface.
        let zoom_factor = 1.0 - ev.y * state.zoom_speed;
        let new_surf = (surface_dist * zoom_factor).clamp(min_surf, max_surf);
        state.distance = GLOBE_RADIUS + new_surf;
    }

    // Apply transform
    if let Ok(mut transform) = query.get_single_mut() {
        *transform = compute_camera_transform(&state);
    }
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
