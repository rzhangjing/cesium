//! cesium-app：CesiumRust 3D 地球查看器
//!
//! 交互式 3D 地球，包含：
//! - 基础球体 + 极地盖片（无 LOD 的兜底安全网）
//! - 带 Bing Maps 卫星影像的动态 LOD 瓦片
//! - Orbit 相机（鼠标拖拽旋转，滚轮缩放）
//! - 大气边缘辉光 + 星空背景

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
mod feature_flags;
mod perf_counters;
mod perf_trace;
mod capture_script;
mod offline_check;
mod map2d;
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
// ── M6 Wave A（任务 #81）───────────────────────────────────────────────────
// 三个 M6 相机组件、拥有其 `Core3d` 边的渲染图入口点，以及这些组件
// 所构建自的*领域*值对象。领域类型通过 `adapters/bevy-render/src/effects/mod.rs`
// 中的重新导出抵达 `cesium-app`（与 `LightingMode` 已在使用的模式相同）：
// `cesium-app` 刻意不直接依赖 `cesium-effects` 领域 crate，因此适配器
// 仍是唯一将领域 f64 收窄为 GPU f32 的层。
use cesium_bevy_render::effects::{
    CesiumClippingPlanes, CesiumClouds, CesiumIbl, CesiumOit, CesiumPanorama, CesiumSplit,
    ClippingPlane, ClippingPlaneCollection, CloudCollection, CubeMapPanorama, CumulusCloud,
    IblMaterial, ImageBasedLighting, M6WaveARenderGraphPlugin,
};

const TILE_SEGMENTS: u32 = 16;

// ── Cesium Ion 地形配置（异步，脱离主线程）────────────────────────
/// 通过 ion API 解析 Cesium World Terrain 端点并返回 `{z}/{x}/{y}`
/// 模板 URL；当 token 缺失 / 调用失败时返回 `None`（优雅降级为一个
/// 空闲的地形链）。
///
/// 运行在 [`IoTaskPool`] worker 上（在此使用阻塞式 ureq 没问题）；它绝
/// 不能运行在 Startup/帧线程上，否则首帧会卡顿。
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

/// 正在进行的 ion 端点解析；一旦解析完成即被移除。
#[derive(Resource)]
struct PendingTerrainEndpoint {
    task: Task<Option<String>>,
}

/// Startup：在 IoTaskPool 上 spawn ion 端点解析。非阻塞。
fn spawn_terrain_config_task(mut commands: Commands) {
    let pool = IoTaskPool::get();
    let task = pool.spawn(async move { resolve_terrain_endpoint() });
    commands.insert_resource(PendingTerrainEndpoint { task });
}

/// Update：每帧轮询端点任务一次；就绪后回填
/// `GlobeConfig.terrain_provider_url` 并丢弃 pending 资源。
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

/// 诊断：每 5 秒记录一次 TileLoadStats。
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

/// Startup：spawn 一个 3D Tiles 瓦片集根。
///
/// 默认是从 GitHub raw 直接供送的 Cesium 示例瓦片集（未压缩 b3dm）——
/// 一个公开、免 token 的在线源。离线运行时可用
/// `CESIUM_TILESET_URL`（例如本地 HTTP 服务器）覆盖。
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

/// 无头验证辅助工具，除非设置了 `CESIUM_SCREENSHOT_AT_FRAME` 否则不生效：
/// 在该帧捕获一张截图并在紧随其后退出。让审阅者无需改变默认
/// 交互式运行就能抓取一个确定性帧（默认或 opt-in 链）。
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
        // FIX-HL-EXIT：在 CESIUM_HEADLESS 下没有主窗口，因此
        // `Screenshot::primary_window()` 不生效（不会触发任何 capture observer）。使
        // 那个静默的空缺变得响亮；受支持的无头产物是 M11.3
        // 离屏 `CesiumHeadlessPlugin` 捕获路径。
        if feature_flags::headless_enabled() {
            warn!(
                "[shot] AutoScreenshot uses Screenshot::primary_window(), inert under \
                 CESIUM_HEADLESS; rely on the headless offscreen capture for the PNG"
            );
        }
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.clone()));
        info!("[shot] capturing at frame {}", shot.at_frame);

        // M3：在截图旁写入相机 pos/quat JSON。
        // Schema: {"pos":[x,y,z],"quat":[x,y,z,w],"fov_y":<deg>}
        // 这是 M2.4 回放保真度的参考基准（<1e-4 render unit）。
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
            // 从截图路径推导 .camera.json 路径。
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

// ── M3.3：FIXED_CAMERA —— 确定性位姿覆盖 ──────────────────

/// `FIXED_CAMERA` 文件的 TOML schema：单个 `[camera]` 表。
#[derive(Debug, Deserialize)]
struct FixedCameraFile {
    camera: capture_script::CameraPose,
}

/// 保存一个每帧 PostUpdate 都应用的冻结相机位姿的资源。
#[derive(Resource)]
struct FixedCameraPose {
    pos: Vec3,
    quat: Quat,
    fov_y: Option<f32>,
}

/// PostUpdate 系统：用固定位姿覆盖 orbit 相机。
/// 在 orbit_camera 的 Update 之后运行，因此确定性位姿胜出。
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

// ── M3.4：批量截图捕获 ─────────────────────────

/// 驱动批量多视图捕获（`CESIUM_SCREENSHOT_SCRIPT`）的资源。
#[derive(Resource)]
struct BatchCapture {
    script: Vec<capture_script::ShotEntry>,
    cursor: usize,
    frame: u32,
    output_dir: String,
    git_sha: String,
    env_snapshot: serde_json::Value,
}

/// PostUpdate 系统：推进批量捕获调度器。应用到期
/// shot 的相机位姿，spawn 一个 `Screenshot`，写入 `.camera.json` +
/// `.meta.json`，并在所有 shot + 30 帧缓冲帧之后退出。
fn batch_capture_system(
    mut commands: Commands,
    mut batch: ResMut<BatchCapture>,
    mut exit: EventWriter<AppExit>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera>>,
) {
    batch.frame += 1;

    if let Some(idx) = capture_script::next_due(&batch.script, batch.cursor, batch.frame) {
        let entry = batch.script[idx].clone();

        // 应用脚本化的相机位姿（本帧覆盖 orbit_camera）。
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

        // spawn 截图。
        let png_path = format!("{}/{}.png", batch.output_dir, entry.name);
        // FIX-HL-EXIT：批量捕获同样尚未无头感知（参见插件布线处的
        // M11.4 NOTE）；宁可告警也不要静默失败。
        if feature_flags::headless_enabled() {
            warn!(
                "[batch] Screenshot::primary_window() is inert under CESIUM_HEADLESS; \
                 batch capture is not yet headless-aware (deferred to M11.4)"
            );
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(png_path));

        // 写入 .camera.json（与单 shot harness 同 schema）。
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

        // 写入 .meta.json（供 pixel_diff 门控的 frame/env/git_sha）。
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

    // 在所有 shot 发射后 + 30 帧缓冲帧供 GPU flush 后退出。
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

/// 为批量捕获元数据构建 env 快照 JSON。
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

/// spawn 基础球体与极地盖片的插件。
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

    // 基础球体 —— 高细分 UV 球体（Mercator 映射 V，因此运行时
    // 全球 composite 纹理能像瓦片层一样垂覆）；略小以保持
    // 在瓦片与极地盖片之下。纯色只是 composite 前的兜底：一旦基础
    // 瓦片层到达，dynamic_globe 就在其上垂覆一个模糊的地球 composite，
    // 使瞬时的空缺读作地球而非一片扁平的蓝色虚空。
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

    // 极地盖片 —— 北盖使用与受光海洋颜色匹配的钢蓝色，因此
    // 北极点无缝地延续周围的海面（参考：CesiumJS 全球观感）；
    // 南盖为冰白色，因为环绕南极的 85° 瓦片环是白色冰原，
    // 盖片必须延续它。
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
    // M3.3：无头离线自检 —— 无需 GPU 即可证明 fetcher 布线。
    // 立即退出（无 Bevy app、无窗口、无需 GPU 上下文）。
    if feature_flags::offline_selfcheck() {
        std::process::exit(offline_check::run());
    }

    // M0.3：功能门控现位于 `feature_flags` 中；总括的
    // `CESIUM_ENABLE_NEW_CHAINS` 与每链标志完全保留其 M0.3 前的
    // 语义（默认 OFF，truthy token 不变）。
    let enable_terrain = terrain_enabled();
    let enable_tileset = tileset_enabled();

    // M0.4：PerfCounters 始终存在（廉价；dynamic_globe 每帧
    // 写入其中）。M0.5 perf-trace 插件读取它。
    let perf_counters = PerfCounters::default();

    // M0.5：用于无头 + trace + camera-script 的 CLI/env 解析。
    let mut cli = perf_trace::Cli::from_env_and_args();

    // ── M11.3：无头模式所有权 ──────────────────────────
    // `CESIUM_HEADLESS`（默认 OFF）现在驱动一次真正的无表面
    // 渲染 → 离屏捕获 → PNG → 通过
    // `CesiumHeadlessPlugin`（下方新增）干净退出。perf-trace 旧的 M0.5
    // 对于同一标志的行为是一个帧数上限的 *auto-exit*，它会在第 N 帧触发
    // 并在异步截图回读（约在同一帧 spawn）落地前杀掉进程 —— 因此
    // PNG 永不会被写入。通过清除 perf-trace 的无头 auto-exit
    // 将退出所有权交给捕获插件；camera-script 回放（一个独立的标志）
    // 不受影响。
    //
    // FIX-HL-EXIT（退出链已验证）：`CesiumHeadlessPlugin` 递减
    // `headless_frames()` Update tick，然后 `headless_capture_tick` spawn
    // 离屏的 `Screenshot::image`，其 `save_to_disk` observer 同步写入 PNG
    // 且第二个 observer 发送 `AppExit::Success`
    // （`headless/mod.rs` L316-320）。因此在此清除 `cli.headless` 是安全的 ——
    // 进程仍会终止且产物仍会被写入；perf-trace 的
    // `headless_exit_system` 仅退让，以便两个退出永不竞态。
    let headless = feature_flags::headless_enabled();
    if headless {
        cli.headless = false;
    }

    let mut app = App::new();

    // M4.1：在插件注册*之前*从 env 读取 lighting 模式，以便
    // CesiumCorePlugin 的 Startup 系统看到正确的值。
    let lighting = lighting_mode();

    // ── M11.3：无头（无表面）渲染模式 ──────────────────
    // 当 `headless` 时，将窗口化的 WindowPlugin 换为一个 `primary_window:
    // None` / `DontExit` 配置，因此 app 渲染时无窗口、无监视器也无
    // 显示服务器。`else` 分支逐字节就是 M11.3 前的窗口化
    // 配置，因此默认启动路径不变（黄金路径中性）。
    if headless {
        // 无表面：既无主窗口也无 winit 事件循环。零窗口时
        // winit 循环会停车等待事件，因此 `Update` 永不会 tick 且捕获→AppExit 路径
        // 永不会触发。改用连续的 ScheduleRunner 驱动帧；离屏捕获
        // 插件（下方新增）渲染 `headless_frames()` 帧，写入
        // PNG，然后发送 AppExit 以终止 runner。
        //
        // FIX-HL-EXIT：`Duration::ZERO` 是有意为之 —— 无头没有可呈现的窗口也
        // 没有 vsync，因此 runner 以 CPU 允许的最快速度 tick，
        // 运行时长的唯一约束是 `headless_frames()` 捕获计数。
        // 人为限流只会拖慢捕获而毫无收益。
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
        // M4.1：在 CesiumCorePlugin 之前插入 lighting 模式，以便 setup_lighting
        // 读取它（否则 init_resource 会用 Default=FullAmbient）。
        .insert_resource(lighting)
        // Core：lighting + 地球配置 + AnimationClock
        .add_plugins(CesiumCorePlugin)
        // Camera：鼠标 orbit/缩放
        .add_plugins(OrbitCameraPlugin)
        // 地球渲染
        .add_plugins(BaseSpherePlugin);

    // 地球渲染。P1-1/建议1 (2026-09-27)：黄金路径是 M1.5
    // 薄壳。冻结的遗留单体（`dynamic_globe_legacy`，旧的
    // CESIUMRST_LEGACY_DYNAMIC_GLOBE A/B 分支）在 G4 证明它与薄壳像素中性
    // 之后已被退役（参见
    // PIPELINE_PROMOTION_PLAN.md 与 verification_evidence/g4/）。
    app.add_plugins(dynamic_globe::DynamicGlobePlugin);

    // ── M5-B：AtmosphereGlow ↔ SkyDome 互斥 ────────────────
    // 8-shell 辉光回退与程序化 sky dome 渲染相互重叠的
    // 大气边缘效果。glow_enabled() 返回 !skydome_enabled() 因此
    // 它们永不同时激活。默认（SKYDOME OFF）→ glow ON → v0 零差异。
    // 安全：AtmosphereGlowPlugin 的 fade 系统读取 Res<OrbitState>，后者
    // 因 OrbitCameraPlugin 在上方 L476 无条件注册而保证存在
    // （与 #47 terrain 插件独立性同一模式）。
    if glow_enabled() {
        app.add_plugins(AtmosphereGlowPlugin);
    }
    app.add_plugins(StarfieldPlugin);

    // ── 2D 平面图模式 + 切换按钮（仅窗口化）─────────────────
    // 在 CESIUM_HEADLESS 下永不注册，因此确定性的离屏
    // 捕获路径保持单相机、无 UI、无额外 render pass（v0 /
    // FIXED_CAMERA / camera-script 基线保持字节精确）。在默认
    // 窗口化会话中 MapMode 以 ThreeD 开始，因此 3D orbit 路径也
    // 不变地运行直到用户点击按钮。
    if !headless {
        app.add_plugins(map2d::Map2dPlugin);
    }

    // ── cesium-plot overlay 桥接（仅窗口化，M0 脚手架）──────────
    // 在 CESIUM_HEADLESS 下永不注册，并由 `plot_enabled()` 门控
    // （未设时默认 ON，用 CESIUM_ENABLE_PLOT=0 退出）。M0 只添加
    // 共享的桥接资源（PlotViewCtx / PlotInputCapture）—— 无系统、
    // 实体或 render-layer 内容 —— 因此 3D 黄金路径与离屏
    // 基线保持字节精确。后续里程碑在 render layer 3 上填充视图同步 / 拾取 /
    // 交互。
    if !headless && feature_flags::plot_enabled() {
        app.add_plugins(cesium_plot_bevy::CesiumPlotBridgePlugin);
    }

    // ── M4.1：shadow 插件（仅 day_night lighting）───────────────────
    if lighting == LightingMode::DayNight {
        app.add_plugins(CesiumShadowPlugin);
        info!("[M4.1] DayNight lighting: shadow plugin registered");
    }

    // ── M5-B：程序化 sky dome —— 独立的 SKYDOME 门控 ────────────
    // CesiumAtmospherePlugin 由 SKYDOME=1 激活，而非由 DayNight。
    // 将天空渲染与 lighting 模式解耦，让 M5-C 扩展
    // sky_system.rs 而无需碰 main.rs。默认 OFF → v0 零差异。
    // DEVIATION：sky dome 独立于 DayNight 门控，与 AtmosphereGlowPlugin
    //   互斥；参见 docs/deviations.md#dev-015
    if skydome_enabled() {
        app.add_plugins(CesiumAtmospherePlugin);
        info!("[M5-B] SkyDome: CesiumAtmospherePlugin registered");
        // M5-C 注意事项：dome shader 发射*未归一化*的在散射
        // radiance（太阳盘可超过 1.0）。那只有在相机运行
        // `Camera{hdr:true}` + `Tonemapping::AcesFitted` 时才可显示，而这由
        // M4.2 专在 CESIUM_ENABLE_POSTPROCESS_BUILTIN 下布线
        // （参见 docs/deviations.md#dev-010）。仅开 SKYDOME 时那些值会落入
        // LDR framebuffer 并被 clip，因此门控 ON 可能看起来比
        // 门控 OFF *更差* —— 宁可告警也不要静默降级。
        if !postprocess_builtin_enabled() {
            warn!(
                "[M5-C] SKYDOME 已开但 POSTPROCESS_BUILTIN 未开：散射将写入 LDR framebuffer、>1.0 被 clip，\
                 建议同开 CESIUM_ENABLE_POSTPROCESS_BUILTIN；AtmosphereGlow 已因互斥被强制关闭"
            );
        }
    }

    // ── M4.2 + M5-E1：后处理（两个独立门控，共存）───────────
    // 当任一个门控 ON 时都注册 CesiumEffectsPlugin：
    //   • CESIUM_ENABLE_POSTPROCESS_BUILTIN → M4.2 雾 clear-color 系统
    //     （tonemapping / bloom / HDR 位于 orbit_camera.rs 的相机 bundle 上）。
    //   • CESIUM_ENABLE_POSTPROCESS         → M5-E1 FXAA render-graph 节点
    //     （自实现 WGSL，仅 quality preset 12；参见 effects/fxaa.rs）。
    // 插件内部按各自的 env 变量门控每个功能（参见
    // CesiumEffectsPlugin::build），因此这两个 POSTPROCESS 门控永不
    // 交叉污染且没有双重 FXAA（Bevy 内置的 FxaaPlugin
    // 未被添加；我们的节点使用一个独立的 CesiumPostProcessLabel::Fxaa +
    // CesiumFxaa 标记组件）。
    // 该声明的适用范围（在 M5 收口时修正，Ultra Review Daniel M3）：
    // 独立性仅对 POSTPROCESS ↔ POSTPROCESS_BUILTIN 成立。它在
    // M5 sky/后处理家族之间并*不*成立：
    //   • SKYDOME ↔ GLOW **按构造互斥** ——
    //     glow_enabled() == !skydome_enabled()（feature_flags.rs），因此 SKYDOME=1
    //     *不是*纯附加的：它强制 AtmosphereGlowPlugin OFF
    //     （docs/deviations.md#dev-015）。
    //   • SKYDOME **推荐** POSTPROCESS_BUILTIN=1 —— dome 的
    //     未归一化 radiance 需要 HDR + AcesFitted tonemapping，否则它
    //     会在 LDR framebuffer 中被 clip（上方已告警；deferred.md #45）。
    // 默认（全 OFF）→ 插件未添加 → v0 基线像素中性（PSNR=∞）。
    if postprocess_builtin_enabled() || postprocess_enabled() {
        app.add_plugins(CesiumEffectsPlugin);
        if postprocess_builtin_enabled() {
            info!("[M4.2] Built-in post-process: CesiumEffectsPlugin registered");
        }
        if postprocess_enabled() {
            info!("[M5-E1] FXAA post-process (preset 12): CesiumEffectsPlugin registered");
        }
    }

    // ── M5-D：Fabric 材质展示（内置 + 3 种 Water 海况）──────
    // 纯附加，env 门控（默认 OFF → v0 基线像素中性）。
    // 通过 CESIUM_ENABLE_MATERIAL_SHOWCASE=1 opt-in 渲染材质球
    // —— 包括忠实移植的 Water.glsl（case 17u）calm/medium/rough
    // 基线，捕获到 specs/baselines/v2_water/（4 PNG：3 张海况
    // 特写 + 1 张弧形总览）。现有插件注册未被改动。
    // 该门控通过 feature_flags 注册表读取（每个 CESIUM_* env
    // 名的单一真相源）而非一个裸字符串字面量。
    if material_showcase_enabled() {
        app.add_plugins(MaterialShowcasePlugin);
        info!("[M5-D] Material showcase: MaterialShowcasePlugin registered");
    }

    // ── M6 Wave A（任务 #81）+ Phase-3 FIX-INTEG：Clipping + Panorama + IBL + OIT + Clouds ──
    // 五个独立门控，均默认 OFF，均为纯附加：
    //   • CESIUM_ENABLE_CLIPPING → M6.2 屏幕空间 clipping 节点      (#77)
    //   • CESIUM_ENABLE_PANORAMA → M6.3 环境 panorama 节点       (#78)
    //   • CESIUM_ENABLE_IBL      → M6.5 基于图像的 lighting 节点       (#79)
    //   • CESIUM_ENABLE_OIT      → M6.4 加权混合 OIT 尾链  (#67, phase-3)
    //   • CESIUM_ENABLE_CLOUDS   → M6.6 体积云 composite (#63, phase-3)
    // `M6WaveARenderGraphPlugin` 通过适配器的字节相同镜像读取
    // 相同的 env 名（DDD：`adapters/bevy-render` 不能导入
    // 这个 `application` crate），并**仅为处于 ON 的门控添加
    // 节点与 `Core3d` 边**。当每个门控都 OFF 时什么都不注册、
    // 不碰任何边也不插入任何组件，因此 v0 基线保持位精确
    // （PSNR = ∞）—— 从本块无法触及黄金路径。
    //
    // 它是一个*插件*而非普通函数调用，因为注册的 render-world
    // 那一半需要 `RenderDevice`，而 Bevy 只在 `RenderPlugin::finish` 中插入它：
    // 插件的 `build` 做 main-world 那一半
    // （shaders + `ExtractComponentPlugin` + prepass 系统）而其 `finish` 做
    // render-world 那一半（pipelines + `Core3d` 节点 + 边）。在此将其作为普通
    // 函数调用会以 "RenderDevice does not exist in the World" panic
    // （docs/deviations.md#dev-029）。
    //
    // 图的入口点是链 *形状*的唯一拥有者：它在拼入 panorama
    // 之前移除 bevy 默认的 `MainOpaquePass → MainTransmissivePass` 边（串行插入，
    // 绝无 diamond —— Daniel H2 标记且 `insert_node_in_core3d`
    // 曾有的缺陷类），并在前置 Clipping + IBL 之前移除
    // `EndMainPass → Tonemapping` / `EndMainPass → PassThrough`，因此 Robin #72 的
    // 后处理重排序（`PassThrough → AmbientOcclusion → Tonemapping → Fxaa`）
    // 逐字节存活。
    //
    // DEVIATION：IBL 向 HDR 场景添加环境光，Clipping
    //   覆写被裁剪的区域——两者都是*屏幕空间* pass，而
    //   上游在 forward pass 中逐材质注入 IBL 并用一个
    //   逐几何的 `discard` 裁剪；参见 docs/deviations.md#dev-027
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
        // FIX-HL-RESMUT：在 Startup 时创建一次（与相机无关的）sky image，
        // 然后让每帧的 setup 附加组件而永不持有 `Assets<Image>`。
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

    // M0.4：注册 PerfCounters 资源（dynamic_globe 写入，trace 读取）。
    app.insert_resource(perf_counters);

    // ── M2.5：CameraControl 驱动端口 ──────────────────────────
    // 程序化的相机控制（set_view / fly_to / look_at / zoom / home），
    // 委派给领域相机算法。作为一个资源注册，因此
    // 脚本/系统可以获取 `CameraControlPort` 并驱动相机；PostUpdate 桥接
    // 将它同步到任意 `CesiumCamera` 实体，且当不存在时不生效
    // （交互式默认相机仍由 orbit_camera 驱动，
    // 像素中性）。
    app.init_resource::<CameraControlPort>()
        .add_systems(PostUpdate, camera_control_port_system);

    // M0.5：perf-trace 插件（CSV writer + 可选 camera-script 回放
    // + 可选无头 auto-exit）。未给 trace 路径时不生效。
    app.add_plugins(PerfTracePlugin::new(cli));

    if enable_terrain {
        app.add_plugins(CesiumTerrainPlugin)
            // M4.3：imagery 管线为地形垂覆提供纹理
            // （地形瓦片上的 base_color_texture）。与 terrain 一同注册，
            // 以便在 render_system 运行前 ImageryCache 已填充。
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

    // 用于无头验证的可选确定性截图（默认不生效）。
    if let Some(at_frame) = std::env::var("CESIUM_SCREENSHOT_AT_FRAME")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    {
        let path = std::env::var("CESIUM_SCREENSHOT").unwrap_or_else(|_| "screenshot.png".into());
        app.insert_resource(AutoScreenshot { frame: 0, at_frame, path });
        app.add_systems(Update, auto_screenshot_system);
    }

    // ── M3.3：FIXED_TIME —— 冻结时钟以确定 lighting ──────
    if feature_flags::fixed_time_enabled() {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::ZERO,
        ));
        info!("[M3.3] FIXED_TIME: clock frozen (delta=0)");
    }

    // ── M3.3：FIXED_CAMERA —— 确定性位姿覆盖 ────────────────
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

    // ── M3.4：CESIUM_SCREENSHOT_SCRIPT —— 批量多视图捕获 ───────
    if let Some(script_path) = feature_flags::screenshot_script_path() {
        match capture_script::ShotScript::from_file(&script_path) {
            Ok(script) => {
                let shots = script.sorted().shot;
                let output_dir = std::env::var("CESIUM_SCREENSHOT_DIR")
                    .unwrap_or_else(|_| ".".to_string());
                // 确保输出目录存在。
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

    // ── M11.3：无头离屏捕获 → PNG → 干净退出 ────────────
    // 仅在 CESIUM_HEADLESS 下激活（默认 OFF → 插件未添加 →
    // 窗口化路径逐字节未被触碰）。创建一个离屏 RGBA8(sRGB)
    // 目标，每帧将场景 Camera3d 重新指向它，渲染
    // `headless_frames()` 预热帧，然后捕获一张 PNG（`tools/pixel_diff`
    // 为像素门控消费的产物）并请求 AppExit。
    // NOTE（推迟至 M11.4）：多视图批量捕获
    // （CESIUM_SCREENSHOT_SCRIPT）与 AutoScreenshot 仍使用
    // Screenshot::primary_window()，因此它们尚未无头感知 —— 它们现在
    // 会发出一个 `warn!`（FIX-HL-EXIT）而非静默失败；通过本插件的单视图
    // 离屏捕获是受支持的无头路径。
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

// ─── M6 Wave A（任务 #81）：相机组件组合 ─────────────────

/// 一次性 setup 系统仍需附加哪些 M6 组件。
///
/// `done` 使系统幂等：每帧重新插入组件会每帧重新提取
/// 一份新副本并扰动 render-world 的 bind group
/// （`prepare_panorama_bind_groups` 反正会逐帧重建它们，但
/// 仅针对真正发生了变化的实体）。
#[derive(Resource)]
struct M6WaveAConfig {
    clipping: bool,
    panorama: bool,
    ibl: bool,
    /// Phase-3 FIX-INTEG：M6.4 加权混合 OIT 门控。
    oit: bool,
    /// Phase-3 FIX-INTEG：M6.6 体积云 composite 门控。
    clouds: bool,
    /// Phase-3 FIX-SPLIT：M6.1 分屏分隔线门控。
    split: bool,
    done: bool,
    /// FIX-HL-RESMUT：程序化 sky-cube [`Image`] 句柄，由
    /// [`m6_wave_a_prepare`] 在 `Startup` 中构建一次。[`m6_wave_a_setup`] 读取（克隆）
    /// 它来附加 panorama，因此那个每帧系统不再需要
    /// 一个持久的 `ResMut<Assets<Image>>` —— 否则它会在每帧为整个 `Update`
    /// stage 占用共享的 image 资产存储。
    sky_image: Option<Handle<Image>>,
}

/// FIX-HL-RESMUT：一次性 `Startup` 系统，构建程序化 sky-cube
/// 图像并将其句柄藏于 [`M6WaveAConfig`] 上。资产创建不
/// 依赖相机，因此它属于 `Startup`（恰好运行一次，之后
/// `Assets<Image>` 借用即被释放）；每帧的
/// [`m6_wave_a_setup`] 随后只消费预先做好的句柄。
fn m6_wave_a_prepare(mut cfg: ResMut<M6WaveAConfig>, mut images: ResMut<Assets<Image>>) {
    if cfg.panorama && cfg.sky_image.is_none() {
        cfg.sky_image = Some(images.add(m6_procedural_sky_cube()));
    }
}

/// 将 M6 组件附加到场景相机，一次性。
///
/// **为何是一个一次性 `Update` 系统而非 `Startup`：**相机实体由
/// `orbit_camera::spawn_orbit_camera` spawn，而后者*私有*于那个模块
/// 并在 `Startup` 中注册，因此 `main.rs` 无法将一个系统
/// `.after(...)` 它。等待第一个存在 `Camera3d` 的 `Update` 帧
/// 是一个无需排序的等价做法，且它保持 `orbit_camera.rs` 未被触碰
/// （M2/M5 红线：相机 bundle 是该模块的地盘）。
///
/// 三个节点由逐视图组件驱动，与 M5 的
/// `CesiumFxaa` / `CesiumAmbientOcclusion` 完全一致。它们需要的 prepass 组件
/// （clipping 的 `DepthPrepass`，IBL 的 `DepthPrepass + NormalPrepass`）由
/// 适配器自己的 `setup_*_prepass` 系统附加，注册在
/// `register_clipping_planes_node` / `register_ibl_node` 内部 —— 因此这个系统只
/// 插入*携带领域值对象的那个组件*。
fn m6_wave_a_setup(
    mut commands: Commands,
    mut cfg: ResMut<M6WaveAConfig>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    if cfg.done {
        return;
    }
    // FIX-HL-RESMUT：确定性地选取相机。`Query::iter().next()`
    // 遵循 archetype/chunk 的迭代顺序，而一旦存在多个 `Camera3d`，
    // 它就不是“主相机”的一个稳定概念。`Entity` 按 (index, generation)
    // 是 `Ord` 的，因此 `.min()` 总解析为先 spawn 的相机
    // （`orbit_camera::spawn_orbit_camera` 在任何辅助视图之前运行）。
    // 一个专用主相机标记会更强大，但 `orbit_camera.rs` 在本
    // 改动的文件范围之外，因此这里使用确定性排序。
    let Some(camera) = cameras.iter().min() else {
        // 相机尚未 spawn —— 保持 `!done` 并在下一帧重试。
        return;
    };

    if cfg.clipping {
        commands.entity(camera).insert(m6_demo_clipping_planes());
    }
    // Phase-3 FIX-INTEG：OIT 是一个整相机标记 —— accumulate/composite
    // 节点在 `enabled == false` 时提前返回，因此一个激活的组件加
    // 上门控 ON 就是开启透明尾链 pass 所需的全部。（将 Bevy 的透明
    // 几何忠实重路由到 MRT 目标被推迟；参见 docs/deviations.md#dev-031。）
    if cfg.oit {
        commands.entity(camera).insert(CesiumOit::default());
    }
    if cfg.clouds {
        commands
            .entity(camera)
            .insert(CesiumClouds::new(m6_demo_cloud_collection()));
    }
    // Phase-3 FIX-SPLIT：分屏分隔线也是一个整相机标记 ——
    // `SplitNode` 在 `enabled == false` 时提前返回，因此插入该组件
    // 加上门控 ON 就开启 overlay。`new(0.5)` 将分隔线居中于
    // 水平中点（一个 `SplitConfig` 拖拽随后移动它）。
    if cfg.split {
        commands.entity(camera).insert(CesiumSplit::new(0.5));
    }
    if cfg.ibl {
        // 领域默认值：`image_based_lighting_factor = [1.0, 1.0]`（因此
        // `CesiumIbl::is_active()` 为 true）且无显式 SH 系数，
        // `IblUniform::from_domain` 通过 `default_spherical_harmonics()` 解析它们。
        commands
            .entity(camera)
            .insert(CesiumIbl::new(
                ImageBasedLighting::default(),
                IblMaterial::default(),
            ));
    }
    if cfg.panorama {
        // FIX-HL-RESMUT：使用在 `m6_wave_a_prepare` 中预创建的句柄，因此这个
        // 每帧系统永不触碰 `Assets<Image>`。`None` 意味着 prepare 未
        // 运行 —— 宁可告警并跳过也不要复活那个持久借用。
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

/// 一个双面地球切割，经典的 CesiumJS `ClippingPlaneCollection` 演示。
///
/// 交集模式（`union_clipping_regions == false`，领域默认值）仅当一个
/// fragment 在**每个**平面之外时才裁剪它，因此这两个平面
/// 移除地球的 `x < 0 && z < 0` 四分之一并保留其余三个
/// 象限完好。距离以**米**计 —— 适配器在打包 uniform 时除以
/// `METERS_PER_RENDER_UNIT = 6378137`，而 `0.0` 将
/// 两个平面都置于球心。
///
/// `edge_width` 以**像素**计，而非米：`shaders/clipping.wgsl` 将其
/// 乘以 `fwidth()`（上游 `czm_metersPerPixel` 的替代，参见
/// docs/deviations.md#dev-023）。
fn m6_demo_clipping_planes() -> CesiumClippingPlanes {
    let mut collection = ClippingPlaneCollection::with_planes(vec![
        // `bevy::math::DVec3`（f64）不在 `bevy::prelude` 中，后者只导出
        // f32 向量类型 —— 完整拼写以使领域端到端保持 f64。
        ClippingPlane::new(bevy::math::DVec3::new(0.0, 0.0, 1.0), 0.0),
        ClippingPlane::new(bevy::math::DVec3::new(1.0, 0.0, 0.0), 0.0),
    ]);
    collection.edge_width = 2.0; // PIXELS
    collection.edge_color = [1.0, 1.0, 1.0, 1.0];
    CesiumClippingPlanes::new(collection)
}

/// M6.6 门控演示的一个小型确定性云场。
///
/// 位置与最大尺寸以**米**计（公制 f64 领域；适配器
/// 在 GPU 边界收窄为 render unit）。这仅在 `CESIUM_ENABLE_CLOUDS=1` 时
/// 才到达 GPU（默认 OFF → 节点永不注册且
/// v0 基线保持位精确）；composite 的 `is_active()` 守卫另外会在集合
/// 为空或隐藏时提前返回。忠实的逐云 billboard + 3D-noise
/// 路径被推迟到一个真 GPU 任务
/// （`docs/deviations.md#dev-032`）；此云场端到端地演练屏幕空间
/// ray-march composite。
fn m6_demo_cloud_collection() -> CloudCollection {
    use bevy::math::DVec3;
    let mut collection = CloudCollection::new();
    // 一片浅弧状的 cumulus 小云，每朵约为一个 ~2 km × 1 km 椭球，
    // 沿 ±x 排列并抬到低高度，以便 orbit 相机能框住它们。
    for i in -2..=2 {
        let x = f64::from(i) * 2500.0;
        let position = DVec3::new(x, 0.0, 3000.0);
        let maximum_size = DVec3::new(2000.0, 1200.0, 800.0);
        collection.add(CumulusCloud::new(position, maximum_size));
    }
    collection
}

/// 一个程序生成的六面 cube map，作为一个真实天空资产的替代。
///
/// **为何是程序生成，以及为何用 `CubeMapPanorama` 而非上游默认的
/// `EquirectangularPanorama`**（docs/deviations.md#dev-028，deferred.md #58）：
/// 上游的等矩形 panorama 是一个*有限球泡*，其默认半径
/// 为 `DEFAULT_PANORAMA_RADIUS = 100_000 m = 0.0157` render unit —— 约地球半径的 1.6 %。
/// `orbit_camera` 从不会靠近到低于 `1.005` render unit，因此那个球泡
/// 完全位于地球内部并被 opaque pass 隐藏：门控 ON 会与
/// 门控 OFF 无法区分。cube-map 放置（`PanoramaPlacement::Skybox`）以相机为中心
/// 且无限，不写深度，因此它正是使文档化的
/// `MainOpaquePass → Panorama → MainTransparentPass(starfield r=50 → sky dome
/// r=40)` 顺序自洽的那个放置 —— panorama 填满天空，透明 draw
/// 仍在其上方通过深度测试，且 sky dome 的三个排序机制
/// （`depth_bias` / `Premultiplied` / `cull Front`）保持未被触碰。
///
/// 形状要求就是 `prepare_panorama_bind_groups` 强制的那些：
/// 一个 `texture_cube` 插槽需要恰好**六个 array layer**，因此图像
/// 构建为一个 `depth_or_array_layers = 6` 的 2D array 纹理，以及一个
/// `dimension` 为 `Cube` 的 `TextureViewDescriptor`（bevy 0.15.3 在
/// `GpuImage::prepare_asset` 中逐字遵从 `Image::texture_view_descriptor`）。
///
/// 颜色是 **sRGB 字节**，因为格式为 `Rgba8UnormSrgb`：硬件
/// 执行上游 `czm_gammaCorrect` 曾手工做的 sRGB → linear 解码
/// （项目 sRGB 红线，docs/deviations.md#dev-025）。
fn m6_procedural_sky_cube() -> Image {
    /// 每面边长。有意很小：跨一个平滑垂直梯度的线性过滤
    /// 就是这个占位物需要做的一切。
    const FACE: u32 = 8;
    const ZENITH: [u8; 3] = [38, 78, 160];
    const HORIZON: [u8; 3] = [126, 148, 176];
    const NADIR: [u8; 3] = [12, 14, 22];

    let mut data = Vec::with_capacity(6 * (FACE * FACE * 4) as usize);
    // 面顺序是上游的 `[+X, -X, +Y, -Y, +Z, -Z]`
    // （`CubeMapPanorama::FACE_NAMES` ≡ `SkyBox.js::createEarthSkyBox`）。
    for face in 0..6u32 {
        for y in 0..FACE {
            // `t = 0` 在该面的 zenith 边缘，`1` 在 nadir 边缘。在一个
            // cube map 中四个侧面沿 `+Y`（行 0）→ `-Y`（最后一行）运行，因此一个
            // 普通的行索引已是梯度轴；`+Y`（面 2）是 zenith 盖，
            // `-Y`（面 3）是 nadir 盖。
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

/// 为占位天空梯度做的逐分量字节 lerp。
///
/// `t` 在两个调用处都始终处于 `[0, 1]`，因此结果不会离开
/// `u8` 范围且 `as u8` 收窄是精确的。
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

    /// FIX-HL-RESMUT：有两个 `Camera3d` 实体时，一次性 setup 必须将
    /// M6 组件附加到*确定性选取*的相机 —— 即 `cameras.iter().min()`
    /// 解析出的同一个 —— 而非 `Query::iter()` 恰好首先产出
    /// 的那一个。一个仅 clipping 的配置使该 fixture 免受
    /// `Assets<Image>` / render-world 依赖。
    #[test]
    fn m6_wave_a_setup_attaches_to_the_deterministic_camera() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let cam_a = app.world_mut().spawn(Camera3d::default()).id();
        let cam_b = app.world_mut().spawn(Camera3d::default()).id();
        // 镜像系统的选取规则（按 `Ord` 取最小的 `Entity`）。
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
