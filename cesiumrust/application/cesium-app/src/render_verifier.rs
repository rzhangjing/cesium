//! Render verifier: systems that confirm the rendering pipeline is working.
//!
//! Provides:
//!   - `SceneStatsSystem`: prints scene stats every 5 seconds
//!   - `ScreenshotSystem`: on 'P' key, logs camera state and entity counts
//!   - `HealthCheckSystem`: verifies globe, camera, and imagery at startup

use bevy::prelude::*;
use cesium_bevy_render::{
    CesiumCamera, CesiumGlobe, CesiumTerrainTile, GlobeConfig,
    TileLoadStats,
};
use cesium_bevy_render::imagery::ImageryLayerManager;
use cesium_geospatial::ellipsoid::Ellipsoid;

use crate::orbit_camera::OrbitState;

// ── Plugin ────────────────────────────────────────────────────────────

pub struct RenderVerifierPlugin;

impl Plugin for RenderVerifierPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VerifierTimer>()
            .add_systems(Startup, health_check_startup)
            .add_systems(
                Update,
                (
                    periodic_scene_stats,
                    screenshot_system,
                ),
            );
    }
}

// ── Resources ────────────────────────────────────────────────────────

#[derive(Resource)]
struct VerifierTimer(Timer);

impl Default for VerifierTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}

// ── Health Check System ──────────────────────────────────────────────

fn health_check_startup(
    globe_query: Query<Entity, With<CesiumGlobe>>,
    camera_query: Query<Entity, With<CesiumCamera>>,
    terrain_tile_query: Query<Entity, With<CesiumTerrainTile>>,
    imagery_mgr: Res<ImageryLayerManager>,
    globe_config: Res<GlobeConfig>,
) {
    println!("═══ RenderVerifier — Health Check ═══");

    let checks = [
        (
            "Globe mesh",
            globe_query.iter().count() > 0,
        ),
        (
            "Camera (CesiumCamera)",
            camera_query.iter().count() > 0,
        ),
        (
            "Imagery layers",
            imagery_mgr.layer_count() > 0,
        ),
        (
            "Terrain tiles",
            terrain_tile_query.iter().count() > 0,
        ),
        (
            "Globe ellipsoid",
            globe_config.ellipsoid == Ellipsoid::WGS84,
        ),
    ];

    let mut all_ok = true;
    for (name, ok) in &checks {
        let icon = if *ok { "✓" } else { "✗" };
        if !ok {
            all_ok = false;
            eprintln!("  {}  {}: MISCONFIGURED", icon, name);
        } else {
            println!("  {}  {}: OK", icon, name);
        }
    }

    println!(
        "═══ Overall: {} ═══",
        if all_ok {
            "ALL CHECKS PASSED"
        } else {
            "SOME CHECKS FAILED"
        }
    );

    if !all_ok {
        eprintln!(
            "[RenderVerifier] ⚠  Warnings: {}",
            checks
                .iter()
                .filter(|(_, ok)| !ok)
                .map(|(n, _)| n.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}

// ── Periodic Scene Stats ─────────────────────────────────────────────

fn periodic_scene_stats(
    time: Res<Time>,
    mut timer: ResMut<VerifierTimer>,
    state: Res<OrbitState>,
    globe_config: Res<GlobeConfig>,
    imagery_mgr: Res<ImageryLayerManager>,
    stats: Res<TileLoadStats>,
    globe_query: Query<(), With<CesiumGlobe>>,
    tile_query: Query<&CesiumTerrainTile>,
    camera_query: Query<&CesiumCamera>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let globe_count = globe_query.iter().count();
    let tile_count = tile_query.iter().count();
    let imagery_count = imagery_mgr.layer_count();
    let fps = 1.0 / time.delta_secs();

    let camera_carto = if let Ok(cc) = camera_query.get_single() {
        globe_config
            .ellipsoid
            .cartesian_to_cartographic(cc.camera.position)
            .map(|c| format!(
                "{:.2}°{:.1} {:.2}°{:.1} h={:.0}m",
                c.latitude.to_degrees().abs(),
                if c.latitude >= 0.0 { 'N' } else { 'S' },
                c.longitude.to_degrees().abs(),
                if c.longitude >= 0.0 { 'E' } else { 'W' },
                c.height,
            ))
            .unwrap_or_else(|| "?".into())
    } else {
        "no_camera".into()
    };

    let orbit_carto = {
        let cam_pos = compute_cam_position(&state);
        globe_config
            .ellipsoid
            .cartesian_to_cartographic(cam_pos)
            .map(|c| format!(
                "{:.2}°{:.1}",
                c.latitude.to_degrees().abs(),
                if c.latitude >= 0.0 { 'N' } else { 'S' },
            ))
            .unwrap_or_else(|| "?".into())
    };

    println!(
        "[Stats] {} globes | {} tiles | {} imagery layers | {:.0} FPS | CesiumCam: {} | Orbit: {}",
        globe_count,
        tile_count,
        imagery_count,
        fps,
        camera_carto,
        orbit_carto,
    );

    if stats.tiles_loaded > 0 || stats.tiles_failed > 0 {
        println!(
            "[Stats] DL: {} ok / {} fail / {} MB",
            stats.tiles_loaded,
            stats.tiles_failed,
            stats.bytes_downloaded / 1_000_000,
        );
    }
}

// ── Screenshot/Frame Info System ─────────────────────────────────────

fn screenshot_system(
    keys: Res<ButtonInput<KeyCode>>,
    globe_query: Query<(), With<CesiumGlobe>>,
    tile_query: Query<&CesiumTerrainTile>,
    camera_query: Query<&CesiumCamera>,
    imagery_mgr: Res<ImageryLayerManager>,
    globe_config: Res<GlobeConfig>,
    state: Res<OrbitState>,
) {
    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }

    println!("═══ Frame Capture — Scene State ═══");
    println!("  Globe entities: {}", globe_query.iter().count());
    println!("  Terrain tiles: {}", tile_query.iter().count());
    println!("  Imagery layers: {}", imagery_mgr.layer_count());

    if let Ok(cc) = camera_query.get_single() {
        let pos = cc.camera.position;
        let dir = cc.camera.direction;
        let up = cc.camera.up;
        let carto = globe_config.ellipsoid.cartesian_to_cartographic(pos);

        println!("  Camera ECEF: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z);
        println!("  Camera dir:  ({:.4}, {:.4}, {:.4})", dir.x, dir.y, dir.z);
        println!("  Camera up:   ({:.4}, {:.4}, {:.4})", up.x, up.y, up.z);
        if let Some(c) = carto {
            println!("  Camera geo:  {:.4}°N {:.4}°W alt={:.0}m",
                c.latitude.to_degrees(), -c.longitude.to_degrees(), c.height
            );
        }
        println!("  Scene mode:  {:?}", cc.scene_mode);
    } else {
        println!("  Camera: NOT FOUND");
    }

    println!("  Orbit dist:  {:.3} RU", state.distance);
    println!("  Orbit head:  {:.3} rad", state.heading);
    println!("  Orbit pitch: {:.3} rad", state.pitch);
    println!("═════════════════════════════════════");
}

// ── Helpers ──────────────────────────────────────────────────────────

fn compute_cam_position(state: &OrbitState) -> glam::DVec3 {
    let cos_pitch = state.pitch.cos() as f64;
    let sin_pitch = state.pitch.sin() as f64;
    let distance = state.distance as f64;
    let heading = state.heading as f64;
    glam::DVec3::new(
        distance * cos_pitch * heading.cos(),
        distance * cos_pitch * heading.sin(),
        distance * sin_pitch,
    )
}

// ── Integration tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_timer_default() {
        let timer = VerifierTimer::default();
        assert_eq!(timer.0.duration().as_secs_f32(), 5.0);
        assert_eq!(timer.0.mode(), TimerMode::Repeating);
    }

    #[test]
    fn test_compute_cam_position_at_default() {
        let state = OrbitState::default();
        let pos = compute_cam_position(&state);
        let expected = glam::DVec3::new(
            3.0 * (0.4_f64).cos() * (0.0_f64).cos(),
            3.0 * (0.4_f64).cos() * (0.0_f64).sin(),
            3.0 * (0.4_f64).sin(),
        );
        assert!((pos - expected).length() < 0.01);
    }

    #[test]
    fn test_compute_cam_position_looking_north() {
        let state = OrbitState {
            heading: 0.0,
            pitch: std::f32::consts::FRAC_PI_2 - 0.01,
            distance: 3.0,
            ..default()
        };
        let pos = compute_cam_position(&state);
        assert!(pos.z > 2.5, "should be mostly Z due to high pitch, got z={}", pos.z);
    }

    #[test]
    fn test_compute_cam_position_east_view() {
        let state = OrbitState {
            heading: std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            distance: 3.0,
            ..default()
        };
        let pos = compute_cam_position(&state);
        assert!(pos.x.abs() < 0.01, "x should be ~0 for heading=PI/2, pitch=0");
        assert!(pos.y > 2.5, "y should be positive for heading=PI/2");
        assert!(pos.z.abs() < 0.01, "z should be ~0 for pitch=0");
    }
}
