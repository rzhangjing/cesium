//! cesium-app: CesiumRust 3D Globe Viewer
//!
//! Interactive 3D globe with:
//! - Base sphere + polar caps (non-LOD safety net)
//! - Dynamic LOD tiles with Bing Maps satellite imagery
//! - Orbit camera (mouse drag to rotate, scroll to zoom)
//! - Atmospheric limb glow + starfield background

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_bevy_render::{
    CesiumCorePlugin, CesiumGlobe, CesiumTilesetPlugin, CesiumTerrainPlugin, GlobeConfig,
    TileLoadStats, CesiumTilesetRoot, TilesetLoadingState,
    CameraControlPort, camera_control_port_system,
    LightingMode, CesiumAtmospherePlugin, CesiumShadowPlugin, CesiumEffectsPlugin,
    CesiumImageryPlugin,
};
mod orbit_camera;
mod starfield;
mod atmosphere_glow;
mod tile_mesh;
mod base_sphere;
mod globe_lod;
mod globe_textures;
mod globe_pipeline;
mod dynamic_globe;
mod dynamic_globe_legacy;
mod feature_flags;
mod perf_counters;
mod perf_trace;
mod capture_script;
mod offline_check;
mod material_showcase;

use orbit_camera::OrbitCameraPlugin;
use material_showcase::MaterialShowcasePlugin;
use starfield::StarfieldPlugin;
use atmosphere_glow::AtmosphereGlowPlugin;
use base_sphere::BaseSphereMarker;
use tile_mesh::{create_mercator_uv_sphere, create_polar_cap, render_scale};
use feature_flags::{terrain_enabled, tileset_enabled, lighting_mode, postprocess_builtin_enabled, postprocess_enabled, skydome_enabled, glow_enabled};
use perf_counters::PerfCounters;
use perf_trace::PerfTracePlugin;
use bevy::time::TimeUpdateStrategy;
use serde::Deserialize;

const TILE_SEGMENTS: u32 = 16;

// ── Cesium Ion terrain configuration (async, off the main thread) ────────
/// Resolves the Cesium World Terrain endpoint via the ion API and returns the
/// `{z}/{x}/{y}` template URL, or `None` when the token is missing / the call
/// fails (graceful degradation to an idle terrain chain).
///
/// Runs on an [`IoTaskPool`] worker (blocking ureq is fine there); it must
/// never run on the Startup/frame thread or the first frame would stall.
fn resolve_terrain_endpoint() -> Option<String> {
    let token = match std::env::var("CESIUM_ION_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ => {
            warn!("[terrain] CESIUM_ION_TOKEN not set. Terrain stays disabled (idle).");
            return None;
        }
    };

    let endpoint_url = format!(
        "https://api.cesium.com/v1/assets/1/endpoint?access_token={}",
        token
    );

    let response = match ureq::get(&endpoint_url).timeout(std::time::Duration::from_secs(10)).call() {
        Ok(r) => r,
        Err(e) => {
            warn!("[terrain] Failed to query Cesium ion endpoint: {}. Terrain disabled.", e);
            return None;
        }
    };

    let body: serde_json::Value = match response
        .into_string()
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(v) => v,
        None => {
            warn!("[terrain] Failed to parse ion endpoint JSON. Terrain disabled.");
            return None;
        }
    };

    let base_url = body["url"].as_str().unwrap_or_default();
    let access_token = body["accessToken"].as_str().unwrap_or_default();

    if base_url.is_empty() || access_token.is_empty() {
        warn!("[terrain] Ion endpoint returned empty url/token. Terrain disabled.");
        return None;
    }

    let template = format!(
        "{}{{z}}/{{x}}/{{y}}.terrain?extensions=octvertexnormals&access_token={}",
        base_url, access_token
    );
    info!("[terrain] Cesium World Terrain configured: {}...", &template[..80.min(template.len())]);
    Some(template)
}

/// In-flight ion endpoint resolution; removed once it resolves.
#[derive(Resource)]
struct PendingTerrainEndpoint {
    task: Task<Option<String>>,
}

/// Startup: spawn the ion endpoint resolution on the IoTaskPool. Non-blocking.
fn spawn_terrain_config_task(mut commands: Commands) {
    let pool = IoTaskPool::get();
    let task = pool.spawn(async move { resolve_terrain_endpoint() });
    commands.insert_resource(PendingTerrainEndpoint { task });
}

/// Update: poll the endpoint task once per frame; when ready, back-fill
/// `GlobeConfig.terrain_provider_url` and drop the pending resource.
fn apply_terrain_endpoint(
    mut commands: Commands,
    pending: Option<ResMut<PendingTerrainEndpoint>>,
    mut config: ResMut<GlobeConfig>,
) {
    let Some(mut pending) = pending else { return };
    let Some(template) = block_on(poll_once(&mut pending.task)) else { return };
    if let Some(url) = template {
        config.terrain_provider_url = Some(url);
    }
    commands.remove_resource::<PendingTerrainEndpoint>();
}

/// Diagnostic: logs TileLoadStats every 5 seconds.
fn stats_logger_system(
    stats: Res<TileLoadStats>,
    time: Res<Time>,
    mut timer: Local<Option<Timer>>,
) {
    if timer.is_none() {
        *timer = Some(Timer::from_seconds(5.0, TimerMode::Repeating));
    }
    if let Some(t) = timer.as_mut() {
        t.tick(time.delta());
        if t.just_finished() {
            info!(
                "[stats] loaded={} failed={} skipped={} pending={} bytes={}",
                stats.tiles_loaded, stats.tiles_failed, stats.tiles_skipped,
                stats.tiles_pending, stats.bytes_downloaded
            );
        }
    }
}

/// Startup: spawn a 3D Tiles tileset root.
///
/// Default is the Cesium sample tileset (uncompressed b3dm) served straight from
/// GitHub raw — a public, token-free online source. Override with
/// `CESIUM_TILESET_URL` (e.g. a local HTTP server) for offline runs.
fn spawn_tileset_root(mut commands: Commands) {
    let url = std::env::var("CESIUM_TILESET_URL").unwrap_or_else(|_| {
        "https://raw.githubusercontent.com/CesiumGS/cesium/main/Apps/SampleData/Cesium3DTiles/Tilesets/Tileset/tileset.json".to_string()
    });
    info!("[tileset] Spawning CesiumTilesetRoot: {}", url);
    commands.spawn(CesiumTilesetRoot {
        url,
        loading_state: TilesetLoadingState::NotLoaded,
    });
}

/// Headless verification aid, inert unless `CESIUM_SCREENSHOT_AT_FRAME` is set:
/// captures a screenshot at that frame and exits shortly after. Lets reviewers
/// grab a deterministic frame (default or opt-in chains) without altering the
/// default interactive run.
#[derive(Resource)]
struct AutoScreenshot {
    frame: u32,
    at_frame: u32,
    path: String,
}

fn auto_screenshot_system(
    mut commands: Commands,
    mut shot: ResMut<AutoScreenshot>,
    mut exit: EventWriter<AppExit>,
    cameras: Query<(&Transform, &Projection), With<Camera>>,
) {
    shot.frame += 1;
    if shot.frame == shot.at_frame {
        let path = shot.path.clone();
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
        info!("[shot] capturing at frame {}", shot.at_frame);

        // M3: write camera pos/quat JSON alongside the screenshot.
        // Schema: {"pos":[x,y,z],"quat":[x,y,z,w],"fov_y":<deg>}
        // This is the M2.4 playback-fidelity reference (<1e-4 render units).
        if let Ok((transform, projection)) = cameras.get_single() {
            let p = transform.translation;
            let q = transform.rotation;
            let fov_y_deg = match projection {
                Projection::Perspective(pp) => pp.fov.to_degrees(),
                Projection::Orthographic(_) => 0.0,
            };
            let json = format!(
                "{{\"pos\":[{:.6},{:.6},{:.6}],\"quat\":[{:.6},{:.6},{:.6},{:.6}],\"fov_y\":{:.4}}}",
                p.x, p.y, p.z,
                q.x, q.y, q.z, q.w,
                fov_y_deg,
            );
            // Derive .camera.json path from the screenshot path.
            let cam_path = std::path::Path::new(&path)
                .with_extension("camera.json");
            if let Err(e) = std::fs::write(&cam_path, &json) {
                warn!("[shot] failed to write camera JSON {}: {}", cam_path.display(), e);
            } else {
                info!("[shot] camera state written: {}", cam_path.display());
            }
        }
    }
    if shot.frame == shot.at_frame + 30 {
        exit.send(AppExit::Success);
    }
}

// ── M3.3: FIXED_CAMERA — deterministic pose override ────────────────────

/// TOML schema for `FIXED_CAMERA` file: a single `[camera]` table.
#[derive(Debug, Deserialize)]
struct FixedCameraFile {
    camera: capture_script::CameraPose,
}

/// Resource holding a frozen camera pose applied every PostUpdate frame.
#[derive(Resource)]
struct FixedCameraPose {
    pos: Vec3,
    quat: Quat,
    fov_y: Option<f32>,
}

/// PostUpdate system: overrides the orbit camera with the fixed pose.
/// Runs after orbit_camera's Update so the deterministic pose wins.
fn fixed_camera_system(
    pose: Res<FixedCameraPose>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera>>,
) {
    if let Ok((mut transform, mut projection)) = cameras.get_single_mut() {
        transform.translation = pose.pos;
        transform.rotation = pose.quat;
        if let Some(fov_deg) = pose.fov_y {
            if let Projection::Perspective(ref mut pp) = *projection {
                pp.fov = fov_deg.to_radians();
            }
        }
    }
}

// ── M3.4: Batch screenshot capture ─────────────────────────────────────

/// Resource driving batch multi-view capture (`CESIUM_SCREENSHOT_SCRIPT`).
#[derive(Resource)]
struct BatchCapture {
    script: Vec<capture_script::ShotEntry>,
    cursor: usize,
    frame: u32,
    output_dir: String,
    git_sha: String,
    env_snapshot: serde_json::Value,
}

/// PostUpdate system: advances the batch capture scheduler. Applies the due
/// shot's camera pose, spawns a `Screenshot`, writes `.camera.json` +
/// `.meta.json`, and exits after all shots + 30 buffer frames.
fn batch_capture_system(
    mut commands: Commands,
    mut batch: ResMut<BatchCapture>,
    mut exit: EventWriter<AppExit>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera>>,
) {
    batch.frame += 1;

    if let Some(idx) = capture_script::next_due(&batch.script, batch.cursor, batch.frame) {
        let entry = batch.script[idx].clone();

        // Apply scripted camera pose (overrides orbit_camera for this frame).
        if let Ok((mut transform, mut projection)) = cameras.get_single_mut() {
            transform.translation = Vec3::new(
                entry.pos[0] as f32,
                entry.pos[1] as f32,
                entry.pos[2] as f32,
            );
            transform.rotation = Quat::from_xyzw(
                entry.quat[0] as f32,
                entry.quat[1] as f32,
                entry.quat[2] as f32,
                entry.quat[3] as f32,
            );
            if let Some(fov_deg) = entry.fov_y {
                if let Projection::Perspective(ref mut pp) = *projection {
                    pp.fov = (fov_deg as f32).to_radians();
                }
            }
        }

        // Spawn screenshot.
        let png_path = format!("{}/{}.png", batch.output_dir, entry.name);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(png_path));

        // Write .camera.json (same schema as single-shot harness).
        let fov_y_deg = entry.fov_y.unwrap_or(60.0);
        let cam_json = format!(
            "{{\"pos\":[{:.6},{:.6},{:.6}],\"quat\":[{:.6},{:.6},{:.6},{:.6}],\"fov_y\":{:.4}}}",
            entry.pos[0], entry.pos[1], entry.pos[2],
            entry.quat[0], entry.quat[1], entry.quat[2], entry.quat[3],
            fov_y_deg,
        );
        let cam_path = format!("{}/{}.camera.json", batch.output_dir, entry.name);
        if let Err(e) = std::fs::write(&cam_path, &cam_json) {
            warn!("[batch] camera JSON write failed {}: {}", cam_path, e);
        }

        // Write .meta.json (frame/env/git_sha for pixel_diff gating).
        let meta_json = capture_script::metadata_json(
            &entry,
            batch.frame,
            &batch.git_sha,
            &batch.env_snapshot,
        );
        let meta_path = format!("{}/{}.meta.json", batch.output_dir, entry.name);
        if let Err(e) = std::fs::write(&meta_path, &meta_json) {
            warn!("[batch] metadata write failed {}: {}", meta_path, e);
        }

        info!(
            "[batch] captured '{}' at frame {} ({}/{})",
            entry.name,
            batch.frame,
            idx + 1,
            batch.script.len()
        );
        batch.cursor = idx + 1;
    }

    // Exit after all shots fired + 30 buffer frames for GPU flush.
    if batch.cursor >= batch.script.len() && !batch.script.is_empty() {
        let last_frame = batch.script.last().map_or(0, |e| e.frame);
        if batch.frame >= last_frame + 30 {
            info!(
                "[batch] all {} shots captured, exiting",
                batch.script.len()
            );
            exit.send(AppExit::Success);
        }
    }
}

/// Builds the env snapshot JSON for batch capture metadata.
fn build_env_snapshot() -> serde_json::Value {
    serde_json::json!({
        "strict_offline": feature_flags::strict_offline(),
        "offline_imagery_root": feature_flags::offline_imagery_root().map(|p| p.display().to_string()),
        "offline_terrain_root": feature_flags::offline_terrain_root().map(|p| p.display().to_string()),
        "fixed_time": feature_flags::fixed_time_enabled(),
        "fixed_camera": feature_flags::fixed_camera_path().map(|p| p.display().to_string()),
        "terrain_enabled": feature_flags::terrain_enabled(),
        "tileset_enabled": feature_flags::tileset_enabled(),
    })
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

    // Base sphere — high-subdivision UV sphere (Mercator-mapped V so the
    // runtime whole-globe composite texture drapes like the tile layer);
    // slightly smaller to stay below tiles and polar caps. Solid color is
    // only the pre-composite fallback: once the base tile layer arrives,
    // dynamic_globe drapes a blurry earth composite over it so transient
    // holes read as earth, not a flat blue void.
    let base_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.17, 0.19),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        CesiumGlobe,
        BaseSphereMarker,
        Mesh3d(meshes.add(create_mercator_uv_sphere(96, 48))),
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
}

fn main() {
    // M3.3: headless offline self-check — proves fetcher wiring without GPU.
    // Exits immediately (no Bevy app, no window, no GPU context needed).
    if feature_flags::offline_selfcheck() {
        std::process::exit(offline_check::run());
    }

    // M0.3: feature gates now live in `feature_flags`; the umbrella
    // `CESIUM_ENABLE_NEW_CHAINS` and the per-chain flags preserve their
    // pre-M0.3 semantics exactly (default OFF, truthy tokens unchanged).
    let enable_terrain = terrain_enabled();
    let enable_tileset = tileset_enabled();

    // M0.4: PerfCounters is always present (cheap; dynamic_globe writes
    // into it every frame). M0.5 perf-trace plugin reads it.
    let perf_counters = PerfCounters::default();

    // M0.5: CLI/env parsing for headless + trace + camera-script.
    let cli = perf_trace::Cli::from_env_and_args();

    let mut app = App::new();

    // M4.1: read lighting mode from env BEFORE plugin registration so
    // CesiumCorePlugin's Startup system sees the correct value.
    let lighting = lighting_mode();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "CesiumRust - 3D Globe Viewer".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        // M4.1: insert lighting mode before CesiumCorePlugin so setup_lighting
        // reads it (init_resource would use Default=FullAmbient otherwise).
        .insert_resource(lighting)
        // Core: lighting + globe config + AnimationClock
        .add_plugins(CesiumCorePlugin)
        // Camera: mouse orbit/zoom
        .add_plugins(OrbitCameraPlugin)
        // Globe rendering
        .add_plugins(BaseSpherePlugin);

    // M1.5: env-gated legacy fallback — CESIUMRST_LEGACY_DYNAMIC_GLOBE=1
    // selects the frozen ~2210-line monolith (see PIPELINE_PROMOTION_PLAN.md);
    // default is the thin shell.
    if feature_flags::legacy_dynamic_globe() {
        app.add_plugins(dynamic_globe_legacy::DynamicGlobePlugin);
    } else {
        app.add_plugins(dynamic_globe::DynamicGlobePlugin);
    }

    // ── M5-B: AtmosphereGlow ↔ SkyDome mutual exclusion ────────────────
    // The 8-shell glow fallback and the procedural sky dome render overlapping
    // atmospheric limb effects. glow_enabled() returns !skydome_enabled() so
    // they are never both active. Default (SKYDOME OFF) → glow ON → v0 zero-diff.
    // Safety: AtmosphereGlowPlugin's fade system reads Res<OrbitState> which is
    // guaranteed present because OrbitCameraPlugin is unconditionally registered
    // at L476 above (same pattern as #47 terrain plugin independence).
    if glow_enabled() {
        app.add_plugins(AtmosphereGlowPlugin);
    }
    app.add_plugins(StarfieldPlugin);

    // ── M4.1: shadow plugin (day_night lighting only) ───────────────────
    if lighting == LightingMode::DayNight {
        app.add_plugins(CesiumShadowPlugin);
        info!("[M4.1] DayNight lighting: shadow plugin registered");
    }

    // ── M5-B: procedural sky dome — independent SKYDOME gate ────────────
    // CesiumAtmospherePlugin is activated by SKYDOME=1, NOT by DayNight.
    // Decoupling sky rendering from lighting mode lets M5-C extend
    // sky_system.rs without touching main.rs. Default OFF → v0 zero-diff.
    // DEVIATION: sky dome gated independently of DayNight, mutually exclusive
    //   with AtmosphereGlowPlugin; see docs/deviations.md#dev-015
    if skydome_enabled() {
        app.add_plugins(CesiumAtmospherePlugin);
        info!("[M5-B] SkyDome: CesiumAtmospherePlugin registered");
    }

    // ── M4.2 + M5-E1: post-process (two independent gates, coexist) ────────
    // CesiumEffectsPlugin is registered when EITHER gate is ON:
    //   • CESIUM_ENABLE_POSTPROCESS_BUILTIN → M4.2 fog clear-color system
    //     (tonemapping / bloom / HDR live on the camera bundle in orbit_camera.rs).
    //   • CESIUM_ENABLE_POSTPROCESS         → M5-E1 FXAA render-graph node
    //     (self-implemented WGSL, quality preset 12 only; see effects/fxaa.rs).
    // The plugin internally gates each feature by its own env var (see
    // CesiumEffectsPlugin::build), so the two never cross-contaminate and there
    // is no double-FXAA (Bevy's built-in FxaaPlugin is not added; our node uses a
    // distinct CesiumPostProcessLabel::Fxaa + CesiumFxaa marker component).
    // Default (both OFF) → plugin not added → v0 baselines pixel-neutral (PSNR=∞).
    if postprocess_builtin_enabled() || postprocess_enabled() {
        app.add_plugins(CesiumEffectsPlugin);
        if postprocess_builtin_enabled() {
            info!("[M4.2] Built-in post-process: CesiumEffectsPlugin registered");
        }
        if postprocess_enabled() {
            info!("[M5-E1] FXAA post-process (preset 12): CesiumEffectsPlugin registered");
        }
    }

    // ── M5-D: Fabric material showcase (built-ins + 3 Water sea states) ──────
    // Pure additive, env-gated (default OFF → v0 baselines pixel-neutral).
    // Opt-in via CESIUM_ENABLE_MATERIAL_SHOWCASE=1 to render the material spheres
    // — including the faithfully-ported Water.glsl (case 17u) calm/medium/rough
    // baselines captured to specs/baselines/v2_water/. No existing plugin
    // registration is altered. Uses feature_flags::env_flag (pub) so this file
    // does not touch feature_flags.rs.
    if feature_flags::env_flag("CESIUM_ENABLE_MATERIAL_SHOWCASE") {
        app.add_plugins(MaterialShowcasePlugin);
        info!("[M5-D] Material showcase: MaterialShowcasePlugin registered");
    }

    // M0.4: register PerfCounters resource (dynamic_globe writes, trace reads).
    app.insert_resource(perf_counters);

    // ── M2.5: CameraControl driving port ────────────────────────────────
    // Programmatic camera control (set_view / fly_to / look_at / zoom / home),
    // delegating to the domain camera algorithms. Registered as a resource so
    // scripts/systems can fetch `CameraControlPort` and drive the camera; the
    // PostUpdate bridge syncs it to any `CesiumCamera` entity and is inert when
    // none exists (the interactive default camera stays orbit_camera-driven,
    // pixel-neutral).
    app.init_resource::<CameraControlPort>()
        .add_systems(PostUpdate, camera_control_port_system);

    // M0.5: perf-trace plugin (CSV writer + optional camera-script playback
    // + optional headless auto-exit). Inert when no trace path is given.
    app.add_plugins(PerfTracePlugin::new(cli));

    if enable_terrain {
        app.add_plugins(CesiumTerrainPlugin)
            // M4.3: imagery pipeline provides textures for terrain draping
            // (base_color_texture on terrain tiles). Registered alongside
            // terrain so ImageryCache is populated before render_system runs.
            .add_plugins(CesiumImageryPlugin)
            .add_systems(Startup, spawn_terrain_config_task)
            .add_systems(Update, apply_terrain_endpoint);
    }
    if enable_tileset {
        app.add_plugins(CesiumTilesetPlugin)
            .add_systems(Startup, spawn_tileset_root);
    }
    if enable_terrain || enable_tileset {
        app.add_systems(Update, stats_logger_system);
    }

    // Optional deterministic screenshot for headless verification (inert by default).
    if let Some(at_frame) = std::env::var("CESIUM_SCREENSHOT_AT_FRAME")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    {
        let path = std::env::var("CESIUM_SCREENSHOT").unwrap_or_else(|_| "screenshot.png".into());
        app.insert_resource(AutoScreenshot { frame: 0, at_frame, path });
        app.add_systems(Update, auto_screenshot_system);
    }

    // ── M3.3: FIXED_TIME — freeze clock for deterministic lighting ──────
    if feature_flags::fixed_time_enabled() {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
        info!("[M3.3] FIXED_TIME: clock frozen (delta=0)");
    }

    // ── M3.3: FIXED_CAMERA — deterministic pose override ────────────────
    if let Some(cam_path) = feature_flags::fixed_camera_path() {
        match std::fs::read_to_string(&cam_path) {
            Ok(text) => match toml::from_str::<FixedCameraFile>(&text) {
                Ok(file) => {
                    let p = file.camera.pos;
                    let q = file.camera.quat;
                    app.insert_resource(FixedCameraPose {
                        pos: Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32),
                        quat: Quat::from_xyzw(
                            q[0] as f32, q[1] as f32, q[2] as f32, q[3] as f32,
                        ),
                        fov_y: file.camera.fov_y.map(|f| f as f32),
                    });
                    app.add_systems(PostUpdate, fixed_camera_system);
                    info!("[M3.3] FIXED_CAMERA: pose from {}", cam_path.display());
                }
                Err(e) => warn!("[M3.3] FIXED_CAMERA parse failed: {e}"),
            },
            Err(e) => warn!("[M3.3] FIXED_CAMERA read failed {}: {e}", cam_path.display()),
        }
    }

    // ── M3.4: CESIUM_SCREENSHOT_SCRIPT — batch multi-view capture ───────
    if let Some(script_path) = feature_flags::screenshot_script_path() {
        match capture_script::ShotScript::from_file(&script_path) {
            Ok(script) => {
                let shots = script.sorted().shot;
                let output_dir = std::env::var("CESIUM_SCREENSHOT_DIR")
                    .unwrap_or_else(|_| ".".to_string());
                // Ensure output directory exists.
                let _ = std::fs::create_dir_all(&output_dir);
                info!(
                    "[M3.4] batch capture: {} shots from {} → {}/",
                    shots.len(),
                    script_path.display(),
                    output_dir
                );
                app.insert_resource(BatchCapture {
                    script: shots,
                    cursor: 0,
                    frame: 0,
                    output_dir,
                    git_sha: feature_flags::git_sha(),
                    env_snapshot: build_env_snapshot(),
                });
                app.add_systems(PostUpdate, batch_capture_system);
            }
            Err(e) => warn!("[M3.4] screenshot script parse failed: {e}"),
        }
    }

    app.run();
}
