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
use feature_flags::{terrain_enabled, tileset_enabled, lighting_mode, postprocess_builtin_enabled, postprocess_enabled, skydome_enabled, glow_enabled, material_showcase_enabled};
use perf_counters::PerfCounters;
use perf_trace::PerfTracePlugin;
use bevy::time::TimeUpdateStrategy;
use serde::Deserialize;
// ── M6 Wave A (task #81) ────────────────────────────────────────────────────
// The three M6 camera components, the render-graph entry point that owns their
// `Core3d` edges, and the *domain* value objects the components are built from.
// The domain types reach `cesium-app` through the re-exports in
// `adapters/bevy-render/src/effects/mod.rs` (the same pattern `LightingMode`
// already uses): `cesium-app` deliberately holds no direct dependency on the
// `cesium-effects` domain crate, so the adapter remains the only layer that
// narrows domain f64 to GPU f32.
use cesium_bevy_render::effects::{
    CesiumClippingPlanes, CesiumClouds, CesiumIbl, CesiumOit, CesiumPanorama, CesiumSplit,
    ClippingPlane, ClippingPlaneCollection, CloudCollection, CubeMapPanorama, CumulusCloud,
    IblMaterial, ImageBasedLighting, M6WaveARenderGraphPlugin,
};

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
        // FIX-HL-EXIT: under CESIUM_HEADLESS there is no primary window, so
        // `Screenshot::primary_window()` is inert (no capture observer fires). Make
        // that silent gap loud; the supported headless artefact is the M11.3
        // offscreen `CesiumHeadlessPlugin` capture path instead.
        if feature_flags::headless_enabled() {
            warn!(
                "[shot] AutoScreenshot uses Screenshot::primary_window(), inert under \
                 CESIUM_HEADLESS; rely on the headless offscreen capture for the PNG"
            );
        }
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
        // FIX-HL-EXIT: batch capture is likewise not yet headless-aware (see the
        // M11.4 NOTE at the plugin wiring); warn rather than fail silently.
        if feature_flags::headless_enabled() {
            warn!(
                "[batch] Screenshot::primary_window() is inert under CESIUM_HEADLESS; \
                 batch capture is not yet headless-aware (deferred to M11.4)"
            );
        }
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
    let mut cli = perf_trace::Cli::from_env_and_args();

    // ── M11.3: headless mode ownership ──────────────────────────────────
    // `CESIUM_HEADLESS` (default OFF) now drives a real surface-less
    // render → offscreen capture → PNG → clean exit via
    // `CesiumHeadlessPlugin` (added below). perf-trace's older M0.5 behaviour
    // for the same flag is a frame-cap *auto-exit*, which would fire at frame N
    // and kill the process before the async screenshot readback (spawned around
    // the same frame) has landed — so no PNG would ever be written. Hand exit
    // ownership to the capture plugin by clearing perf-trace's headless
    // auto-exit; camera-script playback (a separate flag) is unaffected.
    //
    // FIX-HL-EXIT (exit chain verified): `CesiumHeadlessPlugin` counts down
    // `headless_frames()` Update ticks, then `headless_capture_tick` spawns the
    // offscreen `Screenshot::image`, whose `save_to_disk` observer writes the PNG
    // synchronously and a second observer sends `AppExit::Success`
    // (`headless/mod.rs` L316-320). So clearing `cli.headless` here is safe — the
    // process still terminates and the artefact is still written; perf-trace's
    // `headless_exit_system` merely stands down so the two exits never race.
    let headless = feature_flags::headless_enabled();
    if headless {
        cli.headless = false;
    }

    let mut app = App::new();

    // M4.1: read lighting mode from env BEFORE plugin registration so
    // CesiumCorePlugin's Startup system sees the correct value.
    let lighting = lighting_mode();

    // ── M11.3: headless (surface-less) render mode ──────────────────────
    // When `headless`, swap the windowed WindowPlugin for a `primary_window:
    // None` / `DontExit` config so the app renders with no window, monitor or
    // display server. The `else` arm is byte-for-byte the pre-M11.3 windowed
    // config, so the default startup path is unchanged (golden-path neutral).
    if headless {
        // Surface-less: no primary window AND no winit event loop. With zero
        // windows the winit loop parks waiting for events, so `Update` would
        // never tick and the capture→AppExit path would never fire. Drive frames
        // with the continuous ScheduleRunner instead; the offscreen capture
        // plugin (added below) renders `headless_frames()` frames, writes the
        // PNG, then sends AppExit to terminate the runner.
        //
        // FIX-HL-EXIT: `Duration::ZERO` is deliberate — headless has no window to
        // present to and no vsync, so the runner ticks as fast as the CPU allows
        // and the only bound on runtime is the `headless_frames()` capture count.
        // An artificial throttle would only slow capture with nothing to gain.
        app.add_plugins(
            DefaultPlugins
                .set(cesium_bevy_render::headless::headless_window_plugin())
                .disable::<bevy::winit::WinitPlugin>()
                .add(bevy::app::ScheduleRunnerPlugin::run_loop(
                    std::time::Duration::ZERO,
                )),
        );
    } else {
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "CesiumRust - 3D Globe Viewer".into(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }));
    }

    app.insert_resource(ClearColor(Color::BLACK))
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
        // M5-C caveat: the dome shader emits *un-normalized* in-scattered
        // radiance (solar disc can exceed 1.0). That is only displayable when
        // the camera runs `Camera{hdr:true}` + `Tonemapping::AcesFitted`, which
        // M4.2 wires exclusively under CESIUM_ENABLE_POSTPROCESS_BUILTIN
        // (see docs/deviations.md#dev-010). With SKYDOME alone the values land
        // in an LDR framebuffer and clip, so gate ON can look *worse* than
        // gate OFF — warn instead of silently degrading.
        if !postprocess_builtin_enabled() {
            warn!(
                "[M5-C] SKYDOME 已开但 POSTPROCESS_BUILTIN 未开：散射将写入 LDR framebuffer、>1.0 被 clip，\
                 建议同开 CESIUM_ENABLE_POSTPROCESS_BUILTIN；AtmosphereGlow 已因互斥被强制关闭"
            );
        }
    }

    // ── M4.2 + M5-E1: post-process (two independent gates, coexist) ────────
    // CesiumEffectsPlugin is registered when EITHER gate is ON:
    //   • CESIUM_ENABLE_POSTPROCESS_BUILTIN → M4.2 fog clear-color system
    //     (tonemapping / bloom / HDR live on the camera bundle in orbit_camera.rs).
    //   • CESIUM_ENABLE_POSTPROCESS         → M5-E1 FXAA render-graph node
    //     (self-implemented WGSL, quality preset 12 only; see effects/fxaa.rs).
    // The plugin internally gates each feature by its own env var (see
    // CesiumEffectsPlugin::build), so these two POSTPROCESS gates never
    // cross-contaminate and there is no double-FXAA (Bevy's built-in FxaaPlugin
    // is not added; our node uses a distinct CesiumPostProcessLabel::Fxaa +
    // CesiumFxaa marker component).
    // SCOPE OF THAT CLAIM (corrected at M5 收口, Ultra Review Daniel M3):
    // independence holds for POSTPROCESS ↔ POSTPROCESS_BUILTIN only. It does
    // NOT hold across the M5 sky/post-process family:
    //   • SKYDOME ↔ GLOW are **mutually exclusive by construction** —
    //     glow_enabled() == !skydome_enabled() (feature_flags.rs), so SKYDOME=1
    //     is *not* purely additive: it forces AtmosphereGlowPlugin OFF
    //     (docs/deviations.md#dev-015).
    //   • SKYDOME **recommends** POSTPROCESS_BUILTIN=1 — the dome's
    //     un-normalized radiance needs HDR + AcesFitted tonemapping, else it
    //     clips in an LDR framebuffer (warned above; deferred.md #45).
    // Default (all OFF) → plugin not added → v0 baselines pixel-neutral (PSNR=∞).
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
    // baselines captured to specs/baselines/v2_water/ (4 PNG: 3 sea-state
    // close-ups + 1 arc overview). No existing plugin registration is altered.
    // The gate is read through the feature_flags registry (single source of
    // truth for every CESIUM_* env name) rather than a bare string literal.
    if material_showcase_enabled() {
        app.add_plugins(MaterialShowcasePlugin);
        info!("[M5-D] Material showcase: MaterialShowcasePlugin registered");
    }

    // ── M6 Wave A (task #81) + Phase-3 FIX-INTEG: Clipping + Panorama + IBL + OIT + Clouds ──
    // Five independent gates, all default OFF, all purely additive:
    //   • CESIUM_ENABLE_CLIPPING → M6.2 screen-space clipping node      (#77)
    //   • CESIUM_ENABLE_PANORAMA → M6.3 environment panorama node       (#78)
    //   • CESIUM_ENABLE_IBL      → M6.5 image-based lighting node       (#79)
    //   • CESIUM_ENABLE_OIT      → M6.4 weighted-blended OIT tail  (#67, phase-3)
    //   • CESIUM_ENABLE_CLOUDS   → M6.6 volumetric cloud composite (#63, phase-3)
    // `M6WaveARenderGraphPlugin` reads the same env names through the
    // adapter's byte-identical mirrors (DDD: `adapters/bevy-render` cannot import
    // this `application` crate) and adds **nodes and `Core3d` edges only for the
    // gates that are ON**. With every gate OFF nothing is registered, no edge is
    // touched and no component is inserted, so the v0 baselines stay bit-exact
    // (PSNR = ∞) — the golden path is not reachable from this block.
    //
    // It is a *plugin* rather than a plain function call because the render-world
    // half of the registration needs `RenderDevice`, which Bevy only inserts in
    // `RenderPlugin::finish`: the plugin's `build` does the main-world half
    // (shaders + `ExtractComponentPlugin` + prepass systems) and its `finish` the
    // render-world half (pipelines + `Core3d` nodes + edges). Calling it as a
    // plain function here panicked with "RenderDevice does not exist in the World"
    // (docs/deviations.md#dev-029).
    //
    // The graph entry point is the single owner of the chain *shape*: it removes
    // the bevy default `MainOpaquePass → MainTransmissivePass` edge before
    // splicing the panorama in (serial insertion, never a diamond — the defect
    // class Daniel H2 flagged and `insert_node_in_core3d` used to have), and it
    // removes `EndMainPass → Tonemapping` / `EndMainPass → PassThrough` before
    // prepending Clipping + IBL, so Robin #72's post-process reorder
    // (`PassThrough → AmbientOcclusion → Tonemapping → Fxaa`) survives
    // byte-for-byte.
    //
    // DEVIATION: IBL adds environment light to the HDR scene and Clipping
    //   overwrites the clipped region — both are *screen-space* passes, whereas
    //   upstream injects IBL per-material in the forward pass and clips with a
    //   per-geometry `discard`; see docs/deviations.md#dev-027
    let m6_clipping = feature_flags::clipping_enabled();
    let m6_panorama = feature_flags::panorama_enabled();
    let m6_ibl = feature_flags::ibl_enabled();
    let m6_oit = feature_flags::oit_enabled();
    let m6_clouds = feature_flags::clouds_enabled();
    let m6_split = feature_flags::split_enabled();
    if m6_clipping || m6_panorama || m6_ibl || m6_oit || m6_clouds || m6_split {
        app.add_plugins(M6WaveARenderGraphPlugin);
        app.insert_resource(M6WaveAConfig {
            clipping: m6_clipping,
            panorama: m6_panorama,
            ibl: m6_ibl,
            oit: m6_oit,
            clouds: m6_clouds,
            split: m6_split,
            done: false,
            sky_image: None,
        });
        // FIX-HL-RESMUT: create the (camera-independent) sky image once at
        // Startup, then let the per-frame setup attach components without ever
        // holding `Assets<Image>`.
        app.add_systems(Startup, m6_wave_a_prepare);
        app.add_systems(Update, m6_wave_a_setup);
        if m6_clipping {
            info!("[M6.2] ClippingPlanes: node + Core3d edges registered (screen-space)");
        }
        if m6_panorama {
            info!(
                "[M6.3] Panorama: node + Core3d edges registered \
                 (MainOpaquePass → CesiumPanorama → MainTransmissivePass)"
            );
        }
        if m6_ibl {
            info!("[M6.5] IBL: node + Core3d edges registered (HDR, before Tonemapping)");
        }
        if m6_oit {
            info!(
                "[M6.4] OIT: accumulate + composite registered \
                 (MainTransparentPass → Oit → OitComposite → EndMainPass)"
            );
        }
        if m6_clouds {
            info!("[M6.6] Clouds: composite registered (EndMainPass → Clouds → <successor>)");
        }
        if m6_split {
            info!("[M6.1] Split: divider registered (… → Clouds → Split → <successor>)");
        }
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

    // ── M11.3: headless offscreen capture → PNG → clean exit ────────────
    // Only active under CESIUM_HEADLESS (default OFF → plugin not added →
    // windowed path byte-for-byte untouched). Creates an offscreen RGBA8(sRGB)
    // target, retargets the scene Camera3d to it each frame, renders
    // `headless_frames()` warm-up frames, then captures one PNG (the artefact
    // `tools/pixel_diff` consumes for the pixel gate) and requests AppExit.
    // NOTE (deferred to M11.4): multi-view batch capture
    // (CESIUM_SCREENSHOT_SCRIPT) and AutoScreenshot still use
    // Screenshot::primary_window(), so they are not yet headless-aware — they now
    // emit a `warn!` (FIX-HL-EXIT) rather than failing silently; single-view
    // offscreen capture via this plugin is the supported headless path.
    if headless {
        let output = feature_flags::headless_output();
        let frames = feature_flags::headless_frames();
        info!(
            "[M11.3] CESIUM_HEADLESS: offscreen capture → {} after {} frames",
            output.display(),
            frames
        );
        app.add_plugins(cesium_bevy_render::headless::CesiumHeadlessPlugin::new(
            output, frames,
        ));
    }

    app.run();
}

// ─── M6 Wave A (task #81): camera component composition ─────────────────────

/// Which M6 components the one-shot setup system still has to attach.
///
/// `done` makes the system idempotent: re-inserting the components every frame
/// would re-extract a fresh copy each frame and churn the render-world bind
/// groups (`prepare_panorama_bind_groups` rebuilds them per frame *anyway*, but
/// only for entities that actually changed).
#[derive(Resource)]
struct M6WaveAConfig {
    clipping: bool,
    panorama: bool,
    ibl: bool,
    /// Phase-3 FIX-INTEG: M6.4 weighted-blended OIT gate.
    oit: bool,
    /// Phase-3 FIX-INTEG: M6.6 volumetric cloud composite gate.
    clouds: bool,
    /// Phase-3 FIX-SPLIT: M6.1 split-screen divider gate.
    split: bool,
    done: bool,
    /// FIX-HL-RESMUT: procedural sky-cube [`Image`] handle, built once by
    /// [`m6_wave_a_prepare`] in `Startup`. [`m6_wave_a_setup`] reads (clones) it
    /// to attach the panorama, so that per-frame system no longer needs a
    /// persistent `ResMut<Assets<Image>>` — which otherwise claimed the shared
    /// image asset store for the whole `Update` stage on every frame.
    sky_image: Option<Handle<Image>>,
}

/// FIX-HL-RESMUT: one-shot `Startup` system that builds the procedural sky-cube
/// image and stashes its handle on [`M6WaveAConfig`]. Asset creation does not
/// depend on the camera, so it belongs in `Startup` (runs exactly once, after
/// which the `Assets<Image>` borrow is released); the per-frame
/// [`m6_wave_a_setup`] then only consumes the pre-made handle.
fn m6_wave_a_prepare(mut cfg: ResMut<M6WaveAConfig>, mut images: ResMut<Assets<Image>>) {
    if cfg.panorama && cfg.sky_image.is_none() {
        cfg.sky_image = Some(images.add(m6_procedural_sky_cube()));
    }
}

/// Attaches the M6 components to the scene camera, once.
///
/// **Why a one-shot `Update` system and not `Startup`:** the camera entity is
/// spawned by `orbit_camera::spawn_orbit_camera`, which is *private* to that
/// module and registered in `Startup`, so `main.rs` cannot order a system
/// `.after(...)` it. Waiting for the first `Update` in which a `Camera3d` exists
/// is the ordering-free equivalent, and it keeps `orbit_camera.rs` untouched
/// (M2/M5 red line: the camera bundle is that module's territory).
///
/// The three nodes are driven by per-view components, exactly like M5's
/// `CesiumFxaa` / `CesiumAmbientOcclusion`. The prepass components they need
/// (`DepthPrepass` for clipping, `DepthPrepass + NormalPrepass` for IBL) are
/// attached by the adapters' own `setup_*_prepass` systems, registered inside
/// `register_clipping_planes_node` / `register_ibl_node` — so this system only
/// ever inserts the *component that carries the domain value objects*.
fn m6_wave_a_setup(
    mut commands: Commands,
    mut cfg: ResMut<M6WaveAConfig>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    if cfg.done {
        return;
    }
    // FIX-HL-RESMUT: pick the camera deterministically. `Query::iter().next()`
    // follows archetype/chunk iteration order, which is not a stable notion of
    // "the main camera" once more than one `Camera3d` exists. `Entity` is `Ord`
    // by (index, generation), so `.min()` always resolves the first-spawned
    // camera (`orbit_camera::spawn_orbit_camera` runs before any secondary view).
    // A dedicated main-camera marker would be stronger, but `orbit_camera.rs` is
    // outside this change's file scope, so deterministic ordering is used here.
    let Some(camera) = cameras.iter().min() else {
        // Camera not spawned yet — stay `!done` and retry on the next frame.
        return;
    };

    if cfg.clipping {
        commands.entity(camera).insert(m6_demo_clipping_planes());
    }
    // Phase-3 FIX-INTEG: OIT is a whole-camera marker — the accumulate/composite
    // nodes early-return when `enabled == false`, so an active component plus the
    // gate being ON is all that turns the transparent-tail pass on. (Faithful
    // re-routing of Bevy's transparent geometry into the MRT targets is deferred;
    // see docs/deviations.md#dev-031.)
    if cfg.oit {
        commands.entity(camera).insert(CesiumOit::default());
    }
    if cfg.clouds {
        commands
            .entity(camera)
            .insert(CesiumClouds::new(m6_demo_cloud_collection()));
    }
    // Phase-3 FIX-SPLIT: the split-screen divider is a whole-camera marker too —
    // the `SplitNode` early-returns when `enabled == false`, so inserting the
    // component plus the gate being ON turns the overlay on. `new(0.5)` centres
    // the divider at the horizontal mid-point (a `SplitConfig` drag then moves it).
    if cfg.split {
        commands.entity(camera).insert(CesiumSplit::new(0.5));
    }
    if cfg.ibl {
        // Domain defaults: `image_based_lighting_factor = [1.0, 1.0]` (so
        // `CesiumIbl::is_active()` is true) and no explicit SH coefficients, which
        // `IblUniform::from_domain` resolves through `default_spherical_harmonics()`.
        commands
            .entity(camera)
            .insert(CesiumIbl::new(
                ImageBasedLighting::default(),
                IblMaterial::default(),
            ));
    }
    if cfg.panorama {
        // FIX-HL-RESMUT: use the handle pre-created in `m6_wave_a_prepare` so this
        // per-frame system never touches `Assets<Image>`. A `None` means prepare did
        // not run — warn and skip rather than resurrect the persistent borrow.
        match cfg.sky_image.clone() {
            Some(image) => {
                let faces: [String; 6] = CubeMapPanorama::FACE_NAMES
                    .map(|name| format!("procedural://m6-sky-cube/{name}"));
                let panorama = CubeMapPanorama::new(faces);
                debug_assert!(
                    panorama.is_complete(),
                    "all six cube faces must be named or `CubeMapPanorama` is incomplete"
                );
                commands
                    .entity(camera)
                    .insert(CesiumPanorama::from_domain_cubemap(&panorama, image, 1.0));
            }
            None => {
                warn!(
                    "[M6] panorama requested but sky image handle missing \
                     (m6_wave_a_prepare did not run) — skipping panorama attach"
                );
            }
        }
    }

    cfg.done = true;
}

/// A two-plane globe cut, the classic CesiumJS `ClippingPlaneCollection` demo.
///
/// Intersection mode (`union_clipping_regions == false`, the domain default)
/// clips a fragment only when it is outside **every** plane, so these two planes
/// remove the `x < 0 && z < 0` quarter of the globe and leave the other three
/// quadrants intact. Distances are in **metres** — the adapter divides by
/// `METERS_PER_RENDER_UNIT = 6378137` when packing the uniform — and `0.0` puts
/// both planes through the globe centre.
///
/// `edge_width` is in **pixels**, not metres: `shaders/clipping.wgsl` multiplies
/// it by `fwidth()` (the stand-in for upstream's `czm_metersPerPixel`, see
/// docs/deviations.md#dev-023).
fn m6_demo_clipping_planes() -> CesiumClippingPlanes {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        // `bevy::math::DVec3` (f64) is not in `bevy::prelude`, which only exports
        // the f32 vector types — spelled out so the domain stays f64 end to end.
        ClippingPlane::new(bevy::math::DVec3::new(0.0, 0.0, 1.0), 0.0),
        ClippingPlane::new(bevy::math::DVec3::new(1.0, 0.0, 0.0), 0.0),
    ]);
    collection.edge_width = 2.0; // PIXELS
    collection.edge_color = [1.0, 1.0, 1.0, 1.0];
    CesiumClippingPlanes::new(collection)
}

/// A small deterministic cloud field for the M6.6 gate-on demo.
///
/// Positions and maximum sizes are in **metres** (metric f64 domain; the adapter
/// narrows to render units at the GPU boundary). This only ever reaches the GPU
/// when `CESIUM_ENABLE_CLOUDS=1` (default OFF → the node is never registered and
/// the v0 baselines stay bit-exact); the composite `is_active()` guard additionally
/// early-returns while the collection is empty or hidden. The faithful per-cloud
/// billboard + 3D-noise path is deferred to a real-GPU task
/// (`docs/deviations.md#dev-032`); this field exercises the screen-space
/// ray-march composite end to end.
fn m6_demo_cloud_collection() -> CloudCollection {
    use bevy::math::DVec3;
    let mut collection = CloudCollection::new();
    // A shallow arc of cumulus cloudlets, each a ~2 km × 1 km ellipsoid, offset
    // along ±x and lifted to a low altitude so the orbit camera frames them.
    for i in -2..=2 {
        let x = f64::from(i) * 2500.0;
        let position = DVec3::new(x, 0.0, 3000.0);
        let maximum_size = DVec3::new(2000.0, 1200.0, 800.0);
        collection.add(CumulusCloud::new(position, maximum_size));
    }
    collection
}

/// A procedurally generated six-face cube map standing in for a real sky asset.
///
/// **Why procedural, and why `CubeMapPanorama` rather than the upstream default
/// `EquirectangularPanorama`** (docs/deviations.md#dev-028, deferred.md #58):
/// upstream's equirectangular panorama is a *finite bubble* whose default radius
/// is `DEFAULT_PANORAMA_RADIUS = 100_000 m = 0.0157` render units — about 1.6 %
/// of the globe radius. `orbit_camera` never comes closer than `1.005` render
/// units, so that bubble sits entirely inside the globe and the opaque pass
/// hides it: gate ON would be indistinguishable from gate OFF. The cube-map
/// placement (`PanoramaPlacement::Skybox`) is camera-centred and infinite,
/// writes no depth, and is therefore the placement that makes the documented
/// `MainOpaquePass → Panorama → MainTransparentPass(starfield r=50 → sky dome
/// r=40)` order coherent — the panorama fills the sky, the transparent draws
/// still pass the depth test on top of it, and the sky dome's three ordering
/// mechanisms (`depth_bias` / `Premultiplied` / `cull Front`) stay untouched.
///
/// Shape requirements are those `prepare_panorama_bind_groups` enforces: a
/// `texture_cube` slot needs exactly **six array layers**, so the image is built
/// as a 2D array texture with `depth_or_array_layers = 6` and a
/// `TextureViewDescriptor` whose `dimension` is `Cube` (bevy 0.15.3 honours
/// `Image::texture_view_descriptor` verbatim in `GpuImage::prepare_asset`).
///
/// Colours are **sRGB bytes** because the format is `Rgba8UnormSrgb`: the
/// hardware performs the sRGB → linear decode that upstream's `czm_gammaCorrect`
/// did by hand (project sRGB red line, docs/deviations.md#dev-025).
fn m6_procedural_sky_cube() -> Image {
    /// Per-face edge length. Small on purpose: linear filtering across a smooth
    /// vertical gradient is all the placeholder has to do.
    const FACE: u32 = 8;
    const ZENITH: [u8; 3] = [38, 78, 160];
    const HORIZON: [u8; 3] = [126, 148, 176];
    const NADIR: [u8; 3] = [12, 14, 22];

    let mut data = Vec::with_capacity(6 * (FACE * FACE * 4) as usize);
    // Face order is upstream's `[+X, -X, +Y, -Y, +Z, -Z]`
    // (`CubeMapPanorama::FACE_NAMES` ≡ `SkyBox.js::createEarthSkyBox`).
    for face in 0..6u32 {
        for y in 0..FACE {
            // `t = 0` at the zenith edge of the face, `1` at the nadir edge. In a
            // cube map the four side faces run `+Y` (row 0) → `-Y` (last row), so a
            // plain row index is already the gradient axis; `+Y` (face 2) is the
            // zenith cap and `-Y` (face 3) the nadir cap.
            let t = match face {
                2 => 0.0_f32,
                3 => 1.0_f32,
                _ => y as f32 / (FACE - 1) as f32,
            };
            let rgb = if t < 0.5 {
                m6_lerp_u8(ZENITH, HORIZON, t * 2.0)
            } else {
                m6_lerp_u8(HORIZON, NADIR, (t - 0.5) * 2.0)
            };
            let pixel = [rgb[0], rgb[1], rgb[2], 255];
            for _ in 0..FACE {
                data.extend_from_slice(&pixel);
            }
        }
    }

    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: FACE,
            height: FACE,
            depth_or_array_layers: 6,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    image.texture_view_descriptor = Some(bevy::render::render_resource::TextureViewDescriptor {
        dimension: Some(bevy::render::render_resource::TextureViewDimension::Cube),
        ..Default::default()
    });
    image
}

/// Component-wise byte lerp for the placeholder sky gradient.
///
/// `t` is always in `[0, 1]` at both call sites, so the result cannot leave the
/// `u8` range and the `as u8` narrowing is exact.
fn m6_lerp_u8(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let mix = |lo: u8, hi: u8| -> u8 {
        let lo = f32::from(lo);
        let hi = f32::from(hi);
        (lo + (hi - lo) * t).round() as u8
    };
    [mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2])]
}

#[cfg(test)]
mod m6_setup_tests {
    use super::*;

    /// FIX-HL-RESMUT: with two `Camera3d` entities the one-shot setup must attach
    /// the M6 component to the *deterministically selected* camera — the same one
    /// `cameras.iter().min()` resolves — not whichever `Query::iter()` happens to
    /// yield first. A clipping-only config keeps the fixture free of
    /// `Assets<Image>` / render-world dependencies.
    #[test]
    fn m6_wave_a_setup_attaches_to_the_deterministic_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let cam_a = app.world_mut().spawn(Camera3d::default()).id();
        let cam_b = app.world_mut().spawn(Camera3d::default()).id();
        // Mirror the system's selection rule (lowest `Entity` by `Ord`).
        let expected = cam_a.min(cam_b);
        let other = if expected == cam_a { cam_b } else { cam_a };

        app.insert_resource(M6WaveAConfig {
            clipping: true,
            panorama: false,
            ibl: false,
            oit: false,
            clouds: false,
            split: false,
            done: false,
            sky_image: None,
        });
        app.add_systems(Update, m6_wave_a_setup);
        app.update();

        let has = |e: Entity| app.world().get::<CesiumClippingPlanes>(e).is_some();
        assert!(
            app.world().resource::<M6WaveAConfig>().done,
            "setup must complete once a camera exists"
        );
        assert!(has(expected), "component must land on the min-id camera");
        assert!(!has(other), "component must not touch the other camera");
    }
}
