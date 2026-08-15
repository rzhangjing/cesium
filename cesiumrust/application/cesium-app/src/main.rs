//! cesium-app: CesiumRust 3D Globe Viewer
//!
//! Interactive 3D globe with:
//! - Base sphere + polar caps (non-LOD safety net)
//! - Dynamic LOD tiles with Bing Maps satellite imagery
//! - Orbit camera (mouse drag to rotate, scroll to zoom)
//! - Atmospheric limb glow + starfield background

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use cesium_bevy_render::{CesiumCorePlugin, CesiumGlobe};
mod orbit_camera;
mod starfield;
mod atmosphere_glow;
mod tile_mesh;
mod dynamic_globe;

use orbit_camera::{OrbitCameraPlugin, OrbitState};
use starfield::StarfieldPlugin;
use atmosphere_glow::AtmosphereGlowPlugin;
use dynamic_globe::DynamicGlobePlugin;
use tile_mesh::{create_polar_cap, create_uv_sphere, render_scale};

const TILE_SEGMENTS: u32 = 16;

/// Diagnostic autopilot toggle: drives the camera programmatically (rotate
/// then zoom) while capturing frames straight off the GPU. Used to reproduce
/// motion artifacts without synthetic OS input, which a remote or sandboxed
/// session may not deliver to the window.
const DIAG_AUTOPILOT: bool = true;

/// Spawn one GPU screenshot with a numbered path; the observer saves it when
/// the renderer delivers the captured frame.
fn take_screenshot(commands: &mut Commands, idx: &mut u32) {
    let path = format!("d:/Rust/cesium/shot_{:04}.png", *idx);
    *idx += 1;
    commands
        .spawn(Screenshot::primary_window())
        .observe(move |trigger: Trigger<ScreenshotCaptured>| {
            let img = trigger.event().0.clone();
            if let Ok(dyn_img) = img.try_into_dynamic() {
                if let Err(e) = dyn_img
                    .to_rgb8()
                    .save_with_format(&path, image::ImageFormat::Png)
                {
                    println!("[Diag] save failed: {e}");
                } else {
                    println!("[Diag] saved {path}");
                }
            }
        });
}

/// F12 or middle-click: capture one frame from the primary camera straight
/// off the GPU and save it to disk — bypasses the desktop compositor entirely,
/// so it works even when the window is occluded or the session is remote.
/// (Middle-click is the automation fallback: synthetic keyboard events need
/// window focus, synthetic mouse buttons only need the cursor on the window.)
fn f12_screenshot(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    mut idx: Local<u32>,
) {
    if keys.just_pressed(KeyCode::F12) || mouse.just_pressed(MouseButton::Middle) {
        take_screenshot(&mut commands, &mut idx);
    }
}

/// Autopilot timeline: 8 s settle -> 6 s of rotation -> 6 s of zoom-in, with
/// a GPU capture every 150 ms throughout the motion (and a short tail after
/// it stops, so post-motion convergence is sampled too).
fn diag_autopilot(
    mut orbit: ResMut<OrbitState>,
    time: Res<Time>,
    mut t: Local<f32>,
    mut last_shot: Local<f32>,
    mut commands: Commands,
    mut idx: Local<u32>,
) {
    if !DIAG_AUTOPILOT {
        return;
    }
    *t += time.delta_secs();
    let s = *t;
    if (8.0..14.0).contains(&s) {
        // Slow continuous rotation, like a steady left drag.
        orbit.heading += 0.12 * time.delta_secs();
    }
    if (14.0..20.0).contains(&s) {
        // Steady zoom-in through the exponential distance easing.
        orbit.target_distance =
            (orbit.target_distance - 0.22 * time.delta_secs()).max(orbit.min_distance);
    }
    if (8.0..21.5).contains(&s) {
        *last_shot += time.delta_secs();
        if *last_shot >= 0.15 {
            *last_shot = 0.0;
            take_screenshot(&mut commands, &mut idx);
        }
    }
}

/// Plugin that spawns the base sphere and polar caps.
struct BaseSpherePlugin;

impl Plugin for BaseSpherePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_base_sphere);
    }
}

fn spawn_base_sphere(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scale = render_scale();

    // Base sphere — high-subdivision UV sphere so the horizon silhouette is
    // smooth; slightly smaller to stay below tiles and polar caps. Color
    // matches the lit ocean so any transient hole reads as sea, not a black
    // void.
    let base_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.17, 0.19),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        CesiumGlobe,
        Mesh3d(meshes.add(create_uv_sphere(96, 48))),
        MeshMaterial3d(base_material),
        Transform::from_scale(Vec3::splat(scale * 0.99)),
    ));

    // Polar caps — north cap steel-blue matched to the LIT ocean color so
    // the Arctic pole continues the surrounding sea seamlessly (reference:
    // CesiumJS whole-globe look); south cap ice white because the 85° tile
    // ring around Antarctica is white ice and the cap must continue it.
    let north_cap_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.17, 0.19),
        perceptual_roughness: 0.95,
        ..default()
    });
    let south_cap_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.88, 0.92, 0.96),
        perceptual_roughness: 0.95,
        ..default()
    });
    for &north in &[true, false] {
        commands.spawn((
            CesiumGlobe,
            Mesh3d(meshes.add(create_polar_cap(north, TILE_SEGMENTS * 4))),
            MeshMaterial3d(if north {
                north_cap_material.clone()
            } else {
                south_cap_material.clone()
            }),
            Transform::from_scale(Vec3::splat(scale)),
        ));
    }
    println!("[BaseSphere] Spawned base sphere + 2 polar caps");
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "CesiumRust - 3D Globe Viewer".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        // FPS / frame-time diagnostics (console) for performance validation
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_systems(Update, (f12_screenshot, diag_autopilot))
        // Core: lighting + globe config
        .add_plugins(CesiumCorePlugin)
        // Camera: mouse orbit/zoom
        .add_plugins(OrbitCameraPlugin)
        // Globe rendering
        .add_plugins(BaseSpherePlugin)
        .add_plugins(DynamicGlobePlugin)
        // Visual effects
        .add_plugins(AtmosphereGlowPlugin)
        .add_plugins(StarfieldPlugin)
        // ── New architecture plugins (add gradually) ──
        // Phase 1: uncomment one by one to verify
        // .add_plugins(CesiumTilesetPlugin)
        // .add_plugins(CesiumCameraPlugin)
        // .add_plugins(CesiumImageryPlugin)
        // .add_plugins(CesiumTerrainPlugin)
        // .add_plugins(CesiumEntityPlugin)
        // .add_plugins(CesiumMaterialPlugin)
        // .add_plugins(CesiumAtmospherePlugin)
        .run();
}
