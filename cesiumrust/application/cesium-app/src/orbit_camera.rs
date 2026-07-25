//! Orbit camera controller — mouse drag to rotate, scroll to zoom.
//!
//! Mimics CesiumJS default ScreenSpaceCameraController behavior:
//! left-drag orbits around the globe, wheel zooms in/out.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

/// Marker component for the orbit-controlled camera.
#[derive(Component)]
pub struct OrbitCamera;

/// Resource holding the orbit state (spherical coordinates around target).
#[derive(Resource)]
pub struct OrbitState {
    /// Heading angle in radians (rotation around Y axis).
    pub heading: f32,
    /// Pitch angle in radians (negative = looking down).
    pub pitch: f32,
    /// Distance from target in render units.
    pub distance: f32,
    /// Orbit target (world space).
    pub target: Vec3,
    /// Rotation sensitivity.
    pub rotate_speed: f32,
    /// Zoom sensitivity.
    pub zoom_speed: f32,
    /// Min zoom distance.
    pub min_distance: f32,
    /// Max zoom distance.
    pub max_distance: f32,
}

impl Default for OrbitState {
    fn default() -> Self {
        Self {
            heading: 0.0,
            pitch: -0.5, // ~-30 degrees, looking slightly down
            distance: 3.0,
            target: Vec3::ZERO,
            rotate_speed: 0.005,
            zoom_speed: 0.1,
            min_distance: 1.5,
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
    commands.spawn((
        Camera3d::default(),
        OrbitCamera,
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
) {
    // Rotation: left mouse drag
    if mouse_buttons.pressed(MouseButton::Left) {
        for ev in motion_events.read() {
            state.heading -= ev.delta.x * state.rotate_speed;
            state.pitch -= ev.delta.y * state.rotate_speed;
            // Clamp pitch to avoid gimbal flip
            state.pitch = state.pitch.clamp(-1.5, -0.05);
        }
    } else {
        // Consume events even when not dragging to avoid accumulation
        motion_events.clear();
    }

    // Zoom: mouse wheel
    for ev in wheel_events.read() {
        let zoom_delta = -ev.y * state.zoom_speed;
        state.distance *= 1.0 + zoom_delta;
        state.distance = state.distance.clamp(state.min_distance, state.max_distance);
    }

    // Apply transform
    if let Ok(mut transform) = query.get_single_mut() {
        *transform = compute_camera_transform(&state);
    }
}

/// Compute camera Transform from spherical orbit state.
fn compute_camera_transform(state: &OrbitState) -> Transform {
    // Spherical to Cartesian:
    // x = distance * cos(pitch) * sin(heading)
    // y = distance * sin(-pitch)  (pitch is negative for looking down)
    // z = distance * cos(pitch) * cos(heading)
    let cos_pitch = state.pitch.cos();
    let sin_pitch = state.pitch.sin();

    let offset = Vec3::new(
        state.distance * cos_pitch * state.heading.sin(),
        -state.distance * sin_pitch,
        state.distance * cos_pitch * state.heading.cos(),
    );

    let position = state.target + offset;
    Transform::from_translation(position).looking_at(state.target, Vec3::Y)
}
