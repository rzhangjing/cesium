//! 异步 3D Tiles content 加载（content -> mesh）。
//!
//! 逐 tile 流水线：
//! 1. `tileset_traversal_system` 对选择集做 diff，并把新路径推入
//!    `TileSelection::tiles_to_load`。
//! 2. 本系统是那个队列的*唯一*消费者：它 spawn 一个处于
//!    [`TileContentState::Loading`] 的占位 entity，并把下载 + 解码 +
//!    mesh 构建分派到 Bevy 的 [`IoTaskPool`]（从不阻塞帧线程）。
//! 3. 后续帧对每个 task 恰好轮询一次（`poll_once`），并把该占位
//!    entity 转换为 `Ready`（把 GPU handle 存入 [`TileContent`]）、
//!    `Failed`，或在格式不受支持时将其 despawn。
//!
//! 渲染系统改为扫描组件状态（`Ready` + `mesh_handle.is_some()`），
//! 而不再从同一个队列 drain，这在结构上消除了此前的
//! load/render drain 竞态。

use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::geometry::GeometryData;
use cesium_tileset::content_decoder::{
    decode_tile_content, detect_content_type, DecodedTile, TileContentType,
};

use crate::components::{CesiumTileNode, TileContent, TileContentState};
use crate::pipeline::{self, budget};
use crate::resources::TileLoadStats;

use super::loader::LoadedTileset;
use super::traversal_system::TileSelection;

/// 处于飞行中的异步 tile content 加载。
///
/// 以 tile 路径为键；每个条目持有被 spawn 的 [`IoTaskPool`] task，外加
/// 那个在 task 解析后接收解码 mesh 的占位 entity。
#[derive(Resource, Default)]
pub struct PendingTileLoads {
    pub pending: HashMap<Vec<usize>, TileLoadRequest>,
    /// 已被判定为不受支持、从而拒绝的 content URI，使一个被跳过的 tile 从不
    /// 在每帧被重新分派。
    pub skipped_uris: HashSet<String>,
    /// `skipped_uris` 集合所归属的 tileset URL。当活跃的
    /// [`LoadedTileset`] 切换到不同的 URL 时，该集合被清空，于是
    /// 前一个 tileset 的跳过决定从不泄漏进新的那个。
    pub tileset_url: String,
    /// 已解析但尚未上传的 content，按 FIFO 排空至每帧的
    /// tileset mesh 预算（门控 ON）。门控 OFF 时（budget = `UNBOUNDED`）
    /// 总是在帧内完全排空，因此旧行为保持不变。
    pub ready_backlog: VecDeque<ResolvedTileLoad>,
}

/// 一个其 worker task 已解析、正等待其每帧 mesh 上传名额的
/// content 加载。持有轮询步骤构建 GPU handle 并（重新）挂上
/// [`CesiumTileNode`] 所需的一切，无需再次借用领域 `Tile`。
pub struct ResolvedTileLoad {
    pub path: Vec<usize>,
    pub url: String,
    pub entity: Entity,
    pub meta: TileNodeMeta,
    pub payload: TileLoadPayload,
}

pub struct TileLoadRequest {
    pub path: Vec<usize>,
    pub url: String,
    /// 以 `Loading` 状态 spawn 的占位 entity。
    pub entity: Entity,
    /// 在分派时捕获的 tile 元数据（领域 `Tile` 的借用无法
    /// 比帧活得更久，而 task 在更后的一帧才解析）。
    pub meta: TileNodeMeta,
    /// 后台下载 + 解码 + mesh 构建。
    pub task: Task<TileLoadPayload>,
}

/// （重新）构建 [`CesiumTileNode`] 所需的不可变 tile 元数据。
#[derive(Debug, Clone, Copy)]
pub struct TileNodeMeta {
    pub geometric_error: f64,
    pub screen_space_error: f64,
    /// tile 包围体中心，ECEF 米（f64）—— 同时也是 RTC 中心。
    pub bounding_center: glam::DVec3,
    pub bounding_radius: f64,
}

/// Worker 结果：仅 CPU 侧数据。GPU handle 在帧线程上创建，
/// 因为 `Assets<T>` 无法从 task-pool worker 访问。
pub struct TileLoadPayload {
    pub bytes_downloaded: u64,
    pub outcome: TileLoadOutcome,
}

pub enum TileLoadOutcome {
    /// 解码成功；mesh 已以 RTC 中心重新居中。
    Ready(PreparedTileContent),
    /// 不受支持的格式（Draco / pnts / cmpt / subt / 外部 i3dm URI）：
    /// warn + 计为 skipped，绝不计为 failure。
    Skipped(String),
    /// 真正的失败（网络错误、畸形 payload）：计为 failed。
    Failed(String),
}

pub struct PreparedTileContent {
    pub mesh: Mesh,
    pub has_batch_table: bool,
}

/// 区分"不受支持，优雅降级"与"损坏，需上报"。
#[derive(Debug)]
enum ContentError {
    Unsupported(String),
    Invalid(String),
}

impl ContentError {
    /// 构造一个“不受支持”变体（优雅降级，不计为硬失败）。
    fn unsupported(msg: impl Into<String>) -> Self {
        ContentError::Unsupported(msg.into())
    }

    /// 构造一个“格式非法”变体（网络/解码错误，计为失败）。
    fn invalid(msg: impl Into<String>) -> Self {
        ContentError::Invalid(msg.into())
    }
}

/// 从原始 payload 解码出的内嵌 GLB 及其批量表标志。
struct DecodedGlb {
    /// 内嵌的 GLB 字节（已从容器剥离）。
    glb: Vec<u8>,
    /// 是否携带 batch table（供后续拾取/样式使用）。
    has_batch_table: bool,
}

/// 下载原始 tile 字节。
///
/// 阻塞式；面向一个 [`IoTaskPool`] worker 线程而设计。该 fetch 不依赖 tokio ——
/// 它经由 cesium-pipeline core 的 ureq 阻塞后端（[`pipeline::fetch::fetch_gated`]），
/// 选择共享的 keep-alive 池（门控 ON）或每次调用新建的客户端（门控 OFF）。
fn fetch_tile_bytes(url: &str, use_pipeline: bool) -> Result<Vec<u8>, String> {
    pipeline::fetch::fetch_gated(url, use_pipeline)
}

/// 按 magic bytes 分类 payload，然后只解码受支持的格式。
///
/// 分类发生在解码*之前*：`decode_tile_content` 对例如 `subt`/`geom`/`vctr`
/// 会报 `InvalidMagic`，而它们否则会被误计为一次硬失败，而非一次优雅跳过。
fn extract_glb(raw: &[u8]) -> Result<DecodedGlb, ContentError> {
    let content_type = detect_content_type(raw);
    match content_type {
        // 如今只有 b3dm 与裸 GLB 可渲染。i3dm（instanced）在此处以及
        // 下面都被刻意排除：在分类卡口拒绝它，保证*每一个* i3dm 都成为一次
        // 优雅跳过（`tiles_skipped`），而不会冒着某个畸形实例的解码被
        // 误计为一次硬失败的风险。
        TileContentType::Batched3DModel | TileContentType::GltfBinary => {}
        other => {
            return Err(ContentError::unsupported(format!(
                "content format {:?} is not supported yet",
                other
            )))
        }
    }

    let decoded =
        decode_tile_content(raw).map_err(|e| ContentError::invalid(format!("decode: {}", e)))?;

    match decoded {
        DecodedTile::B3dm(b3dm) => Ok(DecodedGlb {
            has_batch_table: b3dm.batch_table_json.is_some() || !b3dm.batch_table_binary.is_empty(),
            glb: b3dm.gltf,
        }),
        // i3dm（instanced）不受支持：无论内嵌 GLB（`gltf_format == 1`）
        // 还是外部 URI（`== 0`）变体皆是如此。只画一次原型会是一个错误的
        // 半态（在 RTC 中心放一份拷贝、每个逐实例变换被忽略，却仍计为已加载）。
        // 上面的分类卡口已经拒绝 i3dm，所以这一分支只是一个防御性的
        // 穷尽兜底 —— 它必须始终是跳过，绝不可渲染。
        DecodedTile::I3dm(_) => Err(ContentError::unsupported(
            "i3dm instancing is not supported yet",
        )),
        DecodedTile::Glb(glb) => Ok(DecodedGlb {
            glb,
            has_batch_table: false,
        }),
        DecodedTile::Pnts(_) => Err(ContentError::unsupported(
            "pnts point clouds are not supported yet",
        )),
        DecodedTile::Cmpt(_) => Err(ContentError::unsupported(
            "cmpt composite tiles are not supported yet",
        )),
    }
}

/// 完整的 CPU 侧 worker：fetch -> 分类/解码 -> 解析 glTF -> 在 f32 转换
/// 之前于 f64 中减去 RTC 中心来构建 Bevy mesh。
///
/// 运行于一个 [`IoTaskPool`] worker 线程；只返回 CPU 侧数据。
fn load_tile_content(url: &str, rtc_center: glam::DVec3, use_pipeline: bool) -> TileLoadPayload {
    let raw = match fetch_tile_bytes(url, use_pipeline) {
        Ok(bytes) => bytes,
        Err(e) => {
            return TileLoadPayload {
                bytes_downloaded: 0,
                outcome: TileLoadOutcome::Failed(e),
            }
        }
    };
    let bytes_downloaded = raw.len() as u64;

    let outcome = match extract_glb(&raw)
        .and_then(|decoded| parse_glb_to_geometry(&decoded.glb).map(|g| (g, decoded.has_batch_table)))
    {
        Ok((geometry, has_batch_table)) => TileLoadOutcome::Ready(PreparedTileContent {
            mesh: crate::geometry_to_mesh(&geometry, Some(rtc_center)),
            has_batch_table,
        }),
        Err(ContentError::Unsupported(reason)) => TileLoadOutcome::Skipped(reason),
        Err(ContentError::Invalid(reason)) => TileLoadOutcome::Failed(reason),
    };

    TileLoadPayload {
        bytes_downloaded,
        outcome,
    }
}

/// 在各 tile 的材质/贴图接入之前的一个中性材质。
fn default_tile_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.8, 0.8, 0.8),
        ..default()
    }
}

/// 由路径、瓦片元信息与目标状态拼装一个 [`CesiumTileNode`] 组件。
///
/// # 参数
/// - `path`：从根到瓦片的子索引路径
/// - `meta`：瓦片的误差与包围球元信息
/// - `state`：当前内容状态（Loading/Ready/Failed）
fn tile_node(path: Vec<usize>, meta: TileNodeMeta, state: TileContentState) -> CesiumTileNode {
    CesiumTileNode {
        path,
        screen_space_error: meta.screen_space_error,
        geometric_error: meta.geometric_error,
        state,
        bounding_sphere_center: Some(meta.bounding_center),
        bounding_sphere_radius: Some(meta.bounding_radius),
    }
}

/// 轮询飞行中的 task（每个一次，使帧线程从不停车），并把已解析的
/// tile 转换 `Loading -> Ready/Failed`，或为那个 content 格式不受支持的 tile
/// despawn 其占位 entity。
///
/// 已解析的 payload 被移入一个 FIFO backlog，每帧最多上传 `budget` 个
/// mesh —— `UNBOUNDED`（门控 OFF）在帧内排空一切，恰如迁移前的代码，
/// 而门控 ON 把 GPU 工作约束到 tileset mesh 权重
/// （[`budget::tileset_mesh_budget`]）。
fn poll_pending_loads(
    commands: &mut Commands,
    pending_loads: &mut PendingTileLoads,
    stats: &mut TileLoadStats,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    budget: usize,
) {
    // 1. 对每个飞行中的 task 轮询一次；把已解析的 payload 移入 backlog。
    //    一个完成的 task 在此被 drain 且从不重轮，所以预算可以在不触碰一个
    //    已完成 `Task` 的前提下推迟 GPU 上传。
    let mut resolved: Vec<(Vec<usize>, TileLoadPayload)> = Vec::new();
    for (path, request) in pending_loads.pending.iter_mut() {
        if let Some(payload) = block_on(poll_once(&mut request.task)) {
            resolved.push((path.clone(), payload));
        }
    }
    for (path, payload) in resolved {
        // 安全：`path` 刚从 `pending` 读出，且没有任何东西移除它。
        let request = pending_loads
            .pending
            .remove(&path)
            .expect("pending tile load vanished during poll");
        pending_loads.ready_backlog.push_back(ResolvedTileLoad {
            path: request.path,
            url: request.url,
            entity: request.entity,
            meta: request.meta,
            payload,
        });
    }

    // 2. 本帧最多上传 `budget` 个 mesh（FIFO）。
    let mut uploaded = 0;
    while uploaded < budget {
        let Some(resolved_load) = pending_loads.ready_backlog.pop_front() else {
            break;
        };
        uploaded += 1;
        let ResolvedTileLoad {
            path,
            url,
            entity,
            meta,
            payload,
        } = resolved_load;

        stats.tiles_pending = stats.tiles_pending.saturating_sub(1);
        stats.bytes_downloaded += payload.bytes_downloaded;

        match payload.outcome {
            TileLoadOutcome::Ready(content) => {
                let mesh_handle = meshes.add(content.mesh);
                let material_handle = materials.add(default_tile_material());

                // `try_insert`：该 tile 可能在飞行途中被卸载。
                commands.entity(entity).try_insert((
                    TileContent {
                        mesh_handle: Some(mesh_handle),
                        material_handle: Some(material_handle),
                        has_batch_table: content.has_batch_table,
                    },
                    tile_node(path, meta, TileContentState::Ready),
                ));
                stats.tiles_loaded += 1;
            }
            TileLoadOutcome::Skipped(reason) => {
                // 优雅降级：warn + skip，绝不失败或 panic。
                warn!("Skipping tile {:?} ({}): {}", path, url, reason);
                pending_loads.skipped_uris.insert(url);
                commands.entity(entity).try_despawn();
                stats.tiles_skipped += 1;
            }
            TileLoadOutcome::Failed(reason) => {
                error!("Failed to load tile {:?} ({}): {}", path, url, reason);
                commands
                    .entity(entity)
                    .try_insert(tile_node(path, meta, TileContentState::Failed));
                stats.tiles_failed += 1;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// 逐帧驱动瓦片内容加载：排空已解析的 task、按预算 spawn 新的下载、更新统计。
///
/// # 参数
/// - `commands`：实体增删改命令
/// - `loaded`：已加载的 tileset 根信息（可选）
/// - `selection`：遍历系统选出的待加载/卸载瓦片
/// - `pending_loads`：飞行中的下载 task 集合
/// - `stats`：加载统计资源
/// - `meshes`/`materials`：资产写入器
/// - `tile_query`：瓦片节点查询
pub fn tile_content_load_system(
    mut commands: Commands,
    loaded: Option<Res<LoadedTileset>>,
    selection: ResMut<TileSelection>,
    mut pending_loads: ResMut<PendingTileLoads>,
    mut stats: ResMut<TileLoadStats>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tile_query: Query<&CesiumTileNode>,
) {
    // 门控：每帧读一次。ON 把 fetch 路由经 cesium-pipeline core 的共享
    // keep-alive ureq 池，并把上传约束到 tileset mesh 权重；OFF 保留旧的
    // 每次调用 fetch，并排空所有已解析 tile。
    let use_pipeline = pipeline::fetch::pipeline_gate_enabled();
    let upload_budget = if use_pipeline {
        budget::tileset_mesh_budget()
    } else {
        budget::UNBOUNDED
    };

    // 退役自上一帧以来已解析的 task。
    poll_pending_loads(
        &mut commands,
        &mut pending_loads,
        &mut stats,
        &mut meshes,
        &mut materials,
        upload_budget,
    );

    let loaded = match loaded {
        Some(l) => l,
        None => return,
    };

    let tileset_json = match &loaded.tileset_json {
        Some(ts) => ts,
        None => return,
    };

    // Tileset 切换：丢弃那些属于另一个 tileset 的跳过决定，使它们
    // 从不压制新激活那个里的 content。
    if pending_loads.tileset_url != loaded.url {
        pending_loads.skipped_uris.clear();
        pending_loads.tileset_url = loaded.url.clone();
    }

    // 对同一个资源做互不相交的字段借用：一边 drain `tiles_to_load`，
    // 一边读 `selected_tiles` 以取逐 tile 的 screen space error。`into_inner`
    // 消费那个 `ResMut` 并标记 change-detection tick。
    let sel = selection.into_inner();

    let ellipsoid = Ellipsoid::WGS84;
    let pool = IoTaskPool::get();

    // 已存在的 tile node 的路径索引。对每个被请求的 tile 都扫描一次 query 曾是
    // O(n*m)；这同时保证每个路径恰有一个 entity。
    let mut known_paths: HashSet<Vec<usize>> = tile_query
        .iter()
        .map(|node| node.path.clone())
        .collect();

    for path in sel.tiles_to_load.drain(..) {
        // 在更早的帧里已 loaded/loading/failed，或此刻正在飞行。
        // 计为一次 skip，以匹配 terrain loader 对一个重复键的记账
        // （该请求是被刻意地不重新分派）。
        if known_paths.contains(&path) || pending_loads.pending.contains_key(&path) {
            stats.tiles_skipped += 1;
            continue;
        }

        let tile = match cesium_tileset::lod_selection::get_tile_by_path(&tileset_json.root, &path) {
            Some(t) => t,
            // 路径不在此 tileset 中（过期选择）—— 无可加载之物。
            None => continue,
        };

        // 纯 group tile 自身不携带任何可渲染的 content。
        let uri = match tile.content_uris().first() {
            Some(u) => (*u).to_string(),
            None => continue,
        };

        let url = loaded.state.resolve_uri(&uri);

        // 已知不受支持的 content：不要每帧重新分派它。出于与重复路径
        // 守卫相同的记账原因，计为一次 skip。
        if pending_loads.skipped_uris.contains(&url) {
            stats.tiles_skipped += 1;
            continue;
        }

        let bounding = tile.bounding_volume.to_bounding_sphere(&ellipsoid);

        let screen_space_error = sel
            .selected_tiles
            .iter()
            .find(|t| t.path == path)
            .map(|t| t.screen_space_error)
            .unwrap_or(0.0);

        let meta = TileNodeMeta {
            geometric_error: tile.geometric_error,
            screen_space_error,
            bounding_center: bounding.center,
            bounding_radius: bounding.radius,
        };

        // 处于 `Loading` 状态的占位；渲染系统只为那些真正携带 mesh handle
        // 的 `Ready` node spawn mesh。
        let entity = commands
            .spawn((
                tile_node(path.clone(), meta, TileContentState::Loading),
                TileContent {
                    mesh_handle: None,
                    material_handle: None,
                    has_batch_table: false,
                },
                Transform::default(),
                Visibility::default(),
            ))
            .id();
        known_paths.insert(path.clone());

        // RTC 中心 = tile 包围体中心（ECEF 米，f64）。顶点在 f32 转换之前
        // 于 f64 中以它重新居中，而渲染 entity 被放回 `center / METERS_PER_RENDER_UNIT`。
        let rtc_center = bounding.center;
        let task_url = url.clone();
        let task = pool.spawn(async move { load_tile_content(&task_url, rtc_center, use_pipeline) });

        pending_loads.pending.insert(
            path.clone(),
            TileLoadRequest {
                path,
                url,
                entity,
                meta,
                task,
            },
        );
        stats.tiles_pending += 1;
    }
}

/// `CESIUM_ENABLE_GLTF_UPGRADE` —— glTF 1.0 → 2.0 升级门控。名称与
/// cesium-app feature-flag 注册表（`feature_flags::ENV_ENABLE_GLTF_UPGRADE`）一致，
/// 使两条读取路径观察到完全相同的环境变量。bevy-render 不依赖应用层，
/// 所以门控在此经由 M1.4 的
/// [`pipeline::fetch::gate_from_env_value`] 先例求值（默认 OFF）。
const ENV_ENABLE_GLTF_UPGRADE: &str = "CESIUM_ENABLE_GLTF_UPGRADE";

/// 在不依赖应用层的前提下读取 glTF-upgrade 门控。默认 OFF，
/// 所以除非 `CESIUM_ENABLE_GLTF_UPGRADE` 显式为真，现有 glTF 2.0 渲染黄金
/// 路径都逐字节保持不变。
fn gltf_upgrade_gate_enabled() -> bool {
    pipeline::fetch::gate_from_env_value(std::env::var(ENV_ENABLE_GLTF_UPGRADE).ok())
}

/// 把一个内嵌 GLB 解码为带类型的 [`cesium_gltf::gltf_model::GltfModel`]，
/// 外加它的逐 buffer 字节源（`buffers[i]` 是 `gltf.buffers[i]` 的解码源；
/// 对一个 GLB，`buffers[0]` 是内嵌的二进制 chunk）。
///
/// 门控 OFF（`upgrade_enabled == false`）：直接委派给
/// [`GlbData::from_bytes`] —— 与迁移前 M9.2 的路径**逐字节一致**，
/// 保护现有的 glTF 2.0 渲染黄金路径。
///
/// 门控 ON：把容器*非类型化*地解析（[`parse_glb_container`]）；一个尚不是
/// 2.0 的 payload 会先经过 [`detect_version`] + [`update_version_with_buffers`]
/// （把内嵌二进制 chunk 作为 `buffers[0]` 穿线以驱动 M9.2 的 binary 阶段），
/// 再进入带类型的 [`GltfModel::from_value`] 反序列化。一个已是 2.0 的 payload
/// 是纯粹的透传（零改动），所以即便门控 ON 也从不扰动一个有效的 2.0 资源。
fn decode_gltf_model(
    glb: &[u8],
    upgrade_enabled: bool,
) -> Result<(cesium_gltf::gltf_model::GltfModel, Vec<Vec<u8>>), ContentError> {
    if !upgrade_enabled {
        let glb_data = cesium_gltf::binary_format::GlbData::from_bytes(glb)
            .map_err(|e| ContentError::invalid(format!("GLB parse: {}", e)))?;
        // GLB buffer 0 是内嵌的二进制 chunk。
        let buffers = vec![glb_data.binary_chunk.unwrap_or_default()];
        return Ok((glb_data.model, buffers));
    }

    let (mut value, binary_chunk) = cesium_gltf::binary_format::parse_glb_container(glb)
        .map_err(|e| ContentError::invalid(format!("GLB parse: {}", e)))?;
    let mut buffers: Vec<Vec<u8>> = vec![binary_chunk.unwrap_or_default()];

    if cesium_gltf::gltf_upgrade::detect_version(&value)
        != cesium_gltf::gltf_upgrade::GltfVersion::V20
    {
        cesium_gltf::gltf_upgrade::update_version_with_buffers(
            &mut value,
            &cesium_gltf::gltf_upgrade::UpgradeOptions::default(),
            &mut buffers,
        )
        .map_err(|e| ContentError::invalid(format!("glTF upgrade: {}", e)))?;
    }

    let model = cesium_gltf::gltf_model::GltfModel::from_value(value)
        .map_err(|e| ContentError::invalid(format!("glTF parse: {}", e)))?;
    Ok((model, buffers))
}

/// 把一个内嵌 GLB 解析为 CPU 侧几何。
///
/// chunk 遍历委派给领域解析器（12 字节头，随后是 `length|type|data`
/// chunk）。此前手写的切片从第 16 字节而非第 20 字节读 JSON chunk，
/// 也就是它试图把 4 字节的 chunk type 当 JSON 解析，因而在每个真实
/// payload 上都失败 —— 没有任何 tile 能产出一个 mesh。
///
/// glTF 1.0 → 2.0 升级路径由 `CESIUM_ENABLE_GLTF_UPGRADE` 门控选择
/// （见 [`decode_gltf_model`]）并默认 OFF，使现有的 2.0 路径逐字节保持不变。
///
/// Draco 压缩的 payload 被作为 `Unsupported` 拒绝（解码器后端仍是一个
/// stub），于是调用方优雅降级，而不去吐出一个空/垃圾 mesh。
fn parse_glb_to_geometry(glb: &[u8]) -> Result<GeometryData, ContentError> {
    parse_glb_to_geometry_gated(glb, gltf_upgrade_gate_enabled())
}

/// [`parse_glb_to_geometry`] 的可测试接缝：`upgrade_enabled` 直接选择门控
/// ON/OFF 解码路径，而不去改动进程全局环境（那会与并行测试竞态）。
fn parse_glb_to_geometry_gated(
    glb: &[u8],
    upgrade_enabled: bool,
) -> Result<GeometryData, ContentError> {
    let (gltf, buffers) = decode_gltf_model(glb, upgrade_enabled)?;

    if gltf
        .extensions_required
        .iter()
        .any(|ext| ext == "KHR_draco_mesh_compression")
    {
        return Err(ContentError::unsupported(
            "KHR_draco_mesh_compression (Draco) is not supported yet",
        ));
    }

    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut normals: Vec<[f64; 3]> = Vec::new();
    let mut tex_coords: Vec<[f64; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut index_offset: u32 = 0;

    for mesh in &gltf.meshes {
        for prim in &mesh.primitives {
            let pos_count = prim
                .attributes
                .get("POSITION")
                .and_then(|&idx| gltf.accessors.get(idx))
                .map(|a| a.count)
                .unwrap_or(0);

            if let Some(&pos_idx) = prim.attributes.get("POSITION") {
                if let Some(acc) = gltf.accessors.get(pos_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(3) {
                        positions.push([chunk[0] as f64, chunk[1] as f64, chunk[2] as f64]);
                    }
                }
            }

            if let Some(&nrm_idx) = prim.attributes.get("NORMAL") {
                if let Some(acc) = gltf.accessors.get(nrm_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(3) {
                        normals.push([chunk[0] as f64, chunk[1] as f64, chunk[2] as f64]);
                    }
                }
            }

            if let Some(&uv_idx) = prim.attributes.get("TEXCOORD_0") {
                if let Some(acc) = gltf.accessors.get(uv_idx) {
                    let float_data = acc.read_f32_data(&buffers, &gltf.buffer_views);
                    for chunk in float_data.chunks_exact(2) {
                        tex_coords.push([chunk[0] as f64, chunk[1] as f64]);
                    }
                }
            }

            if let Some(idx_idx) = prim.indices {
                if let Some(acc) = gltf.accessors.get(idx_idx) {
                    let use_u16 = matches!(
                        acc.component_type,
                        cesium_gltf::gltf_model::ComponentType::U16
                    );
                    if use_u16 {
                        let idx_data = acc.read_u16_data(&buffers, &gltf.buffer_views);
                        for idx in &idx_data {
                            indices.push(*idx as u32 + index_offset);
                        }
                    } else {
                        let idx_data = acc.read_u32_data(&buffers, &gltf.buffer_views);
                        for idx in &idx_data {
                            indices.push(*idx + index_offset);
                        }
                    }
                }
            }

            index_offset += pos_count as u32;
        }
    }

    if positions.is_empty() {
        return Err(ContentError::invalid("No vertex positions found"));
    }

    let bounding_sphere = cesium_geospatial::bounding::BoundingSphere::from_points(
        &positions
            .iter()
            .map(|p| glam::DVec3::new(p[0], p[1], p[2]))
            .collect::<Vec<_>>(),
    );

    Ok(GeometryData {
        positions,
        normals: if normals.is_empty() { None } else { Some(normals) },
        tex_coords: if tex_coords.is_empty() {
            None
        } else {
            Some(tex_coords)
        },
        tangents: None,
        bitangents: None,
        indices,
        bounding_sphere,
        primitive_type: cesium_geospatial::geometry::PrimitiveType::Triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构建一个最小的 GLB 容器来包裹 `json`（4 字节对齐，无 BIN
    /// chunk），使 header/JSON 解析路径无需文件即可被演练。
    fn build_glb(json: &str) -> Vec<u8> {
        build_glb_with_bin(json, &[])
    }

    /// 同 [`build_glb`]，另当 `bin` 非空时追加一个尾随 BIN chunk。
    fn build_glb_with_bin(json: &str, bin: &[u8]) -> Vec<u8> {
        let mut json_chunk = json.as_bytes().to_vec();
        // GLB chunk 是 4 字节对齐的；JSON 用空格（0x20）填充。
        let json_pad = (4 - json_chunk.len() % 4) % 4;
        json_chunk.resize(json_chunk.len() + json_pad, b' ');

        let mut bin_chunk = bin.to_vec();
        // BIN chunk 也是 4 字节对齐的，用零填充。
        let bin_pad = (4 - bin_chunk.len() % 4) % 4;
        bin_chunk.resize(bin_chunk.len() + bin_pad, 0);

        let bin_total = if bin_chunk.is_empty() {
            0
        } else {
            8 + bin_chunk.len()
        };
        let total = 12 + 8 + json_chunk.len() + bin_total;

        let mut glb = Vec::with_capacity(total);
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&(total as u32).to_le_bytes());
        glb.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
        glb.extend_from_slice(&0x4E4F534Au32.to_le_bytes()); // "JSON"
        glb.extend_from_slice(&json_chunk);

        if !bin_chunk.is_empty() {
            glb.extend_from_slice(&(bin_chunk.len() as u32).to_le_bytes());
            glb.extend_from_slice(&0x004E4942u32.to_le_bytes()); // "BIN\0"
            glb.extend_from_slice(&bin_chunk);
        }

        glb
    }

    #[test]
    fn test_detect_content_types() {
        assert_eq!(
            detect_content_type(b"b3dm...."),
            TileContentType::Batched3DModel
        );
        assert_eq!(
            detect_content_type(b"i3dm...."),
            TileContentType::Instanced3DModel
        );
        assert_eq!(
            detect_content_type(b"glTF...."),
            TileContentType::GltfBinary
        );
        assert_eq!(detect_content_type(b"xxxx...."), TileContentType::Unknown);
    }

    #[test]
    fn test_decode_b3dm_to_gltf() {
        let gltf_content = b"glTF test glb data here".to_vec();
        let b3dm = cesium_tileset::content_decoder::B3dmContent {
            batch_length: 1,
            feature_table_json: None,
            feature_table_binary: vec![],
            batch_table_json: None,
            batch_table_binary: vec![],
            gltf: gltf_content.clone(),
        };
        let decoded = DecodedTile::B3dm(b3dm);
        match decoded {
            DecodedTile::B3dm(c) => assert_eq!(c.gltf, gltf_content),
            _ => panic!("Expected B3dm"),
        }
    }

    #[test]
    fn test_pending_loads_default() {
        let pending = PendingTileLoads::default();
        assert!(pending.pending.is_empty());
        assert!(pending.skipped_uris.is_empty());
    }

    #[test]
    fn test_parse_glb_rejects_draco() {
        // 一个被必需的 Draco 扩展必须降级为 `Unsupported`（warn + skip），
        // 绝不降级为一次硬失败或一个空 mesh。
        let glb = build_glb(
            r#"{"asset":{"version":"2.0"},"extensionsRequired":["KHR_draco_mesh_compression"]}"#,
        );

        match parse_glb_to_geometry(&glb) {
            Err(ContentError::Unsupported(reason)) => {
                assert!(reason.contains("Draco"), "unexpected reason: {}", reason);
            }
            other => panic!("expected Unsupported(Draco), got {:?}", other.err()),
        }
    }

    #[test]
    fn test_parse_glb_accepts_plain_gltf() {
        // 去掉该 Draco 要求的同一容器能通过扩展卡口
        // （它随后只因这个 fixture 没有顶点数据而在更后面失败）。
        let glb = build_glb(r#"{"asset":{"version":"2.0"}}"#);

        match parse_glb_to_geometry(&glb) {
            Err(ContentError::Invalid(_)) => {}
            Err(ContentError::Unsupported(reason)) => {
                panic!("plain glTF must not be treated as unsupported: {}", reason)
            }
            Ok(_) => panic!("fixture has no positions"),
        }
    }

    #[test]
    fn test_parse_glb_reads_positions_from_binary_chunk() {
        // 守卫 GLB chunk 布局修复：对一个符合规范的容器，JSON chunk
        // 必须能在第 20 字节处被找到、且 BIN chunk 紧随其后，
        // 于是那一个三角形才被真正解码。
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut bin: Vec<u8> = Vec::new();
        for p in positions {
            bin.extend_from_slice(&p.to_le_bytes());
        }

        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":36}],
            "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],
            "accessors":[{"bufferView":0,"byteOffset":0,"componentType":5126,
                "count":3,"type":"VEC3"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}]}"#;

        let geometry = parse_glb_to_geometry(&build_glb_with_bin(json, &bin))
            .expect("a plain GLB triangle must decode");
        assert_eq!(geometry.positions.len(), 3);
        assert_eq!(geometry.positions[1], [1.0, 0.0, 0.0]);
    }

    /// M9.2-b：一个 glTF **1.0** payload（以对象为键的集合）被内嵌在一个
    /// GLB v2 容器里。门控 OFF 必须失败 —— 基于数组的带类型模型无法
    /// 反序列化以对象为键的字典，这证明旧路径未被改动且 1.0 从未被静默支持。
    /// 门控 ON 必须运行 `detect_version` + `update_version_with_buffers`
    /// （对象→数组、字符串 id→索引、min/max 取自二进制 chunk）并解码该三角形。
    #[test]
    fn test_parse_glb_upgrades_gltf_10_only_when_gate_on() {
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut bin: Vec<u8> = Vec::new();
        for p in positions {
            bin.extend_from_slice(&p.to_le_bytes());
        }

        // glTF 1.0：每个顶层集合都是一个以字符串 id 为键的对象，
        // 且引用（bufferView / buffer / POSITION）也是字符串 id。
        let json = r#"{"asset":{"version":"1.0"},
            "buffers":{"buf":{"byteLength":36}},
            "bufferViews":{"bv":{"buffer":"buf","byteOffset":0,"byteLength":36}},
            "accessors":{"acc":{"bufferView":"bv","byteOffset":0,
                "componentType":5126,"count":3,"type":"VEC3"}},
            "meshes":{"mesh":{"primitives":[{"attributes":{"POSITION":"acc"}}]}}}"#;
        let glb = build_glb_with_bin(json, &bin);

        // 门控 OFF：逐字即迁移前 M9.2 路径 → 对以对象为键的 1.0 集合的
        // 带类型反序列化失败。
        assert!(
            parse_glb_to_geometry_gated(&glb, false).is_err(),
            "gate OFF must not decode a glTF 1.0 payload"
        );

        // 门控 ON：1.0 → 2.0 升级随后带类型解码。
        let geometry = parse_glb_to_geometry_gated(&glb, true)
            .expect("gate ON must upgrade a glTF 1.0 payload to 2.0");
        assert_eq!(geometry.positions.len(), 3);
        assert_eq!(geometry.positions[1], [1.0, 0.0, 0.0]);
        assert_eq!(geometry.positions[2], [0.0, 1.0, 0.0]);
    }

    /// M9.2-b 门控中立性：对一个已是 2.0 的 GLB，门控 ON 路径是一个纯粹
    /// 透传（`detect_version == V20` ⇒ 跳过 `update_version`），所以它必须产出
    /// 与门控 OFF（旧）路径逐字节相同的几何。几何 → mesh → 像素是确定的，
    /// 因而相同的几何就是现有 glTF 2.0 渲染路径上像素零差的 CPU 侧证据
    /// （完整的 GPU 截图 diff 在 xvfb 无头 e2e CI 中运行）。
    #[test]
    fn test_parse_glb_gate_on_is_passthrough_for_gltf_20() {
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let mut bin: Vec<u8> = Vec::new();
        for p in positions {
            bin.extend_from_slice(&p.to_le_bytes());
        }
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":36}],
            "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],
            "accessors":[{"bufferView":0,"byteOffset":0,"componentType":5126,
                "count":3,"type":"VEC3"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}]}"#;
        let glb = build_glb_with_bin(json, &bin);

        let off = parse_glb_to_geometry_gated(&glb, false).expect("gate OFF decodes 2.0");
        let on = parse_glb_to_geometry_gated(&glb, true).expect("gate ON passthrough decodes 2.0");

        assert_eq!(
            off.positions, on.positions,
            "gate ON must not perturb 2.0 positions"
        );
        assert_eq!(
            off.indices, on.indices,
            "gate ON must not perturb 2.0 indices"
        );
        assert_eq!(off.normals.is_none(), on.normals.is_none());
        assert_eq!(off.tex_coords.is_none(), on.tex_coords.is_none());
    }

    #[test]
    fn test_extract_glb_skips_unsupported_formats() {
        // pnts / cmpt / subt / unknown magic 被 skip，而非 failed。
        for magic in [b"pnts....", b"cmpt....", b"subt....", b"geom....", b"xxxx...."] {
            match extract_glb(magic) {
                Err(ContentError::Unsupported(_)) => {}
                other => panic!(
                    "expected Unsupported for {:?}, got {:?}",
                    &magic[0..4],
                    other.err()
                ),
            }
        }
    }

    #[test]
    fn test_extract_glb_accepts_glb_magic() {
        let glb = build_glb(r#"{"asset":{"version":"2.0"}}"#);
        let decoded = extract_glb(&glb).expect("glTF magic must be accepted");
        assert!(!decoded.has_batch_table);
        assert_eq!(decoded.glb, glb);
    }

    /// 把一个 GLB body 包进一个最小的 b3dm 容器：28 字节头（magic、
    /// version 1、byteLength，随后四个零表长）后接该 GLB。
    fn build_b3dm(glb: &[u8]) -> Vec<u8> {
        let total = 28 + glb.len();
        let mut b3dm = Vec::with_capacity(total);
        b3dm.extend_from_slice(b"b3dm");
        b3dm.extend_from_slice(&1u32.to_le_bytes()); // version
        b3dm.extend_from_slice(&(total as u32).to_le_bytes()); // byteLength
        b3dm.extend_from_slice(&0u32.to_le_bytes()); // featureTableJSONByteLength
        b3dm.extend_from_slice(&0u32.to_le_bytes()); // featureTableBinaryByteLength
        b3dm.extend_from_slice(&0u32.to_le_bytes()); // batchTableJSONByteLength
        b3dm.extend_from_slice(&0u32.to_le_bytes()); // batchTableBinaryByteLength
        b3dm.extend_from_slice(glb);
        b3dm
    }

    /// 为带类型的 `BufferTarget` 反序列化提供一个始终开启的端到端守卫。
    ///
    /// 一个合成的 b3dm，其内嵌 GLB 把 `bufferView.target` 声明为 OpenGL
    /// 整数 34962/34963。修复前，派生的字符串变体 serde impl 会拒绝这些，而
    /// 在严格性改动之后，一个*未知*整数必须报错而非静默降级。两种行为
    /// 都不依赖外部样例文件，所以它在每台 CI 机器上都运行。
    #[test]
    fn test_parse_synthetic_b3dm_numeric_buffer_target() {
        // 3 个 position（36 字节）随后 3 个 u16 index（6 字节），填充到 44。
        let positions: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let indices: [u16; 3] = [0, 1, 2];
        let mut bin: Vec<u8> = Vec::new();
        for p in positions {
            bin.extend_from_slice(&p.to_le_bytes());
        }
        for i in indices {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        bin.resize(44, 0);

        // 两个 buffer view 都携带一个数值型 `target`（34962 / 34963）。
        let json = r#"{"asset":{"version":"2.0"},"buffers":[{"byteLength":44}],
            "bufferViews":[
                {"buffer":0,"byteOffset":0,"byteLength":36,"target":34962},
                {"buffer":0,"byteOffset":36,"byteLength":6,"target":34963}],
            "accessors":[
                {"bufferView":0,"byteOffset":0,"componentType":5126,"count":3,"type":"VEC3"},
                {"bufferView":1,"byteOffset":0,"componentType":5123,"count":3,"type":"SCALAR"}],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1}]}]}"#;

        let b3dm = build_b3dm(&build_glb_with_bin(json, &bin));
        let decoded = extract_glb(&b3dm).expect("synthetic b3dm must extract a GLB");
        let geometry = parse_glb_to_geometry(&decoded.glb)
            .expect("numeric bufferView.target (34962/34963) must deserialize");
        assert_eq!(geometry.positions.len(), 3);
        assert_eq!(geometry.indices, vec![0, 1, 2]);
    }

    /// 回归：来自 Cesium 样例 tileset 的一个真实 b3dm 必须一路解码到几何。
    /// 守卫 `BufferTarget` 数值反序列化修复 —— `bufferView.target` 是
    /// 34962/34963（整数），此前派生的字符串变体 serde impl 会以一个 JSON
    /// 解析错误拒绝它。
    ///
    /// 该路径相对于 `CARGO_MANIFEST_DIR` 解析（向上三层到仓库根，那里也存有
    /// CesiumJS 的 `Apps/SampleData` 树），所以它在任何 checkout 上都能工作，
    /// 而不只是原作者的机器。当样例数据缺失时该测试跳过；上面那个合成测试
    /// 才是那个始终开启的守卫。
    #[test]
    fn test_parse_real_parent_b3dm_to_geometry() {
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../Apps/SampleData/Cesium3DTiles/Tilesets/Tileset/parent.b3dm"
        ));
        if !path.exists() {
            eprintln!("skipping: sample b3dm not found at {}", path.display());
            return;
        }
        let raw = std::fs::read(path).expect("read parent.b3dm");
        let decoded = extract_glb(&raw).expect("b3dm must extract a GLB");
        let geometry = parse_glb_to_geometry(&decoded.glb)
            .expect("embedded GLB must parse to geometry (BufferTarget fix)");
        assert_eq!(geometry.positions.len(), 240, "parent.b3dm has 240 vertices");
        assert!(!geometry.indices.is_empty(), "must have triangle indices");
    }
}
