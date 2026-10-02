use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_decoders::decode_quantized_mesh;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use cesium_terrain::{QuantizedMeshTerrainData, TerrainMesh};

use crate::components::{CesiumTerrainTile, TileContentState};
use crate::pipeline::{self, budget};
use crate::resources::{GlobeConfig, TileLoadStats};

use super::lod_system::TerrainSelection;

/// Tile 坐标 key `(x, y, level)`。
pub type TileKey = (u32, u32, u32);

#[derive(Resource, Default)]
pub struct TerrainLoadState {
    pub loaded_count: u32,
    pub failed_count: u32,
}

/// 飞行中的异步 terrain tile 加载。
///
/// 以 `TileKey` 为键；每个条目拥有所 spawn 的 [`IoTaskPool`] task 加上
/// task 解析完成后接收解码 mesh 的 placeholder entity。
/// 把 task 存在这里（而非 drain 一个共享队列）正是
/// 从结构上消除 load/render drain 竞态的原因：loader 是 `TerrainSelection::tiles_to_load`
/// 的*唯一*消费者。
#[derive(Resource, Default)]
pub struct TerrainPendingLoads {
    pub pending: HashMap<TileKey, PendingLoad>,
    /// 已解析但尚未上传的 mesh，按 FIFO drain 至多到逐帧
    /// terrain mesh 预算（门控 ON）。门控 OFF 时（budget = `UNBOUNDED`）始终在帧内
    /// 完全 drain，因此旧行为保持不变。条目在其 task 解析的那一刻
    /// 被移到这里（从不重新 poll），这正是
    /// 预算能推迟 GPU 上传而又不触及已完成 task 的原因。
    pub ready_backlog: VecDeque<(TileKey, Entity, Result<TerrainMesh, String>)>,
}

pub struct PendingLoad {
    /// 以 `Loading` 状态 spawn、等待解码 mesh 的 entity。
    pub entity: Entity,
    /// 后台下载+解码 task；产出 CPU 侧的 `TerrainMesh`。
    pub task: Task<Result<TerrainMesh, String>>,
}

/// 将 `{z}/{x}/{y}` 模板展开为具体瓦片 URL；未配置底图地址时返回 `None`。
///
/// # 参数
/// - `config`：地球配置（携 terrain_provider_url 模板）
/// - `x`/`y`/`level`：瓦片的列/行/层级
fn build_terrain_url(config: &GlobeConfig, x: u32, y: u32, level: u32) -> Option<String> {
    let base = config.terrain_provider_url.as_ref()?;
    Some(
        base.replace("{z}", &level.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string()),
    )
}

/// 由瓦片索引推导其覆盖的经纬度矩形（等经纬度切分）。
///
/// # 参数
/// - `x`/`y`/`level`：瓦片的列/行/层级
fn tile_rectangle(x: u32, y: u32, level: u32) -> Rectangle {
    let n = 2u32.pow(level.max(1)) as f64;
    let west = (x as f64 / n) * std::f64::consts::TAU - std::f64::consts::PI;
    let east = ((x as f64 + 1.0) / n) * std::f64::consts::TAU - std::f64::consts::PI;
    let south = (y as f64 / n) * std::f64::consts::PI - std::f64::consts::FRAC_PI_2;
    let north = ((y as f64 + 1.0) / n) * std::f64::consts::PI - std::f64::consts::FRAC_PI_2;
    Rectangle::from_radians(west, south, east, north)
}

/// terrain 的零级最大*几何*误差，以米为单位。
///
/// 这是最粗糙的 terrain 高度误差尺度（米）——**不是** tiling
/// scheme 的水平半周长（`semiMajorAxis * PI / 2 ≈ 1e7 m`）。
/// 从那个水平尺度推导垂裙会使 level-0 垂裙达 ~5e7 m
///（大于地球半径 ~6.4e6 m），于是 `add_skirts` 的 `carto.height -
/// skirt` 变成巨大的负值，将垂裙顶点通过地心镜像
/// 成刺穿 globe 的退化尖峰。
const LEVEL_ZERO_MAXIMUM_GEOMETRIC_ERROR: f64 = 100.0;

/// 任意 tile 垂裙高度的硬上界（米）。垂裙只需
/// 覆盖相邻 tile 之间的 LOD 裂缝；这个钳位保证它
/// 永远不会大到把几何折叠穿过地心，即使上面
/// 的 level-0 估计日后被向上修正。
const MAX_SKIRT_HEIGHT: f64 = 1000.0;

/// `level` 处 tile 的垂直垂裙高度（米）。
///
/// 镜像 CesiumJS 的 `getLevelMaximumGeometricError(level) * 5.0`
///（`= levelZeroMaximumGeometricError / 2^level * 5.0`），但使用米级的
/// level-0 误差和一个合理的上钳位，使垂裙保持物理上合理。
/// TODO 对齐 CesiumJS 垂裙公式：接入真实 tiling scheme 的 level-zero error 后精确对齐。
fn skirt_height_for_level(level: u32) -> f64 {
    let denom = (1u64 << level.min(32)) as f64;
    (LEVEL_ZERO_MAXIMUM_GEOMETRIC_ERROR / denom * 5.0).min(MAX_SKIRT_HEIGHT)
}

/// 下载并二进制解码一个 quantized-mesh terrain tile。
///
/// 阻塞式：预期运行在一个 [`IoTaskPool`] worker 线程上，从不在
/// 帧线程。fetch 不依赖 tokio——它经由 cesium-pipeline
/// core 的 ureq 阻塞后端（[`pipeline::fetch::fetch_gated`]），选择
/// 共享 keep-alive 池（门控 ON）或每次调用新建的客户端（门控 OFF）。
fn fetch_and_decode_terrain(
    url: &str,
    skirt_height: f64,
    use_pipeline: bool,
) -> Result<QuantizedMeshTerrainData, String> {
    let data = pipeline::fetch::fetch_gated(url, use_pipeline)?;

    // 二进制 quantized-mesh 解码（之前错误地用了 `serde_json::from_slice`）。
    decode_quantized_mesh(&data, skirt_height).map_err(|e| format!("decode: {}", e))
}

/// 完整的 CPU 侧 worker：fetch + 解码 + 构建带垂裙的渲染 mesh。
///
/// 只产出 CPU 侧数据（`TerrainMesh`）；GPU 上传稍后在
/// `terrain_render_system` 中进行。运行在一个 [`IoTaskPool`] worker 线程上。
fn load_and_decode_terrain(
    url: &str,
    rect: &Rectangle,
    ellipsoid: &Ellipsoid,
    skirt_height: f64,
    use_pipeline: bool,
) -> Result<TerrainMesh, String> {
    let qm = fetch_and_decode_terrain(url, skirt_height, use_pipeline)?;
    Ok(qm.create_mesh_with_skirts(rect, ellipsoid, 1.0))
}

/// Poll 飞行中的 task，并把已解析的 tile 转换 `Loading → Ready/Failed`。
///
/// 每个 task 都被恰好 poll 一次（`poll_once`），因此这从不阻塞帧
/// 线程；未解析的 task 留在 `pending` 中，下一帧再次 poll。
/// 已解析的结果被移进一个 FIFO backlog，每帧至多上传 `budget`
/// 个 mesh——`UNBOUNDED`（门控 OFF）像迁移前的代码那样在帧内 drain 一切，
/// 而门控 ON 把 GPU 工作约束到 terrain
/// mesh 权重（[`budget::terrain_mesh_budget`]）。
fn poll_pending_loads(
    commands: &mut Commands,
    pending: &mut TerrainPendingLoads,
    load_state: &mut TerrainLoadState,
    stats: &mut TileLoadStats,
    budget: usize,
) {
    // 1. 把每个飞行中的 task 都 poll 一次；将已解析的结果移进 backlog。
    //    已完成的 task 在这里被 drain 且从不重新 poll，所以预算
    //    能推迟 GPU 上传而又不触及已完成的 `Task`。
    let mut resolved: Vec<(TileKey, Entity, Result<TerrainMesh, String>)> = Vec::new();
    for (key, load) in pending.pending.iter_mut() {
        if let Some(result) = block_on(poll_once(&mut load.task)) {
            resolved.push((*key, load.entity, result));
        }
    }
    for (key, entity, result) in resolved {
        // 安全：`key` 刚从 `pending.pending` 读出，没有任何东西移除它。
        pending.pending.remove(&key);
        pending.ready_backlog.push_back((key, entity, result));
    }

    // 2. 本帧至多上传 `budget` 个 mesh（FIFO）。
    let mut uploaded = 0;
    while uploaded < budget {
        let Some((key, entity, result)) = pending.ready_backlog.pop_front() else {
            break;
        };
        uploaded += 1;
        stats.tiles_pending = stats.tiles_pending.saturating_sub(1);

        match result {
            Ok(mesh) => {
                // `try_insert`：placeholder entity 可能在 task 飞行期间已被
                // 卸载（despawn）——一次普通 `insert` 会因 Bevy B0003 而
                // panic。镜像 tileset content_loader。
                commands.entity(entity).try_insert(TerrainTileReady {
                    terrain_mesh: Some(mesh),
                    state: TileContentState::Ready,
                });
                load_state.loaded_count += 1;
                stats.tiles_loaded += 1;
            }
            Err(e) => {
                // 优雅降级：warn + skip，从不 panic。
                warn!(
                    "Terrain tile ({},{},{}) failed to load/decode, skipping: {}",
                    key.0, key.1, key.2, e
                );
                commands.entity(entity).try_insert(TerrainTileReady {
                    terrain_mesh: None,
                    state: TileContentState::Failed,
                });
                load_state.failed_count += 1;
                stats.tiles_failed += 1;
            }
        }
    }
}

/// 逐帧驱动地形瓦片加载：排空已解析 task、按选择 spawn 新的下载。
///
/// # 参数
/// - `commands`：实体增删改命令
/// - `config`：地球配置（可选）
/// - `selection`：LOD 系统选出的待加载瓦片
/// - `pending`：飞行中的下载 task 集合
/// - `load_state`：加载计数状态
/// - `stats`：加载统计资源
/// - `terrain_query`：地形瓦片组件查询
pub fn terrain_tile_load_system(
    mut commands: Commands,
    config: Option<Res<GlobeConfig>>,
    mut selection: ResMut<TerrainSelection>,
    mut pending: ResMut<TerrainPendingLoads>,
    mut load_state: ResMut<TerrainLoadState>,
    mut stats: ResMut<TileLoadStats>,
    terrain_query: Query<(Entity, &CesiumTerrainTile)>,
) {
    // 门控：每帧读一次。ON 把 fetch 路由经由 cesium-pipeline
    // core 的共享 keep-alive ureq 池，并把上传约束到 terrain mesh
    // 权重；OFF 保留旧的每次调用 fetch，并 drain 所有已解析 tile。
    let use_pipeline = pipeline::fetch::pipeline_gate_enabled();
    let upload_budget = if use_pipeline {
        budget::terrain_mesh_budget()
    } else {
        budget::UNBOUNDED
    };

    // 退役自上一帧以来已解析的 task（Loading → Ready/Failed）。
    poll_pending_loads(
        &mut commands,
        &mut pending,
        &mut load_state,
        &mut stats,
        upload_budget,
    );

    let config = match config {
        Some(c) => c,
        None => return,
    };

    // HashMap<TileKey, Entity> 索引取代了之前对整个 query 逐 tile 的 O(n) 线性
    // `find`，后者使整个系统变成 O(n²)。
    let mut index: HashMap<TileKey, Entity> = HashMap::new();
    for (entity, tile) in terrain_query.iter() {
        index.insert((tile.x, tile.y, tile.level), entity);
    }

    let ellipsoid = config.ellipsoid;
    let pool = IoTaskPool::get();

    // loader 是 `tiles_to_load` 的唯一消费者；render 系统现在
    // 改为 scan `state == Ready`，而非 drain 同一个队列（竞态已消除）。
    for key in selection.tiles_to_load.drain(..) {
        let (x, y, level) = key;

        // 已加载/加载中（entity 已存在）或一个 task 已在飞行中。
        if index.contains_key(&key) || pending.pending.contains_key(&key) {
            stats.tiles_skipped += 1;
            continue;
        }

        let url = match build_terrain_url(&config, x, y, level) {
            Some(u) => u,
            None => {
                stats.tiles_skipped += 1;
                continue;
            }
        };

        // spawn 一个处于 `Loading` 状态的 placeholder entity，以便 LOD 系统将该
        // tile 视为已存在（无重复请求），且 loader 拥有它。
        let entity = commands
            .spawn((
                CesiumTerrainTile { x, y, level },
                TerrainTileReady {
                    terrain_mesh: None,
                    state: TileContentState::Loading,
                },
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        index.insert(key, entity);

        // 将异步下载 + 解码分派到 IO task 池（从不阻塞
        // 帧线程）。worker 只产出 CPU 侧的 `TerrainMesh`。
        let rect = tile_rectangle(x, y, level);
        let skirt_height = skirt_height_for_level(level);
        let task = pool.spawn(async move {
            load_and_decode_terrain(&url, &rect, &ellipsoid, skirt_height, use_pipeline)
        });

        pending.pending.insert(key, PendingLoad { entity, task });
        stats.tiles_pending += 1;
    }
}

#[derive(Component)]
pub struct TerrainTileReady {
    pub terrain_mesh: Option<cesium_terrain::TerrainMesh>,
    pub state: TileContentState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_terrain_url_xyz() {
        let config = GlobeConfig {
            terrain_provider_url: Some("https://tiles.example.com/{z}/{x}/{y}.terrain".into()),
            ..Default::default()
        };
        let url = build_terrain_url(&config, 3, 1, 2);
        assert_eq!(url.unwrap(), "https://tiles.example.com/2/3/1.terrain");
    }

    #[test]
    fn test_build_terrain_url_none() {
        let config = GlobeConfig::default();
        let url = build_terrain_url(&config, 0, 0, 0);
        assert!(url.is_none());
    }

    #[test]
    fn test_tile_rectangle_level_0() {
        let rect = tile_rectangle(0, 0, 0);
        assert!(rect.west <= rect.east);
        assert!(rect.south <= rect.north);
    }

    #[test]
    fn test_tile_rectangle_hemisphere() {
        let rect0 = tile_rectangle(0, 0, 1);
        let rect1 = tile_rectangle(1, 0, 1);
        assert!(rect0.east <= rect1.west + 1e-10);
    }

    #[test]
    fn test_terrain_load_state_default() {
        let state = TerrainLoadState::default();
        assert_eq!(state.loaded_count, 0);
        assert_eq!(state.failed_count, 0);
    }

    #[test]
    fn test_pending_loads_default() {
        let pending = TerrainPendingLoads::default();
        assert!(pending.pending.is_empty());
    }

    #[test]
    fn test_skirt_height_halves_per_level() {
        // 垂裙高度必须随 level 几何收缩（每 level ÷ 2）。
        let l0 = skirt_height_for_level(0);
        let l1 = skirt_height_for_level(1);
        let l2 = skirt_height_for_level(2);
        assert!(l0 > 0.0);
        assert!((l1 - l0 / 2.0).abs() < 1e-6);
        assert!((l2 - l0 / 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_skirt_height_high_level_no_overflow() {
        // 超过 32 的 level 会被钳位；helper 必须不溢出也不 panic。
        let h = skirt_height_for_level(64);
        assert!(h >= 0.0);
    }

    #[test]
    fn test_skirt_height_level0_is_physically_bounded() {
        // BLOCKER 回归：level-0 垂裙必须保持在一个物理上合理的
        // 范围（米/数百米），远低于地球半径
        //（~6.4e6 m），以便 add_skirts 中的 `carto.height - skirt` 从不
        // 将顶点折叠穿过地心。它之前约为 ~5e7 m。
        let l0 = skirt_height_for_level(0);
        assert!(l0 > 0.0);
        assert!(l0 < 5_000.0, "level-0 skirt {} m is unphysically large", l0);
        assert!(l0 <= MAX_SKIRT_HEIGHT);
    }
}
