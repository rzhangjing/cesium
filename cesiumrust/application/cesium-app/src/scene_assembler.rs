//! 场景组装器：在启动时引导出一个最小但完整的 Cesium 场景。
//!
//! 创建地球实体、camera、测试影像图层与测试实体
//! （点、折线、多边形、billboard），以验证渲染管线。
//!
//! 键盘控制：
//!   R —— 将 camera 重置为默认视图
//!   T —— 切换地形线框
//!   L —— 循环切换影像图层
//!   F —— 飞行至 Grand Canyon 预设
//!   H —— 向控制台打印场景统计

use bevy::prelude::*;
use cesium_bevy_render::{
    create_ellipsoid_mesh, CesiumCamera, CesiumGlobe, CesiumTerrainTile,
    FlyToRequest, GlobeConfig, TileLoadStats, METERS_PER_RENDER_UNIT,
};
use cesium_bevy_render::imagery::ImageryLayerManager;
use cesium_camera::Camera as DomainCamera;
use cesium_geospatial::cartographic::Cartographic;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_scene_mode::SceneMode;

use crate::orbit_camera::OrbitState;

// ── 资源 ────────────────────────────────────────────────────────

/// 地形线框开关资源（true 表示开启线框显示）。
#[derive(Resource, Default)]
struct WireframeMode(bool);

/// 当前激活的影像图层循环索引资源。
#[derive(Resource)]
struct ImageryCycleIndex(usize);

impl Default for ImageryCycleIndex {
    /// 默认从第一个影像图层（索引 0）开始。
    fn default() -> Self {
        Self(0)
    }
}

/// 周期性打印场景统计的定时器资源。
#[derive(Resource)]
struct SceneStatsTimer(Timer);

impl Default for SceneStatsTimer {
    /// 默认每 5.0 秒重复触发一次统计打印。
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}

// ── 插件 ───────────────────────────────────────────────────────────

/// 场景组装插件：注册资源并在启动/更新阶段挂载各系统。
pub struct SceneAssemblerPlugin;

impl Plugin for SceneAssemblerPlugin {
    /// 插件装配入口：初始化资源，注册启动系统（搭场景/建实体/健康检查）
    /// 与更新系统（键盘控制/周期统计）。
    ///
    /// # 参数
    /// - `app`：Bevy 应用。
    fn build(&self, app: &mut App) {
        app.init_resource::<WireframeMode>()
            .init_resource::<ImageryCycleIndex>()
            .init_resource::<SceneStatsTimer>()
            .add_systems(Startup, (setup_scene, setup_entities).chain())
            .add_systems(Startup, print_scene_health_check)
            .add_systems(
                Update,
                (
                    keyboard_controls,
                    print_scene_stats,
                ),
            );
    }
}

// ── 地球 spawn ──────────────────────────────────────────────────

/// 启动系统：组装地球实体、camera 与影像图层。
///
/// # 参数
/// - `commands`：实体命令生成器。
/// - `meshes`：网格资产库。
/// - `materials`：标准材质资产库。
/// - `globe_config`：地球配置资源（可变）。
/// - `imagery_mgr`：影像图层管理器（可变）。
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut globe_config: ResMut<GlobeConfig>,
    mut imagery_mgr: ResMut<ImageryLayerManager>,
) {
    let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;

    // ── 地球实体 ──────────────────────────────────────────────
    // 生成一个低精度椭球网格作为地球基础几何，并配一个深蓝色粗精面材质。
    let globe_mesh = create_ellipsoid_mesh(64, 128);
    let globe_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.18, 0.35),
        perceptual_roughness: 0.85,
        ..default()
    });

    let globe_id = commands
        .spawn((
            CesiumGlobe,
            Mesh3d(meshes.add(globe_mesh)),
            MeshMaterial3d(globe_material),
            Transform::from_scale(Vec3::splat(scale)),
        ))
        .with_children(|parent| {
            // 作为占位，给地球附一个 level 0 的地形瓦片子实体。
            parent.spawn((
                CesiumTerrainTile {
                    x: 0,
                    y: 0,
                    level: 0,
                },
            ));
        })
        .id();

    println!(
        "[SceneAssembler] Spawned globe entity {:?} with WGS84 ellipsoid mesh (64×128)",
        globe_id
    );

    // ── 配置地球 ───────────────────────────────────────────────
    // 配置地球使用 WGS84 椭球，并预置一个 OSM 影像模板供快速验证。
    globe_config.ellipsoid = Ellipsoid::WGS84;
    globe_config.imagery_providers.push(
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png".into(),
    );

    // ── Camera ────────────────────────────────────────────────────
    // 位置：从太空（约 3 倍地球半径外）望向北美
    // 先由经纬度/高度算出相机位置与注视目标，再导出朝向与 up 向量。
    let camera_position = ellipsoid_position(-95.0, 40.0, 20_000_000.0);
    let look_target = ellipsoid_position(-95.0, 40.0, 0.0);
    let direction = (look_target - camera_position).normalize();
    let up = camera_position.normalize();

    let domain_camera = DomainCamera::new(
        camera_position,
        direction,
        up,
    );

    // 用领域 camera 包装为 CesiumCamera 组件，并给实体挂上面向目标的 Transform。
    commands.spawn((
        CesiumCamera {
            camera: domain_camera,
            scene_mode: SceneMode::Scene3D,
            enable_collision_detection: true,
            minimum_zoom_distance: 100.0,
            maximum_zoom_distance: 20_000_000.0,
        },
        Transform::from_translation(camera_position.as_vec3())
            .looking_at(look_target.as_vec3(), up.as_vec3()),
    ));

    println!(
        "[SceneAssembler] Spawned CesiumCamera at ({:.1}, {:.1}) altitude={:.0}m",
        -95.0, 40.0, 20_000_000.0
    );

    // ── 影像图层 ───────────────────────────────────────────────
    // 依次添加 OSM / ArcGIS World Imagery / Stamen Toner 三层（不透明度递减）。
    imagery_mgr.add_layer(
        "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
        1.0,
        0,
        18,
    );
    imagery_mgr.add_layer(
        // 第二层：ArcGIS World Imagery 影像（z/y/x 轴序）。
        "https://services.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}",
        1.0,
        0,
        18,
    );
    imagery_mgr.add_layer(
        // 第三层：Stamen Toner 风格图层，不透明度 0.8。
        "https://tiles.stadiamaps.com/tiles/stamen_toner/{z}/{x}/{y}.png",
        0.8,
        0,
        18,
    );

    println!(
        "[SceneAssembler] Configured {} imagery layers",
        imagery_mgr.layer_count()
    );

    println!("[SceneAssembler] Scene setup complete");
}

// ── 测试实体 ────────────────────────────────────────────────────────

/// 启动系统：在地球上放置测试实体（点/折线/多边形/billboard）。
///
/// # 参数
/// - `commands`：实体命令生成器。
/// - `meshes`：网格资产库。
/// - `materials`：标准材质资产库。
fn setup_entities(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;

    // 纽约处的点
    // 用高亮的 unlit 红色小球代表一个点实体，验证地理坐标→世界坐标映射。
    let ny_pos = ellipsoid_position(-74.006, 40.7128, 1000.0);
    let point_material = materials.add(StandardMaterial {
        // 点实体使用纯红 unlit 材质，保证在地球上高亮可见。
        base_color: Color::srgb(1.0, 0.0, 0.0),
        emissive: LinearRgba::rgb(50.0, 0.0, 0.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.002))),
        MeshMaterial3d(point_material),
        Transform {
            translation: ny_pos.as_vec3() * scale,
            scale: Vec3::splat(5.0),
            ..default()
        },
    ));
    println!(
        "[SceneAssembler] Spawned point at New York ({:.4}, {:.4})",
        -74.006, 40.7128
    );

    // 从旧金山到纽约的折线
    // 用 LineStrip 网格连接 SF→NY 两点，验证折线几何上传。
    let sf_pos = ellipsoid_position(-122.4194, 37.7749, 1000.0);
    let line_mesh = create_line_mesh(&[sf_pos, ny_pos], scale);
    let line_material = materials.add(StandardMaterial {
        // 折线使用蓝色 unlit 材质，与点区隔。
        base_color: Color::srgb(0.0, 0.3, 1.0),
        emissive: LinearRgba::rgb(5.0, 0.0, 15.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(line_mesh)),
        MeshMaterial3d(line_material),
        Transform::default(),
    ));
    println!(
        "[SceneAssembler] Spawned polyline SF→NY ({}→{})",
        "37.8N 122.4W", "40.7N 74.0W"
    );

    // 覆盖德克萨斯的多边形（近似包围矩形）
    // 以四角经纬度构造矩形，用半透绿色填充验证多边形三角扇。
    let texas_points = [
        (-106.5, 36.5),
        (-106.5, 31.5),
        (-95.5, 31.5),
        (-95.5, 36.5),
    ];
    let tx_surf: Vec<glam::DVec3> = texas_points
        .iter()
        .map(|&(lon, lat)| ellipsoid_position(lon, lat, 5000.0))
        .collect();
    let tx_material = materials.add(StandardMaterial {
        // 多边形使用半透绿色 Blend 材质，可透出下方地形。
        base_color: Color::srgba(0.0, 0.8, 0.2, 0.3),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(create_polygon_mesh(&tx_surf, scale))),
        MeshMaterial3d(tx_material),
        Transform::default(),
    ));
    println!("[SceneAssembler] Spawned polygon over Texas (4 vertices)");

    // 伦敦处的 billboard
    // 用一个金黄色小球作为简易 billboard 占位。
    let london_pos = ellipsoid_position(-0.1276, 51.5074, 50000.0);
    let billboard_material = materials.add(StandardMaterial {
        // billboard 使用金黄色 unlit 材质。
        base_color: Color::srgb(1.0, 0.84, 0.0),
        emissive: LinearRgba::rgb(30.0, 20.0, 0.0),
        unlit: true,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.003))),
        MeshMaterial3d(billboard_material),
        Transform {
            translation: london_pos.as_vec3() * scale,
            scale: Vec3::splat(5.0),
            ..default()
        },
    ));
    println!("[SceneAssembler] Spawned billboard at London ({:.4}, {:.4})", -0.1276, 51.5074);
}

// ── 键盘控制 ────────────────────────────────────────────────────────

/// 更新系统：响应键盘事件（R/T/L/F/H）切换视图、线框、影像与飞行。
///
/// # 参数
/// - `keys`：本帧按键输入。
/// - `state`：轨道 camera 状态（可变）。
/// - `wireframe`：线框模式资源（可变）。
/// - `cycle_idx`：影像图层循环索引（可变）。
/// - `imagery_mgr`：影像图层管理器。
/// - `ev_fly`：FlyToRequest 事件写入器。
/// - `globe_config`：地球配置。
fn keyboard_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<OrbitState>,
    mut wireframe: ResMut<WireframeMode>,
    mut cycle_idx: ResMut<ImageryCycleIndex>,
    imagery_mgr: Res<ImageryLayerManager>,
    mut ev_fly: EventWriter<FlyToRequest>,
    globe_config: Res<GlobeConfig>,
) {
    // R 键：将轨道 camera 状态重置为默认视图。
    if keys.just_pressed(KeyCode::KeyR) {
        *state = OrbitState::default();
        println!("[Ctrl] Camera reset to default view");
    }

    // T 键：切换地形线框显示。
    if keys.just_pressed(KeyCode::KeyT) {
        wireframe.0 = !wireframe.0;
        println!("[Ctrl] Terrain wireframe: {}", if wireframe.0 { "ON" } else { "OFF" });
    }

    // L 键：在可用影像图层之间循环切换活动图层。
    if keys.just_pressed(KeyCode::KeyL) {
        let count = imagery_mgr.layer_count();
        if count > 0 {
            // 循环递增并取模，切换到一个活动影像层。
            cycle_idx.0 = (cycle_idx.0 + 1) % count;
            println!(
                "[Ctrl] Active imagery layer: {} / {}",
                cycle_idx.0 + 1,
                count
            );
        }
    }

    // F 键：发送 FlyToRequest 事件，飞向 Grand Canyon 预设。
    if keys.just_pressed(KeyCode::KeyF) {
        let target = Cartographic::from_degrees(-112.1, 36.1, 5000.0);
        ev_fly.send(FlyToRequest {
            destination: target,
            duration_secs: 1.5,
        });
        println!("[Ctrl] Flying to Grand Canyon (36.1N 112.1W)");
    }

    // H 键：立即向控制台打印一份当前场景统计快照。
    if keys.just_pressed(KeyCode::KeyH) {
        print_concurrent_stats(&globe_config, &imagery_mgr, &state);
    }
}

// ── 场景统计 ───────────────────────────────────────────────────────

/// 更新系统：每 5 秒打印一次场景统计（地球/瓦片/影像/FPS/camera）。
///
/// # 参数
/// - `time`：帧时间资源。
/// - `timer`：统计定时器（可变）。
/// - `state`：轨道 camera 状态。
/// - `globe_config`：地球配置。
/// - `imagery_mgr`：影像图层管理器。
/// - `stats`：瓦片加载统计。
/// - `globe_query`：地球实体查询。
/// - `tile_query`：地形瓦片查询。
fn print_scene_stats(
    time: Res<Time>,
    mut timer: ResMut<SceneStatsTimer>,
    state: Res<OrbitState>,
    globe_config: Res<GlobeConfig>,
    imagery_mgr: Res<ImageryLayerManager>,
    stats: Res<TileLoadStats>,
    globe_query: Query<(), With<CesiumGlobe>>,
    tile_query: Query<&CesiumTerrainTile>,
) {
    // 定时器未到期则直接返回，避免每帧都打印。
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    // 统计地球/瓦片/影像图层数量，并由帧间隔估算 FPS。
    let globe_count = globe_query.iter().count();
    let tile_count = tile_query.iter().count();
    let imagery_count = imagery_mgr.layer_count();
    // FPS 由本帧间隔的倒数估算。
    let fps = 1.0 / time.delta_secs();

    // 计算近似的 camera 经纬度
    let cam_pos = compute_cam_position(&state);
    let ellipsoid = &globe_config.ellipsoid;
    let carto = ellipsoid.cartesian_to_cartographic(cam_pos);

    println!(
        "[Stats] Globes={} Tiles={} ImageryLayers={} FPS={:.1} | Cam={}",
        globe_count,
        tile_count,
        imagery_count,
        fps,
        carto
            .map(|c| format!("{:.2}°N {:.2}°W H={:.0}m", c.latitude.to_degrees(), -c.longitude.to_degrees(), c.height))
            .unwrap_or_else(|| "unknown".into())
    );

    if stats.tiles_loaded > 0 || stats.tiles_failed > 0 {
        println!(
            "[Stats] Downloads: {} loaded, {} failed ({} MB)",
            stats.tiles_loaded,
            stats.tiles_failed,
            stats.bytes_downloaded / 1_000_000
        );
    }
}

/// 向控制台打印当前场景统计：椭球、影像图层列表、camera 经纬度与高度。
///
/// # 参数
/// - `globe_config`：地球配置（提供椭球）。
/// - `imagery_mgr`：影像图层管理器。
/// - `state`：当前轨道 camera 状态。
fn print_concurrent_stats(
    globe_config: &GlobeConfig,
    imagery_mgr: &ImageryLayerManager,
    state: &OrbitState,
) {
    let ellipsoid = &globe_config.ellipsoid;
    // 先由轨道状态算出相机世界坐标，再反投影为经纬度供输出。
    let cam_pos = compute_cam_position(state);
    let carto = ellipsoid.cartesian_to_cartographic(cam_pos);

    println!("═══ CesiumRust Scene Statistics ═══");
    println!("  Ellipsoid: {:?}", ellipsoid);
    println!("  Imagery layers: {}", imagery_mgr.layer_count());
    for (i, layer) in imagery_mgr.layers.iter().enumerate() {
        println!(
            "    [{}] {} (levels {}-{}, opacity={:.2})",
            i + 1,
            &layer.url_template[..layer.url_template.len().min(60)],
            layer.min_level,
            layer.max_level,
            layer.opacity
        );
    }
    println!("  Imagery enabled: {}", imagery_mgr.enabled);
    println!(
        "  Camera: {:.4}°N {:.4}°W altitude={:.0}m dist={:.2}RU",
        carto.map(|c| c.latitude.to_degrees()).unwrap_or(0.0),
        carto.map(|c| -c.longitude.to_degrees()).unwrap_or(0.0),
        carto.map(|c| c.height).unwrap_or(0.0),
        state.distance
    );
    println!("══════════════════════════════════════");
}

// ── 健康检查 ──────────────────────────────────────────────────────

/// 启动系统：打印一次健康检查，确认地球/camera/影像均已就绪。
///
/// # 参数
/// - `globe_query`：地球实体查询。
/// - `camera_query`：camera 实体查询。
/// - `imagery_mgr`：影像图层管理器。
fn print_scene_health_check(
    globe_query: Query<(), With<CesiumGlobe>>,
    camera_query: Query<(), With<CesiumCamera>>,
    imagery_mgr: Res<ImageryLayerManager>,
) {
    // 逐项检查地球/camera/影像是否就绪，汇总为一个通过/警告标志。
    println!("═══ CesiumRust Health Check ═══");

    // 地球网格是否已被 spawn（查询到带 CesiumGlobe 的实体）。
    let globe_ok = !globe_query.is_empty();
    println!(
        "  Globe mesh:  {}",
        if globe_ok { "✓ SPAWNED" } else { "✗ MISSING" }
    );

    // camera 是否已激活。
    let camera_ok = !camera_query.is_empty();
    println!(
        "  Camera:      {}",
        if camera_ok {
            "✓ ACTIVE"
        } else {
            "✗ MISSING"
        }
    );

    // 影像层至少配置了一个。
    let imagery_ok = imagery_mgr.layer_count() > 0;
    println!(
        "  Imagery:     {} ({} layers)",
        if imagery_ok { "✓ CONFIGURED" } else { "⚠  NONE" },
        imagery_mgr.layer_count()
    );

    let all_ok = globe_ok && camera_ok && imagery_ok;
    // 三项全部就绪才判为通过；否则向 stderr 输出缺项警告。
    println!(
        "  Overall:     {}",
        if all_ok { "✓ ALL CHECKS PASSED" } else { "⚠  SOME CHECKS FAILED" }
    );
    println!("═════════════════════════════");

    if !all_ok {
        eprintln!("[HealthCheck] ⚠  WARNING: {}",
            vec![
                if !globe_ok { Some("globe mesh not spawned") } else { None },
                if !camera_ok { Some("camera not spawned") } else { None },
                if !imagery_ok { Some("no imagery layers configured") } else { None },
            ].into_iter().flatten().collect::<Vec<_>>().join(", ")
        );
    } else {
        println!("[HealthCheck] ✓ All systems nominal");
    }
}

// ── 辅助函数 ───────────────────────────────────────────────────────

/// 由经纬度（度）与高度（米）计算 WGS84 下的笛卡尔位置。
///
/// # 参数
/// - `lon_deg`：经度（度）。
/// - `lat_deg`：纬度（度）。
/// - `height`：椭球以上高度（米）。
fn ellipsoid_position(lon_deg: f64, lat_deg: f64, height: f64) -> glam::DVec3 {
    let carto = Cartographic::from_degrees(lon_deg, lat_deg, height);
    Ellipsoid::WGS84.cartographic_to_cartesian(&carto)
}

/// 根据轨道状态（heading/pitch/distance）计算 camera 在世界坐标中的位置。
///
/// # 参数
/// - `state`：轨道状态。
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

/// 由一串世界坐标点构造 LineStrip 拓扑的折线 [`Mesh`]（顶点已乘以 scale）。
///
/// # 参数
/// - `points`：世界坐标点列表。
/// - `scale`：渲染单位缩放因子。
fn create_line_mesh(points: &[glam::DVec3], scale: f32) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // 逐点收集：每个点占一个顶点索引，按顺序连成线段。
    for point in points {
        indices.push(positions.len() as u32);
        let p = point.as_vec3() * scale;
        positions.push(p.into());
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::LineStrip,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

/// 由多边形顶点集构造一个以质心为顶点的三角扇填充 [`Mesh`]。
///
/// # 参数
/// - `points`：多边形顶点（世界坐标）。
/// - `scale`：渲染单位缩放因子。
fn create_polygon_mesh(points: &[glam::DVec3], scale: f32) -> Mesh {
    // 从首个顶点出发的简单三角扇
    let mut positions: Vec<[f32; 3]> = Vec::new();
    // 先求所有顶点的均值作为扇形中心点。
    let center = points.iter().fold(glam::DVec3::ZERO, |a, b| a + *b) / points.len() as f64;

    // 从首个顶点开始按顺序遍历，累加顶点并记录索引。
    for point in points {
        let p = point.as_vec3() * scale;
        positions.push(p.into());
    }
    // 追加质心作为三角扇的公共顶点。
    let c = center.as_vec3() * scale;
    positions.push(c.into());

    let n = points.len() as u32;
    let center_idx = n;
    let mut indices: Vec<u32> = Vec::new();

    // 相邻两顶点与中心点组成一个三角形，遍历一周形成扇形。
    for i in 0..n {
        let next = (i + 1) % n;
        indices.push(center_idx);
        indices.push(i);
        indices.push(next);
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh.compute_normals();
    mesh
}

// ── 集成测试 ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cesium_bevy_render::{
        components::{CesiumGlobe, CesiumTerrainTile},
        camera::components::CesiumCamera,
    };

    #[test]
    fn test_ellipsoid_position_origin() {
        let pos = ellipsoid_position(0.0, 0.0, 0.0);
        assert!((pos.x - METERS_PER_RENDER_UNIT).abs() < 10.0, "x={}", pos.x);
        assert!(pos.y.abs() < 1.0, "y={}", pos.y);
        assert!(pos.z.abs() < 1.0, "z={}", pos.z);
    }

    #[test]
    fn test_ellipsoid_position_north_pole() {
        let pos = ellipsoid_position(0.0, 90.0, 0.0);
        assert!(pos.x.abs() < 1.0, "x={}", pos.x);
        assert!(pos.y.abs() < 1.0, "y={}", pos.y);
        assert!(pos.z > 6_350_000.0, "z={}", pos.z);
    }

    #[test]
    fn test_ellipsoid_position_new_york() {
        let pos = ellipsoid_position(-74.006, 40.7128, 0.0);
        let dist = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
        assert!(dist > 6_000_000.0 && dist < 6_500_000.0, "distance from center: {}", dist);
    }

    #[test]
    fn test_create_line_mesh_has_correct_topology() {
        let sf = ellipsoid_position(-122.4194, 37.7749, 1000.0);
        let ny = ellipsoid_position(-74.006, 40.7128, 1000.0);
        let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;
        let mesh = create_line_mesh(&[sf, ny], scale);

        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            assert_eq!(pos.len(), 2);
        } else {
            panic!("Expected Float32x3 positions");
        }
    }

    #[test]
    fn test_create_polygon_mesh_has_correct_vertices() {
        let points: Vec<glam::DVec3> = [
            (-106.5, 36.5),
            (-106.5, 31.5),
            (-95.5, 31.5),
            (-95.5, 36.5),
        ]
        .iter()
        .map(|&(lon, lat)| ellipsoid_position(lon, lat, 0.0))
        .collect();
        let scale = (1.0 / METERS_PER_RENDER_UNIT) as f32;
        let mesh = create_polygon_mesh(&points, scale);

        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        if let bevy::render::mesh::VertexAttributeValues::Float32x3(pos) = positions {
            // 4 个周边顶点 + 1 个中心
            assert_eq!(pos.len(), 5);
        } else {
            panic!("Expected Float32x3 positions");
        }

        assert!(mesh.indices().is_some(), "polygon mesh should have indices");
    }

    #[test]
    fn test_compute_cam_position_default() {
        let state = OrbitState::default();
        let pos = compute_cam_position(&state);
        let expected_dir = glam::DVec3::new(
            3.0 * (0.4_f64).cos() * (0.0_f64).cos(),
            3.0 * (0.4_f64).cos() * (0.0_f64).sin(),
            3.0 * (0.4_f64).sin(),
        );
        let diff = (pos - expected_dir).length();
        assert!(diff < 0.01, "diff={}", diff);
    }

    #[test]
    fn test_imagery_cycle_has_expected_count() {
        let mut mgr = ImageryLayerManager::default();
        mgr.add_layer("https://a.tiles.example.com/{z}/{x}/{y}.png", 1.0, 0, 18);
        mgr.add_layer("https://b.tiles.example.com/{z}/{x}/{y}.png", 0.5, 0, 12);

        assert_eq!(mgr.layer_count(), 2);
        assert_eq!(mgr.visible_layers().count(), 2);
    }

    #[test]
    fn test_imagery_layer_cycle() {
        let mut mgr = ImageryLayerManager::default();
        mgr.add_layer("https://a.tiles.example.com/{z}/{x}/{y}.png", 1.0, 0, 18);

        let mut idx = ImageryCycleIndex::default();
        assert_eq!(idx.0, 0);

        let count = mgr.layer_count();
        idx.0 = (idx.0 + 1) % if count > 0 { count } else { 1 };
        assert_eq!(idx.0, 0, "should wrap around to 0 when only 1 layer");
    }

    #[test]
    fn test_scene_assembler_resources_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<WireframeMode>();
        assert_send::<ImageryCycleIndex>();
        assert_send::<SceneStatsTimer>();
    }
}
