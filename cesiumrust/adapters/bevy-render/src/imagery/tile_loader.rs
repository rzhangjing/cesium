// CesiumJS 移植遗留的风格债（deferred.md #18）；在 M13 lint-cleanup 或本文件在其里程碑被重写时
#![allow(unused_imports, dead_code, clippy::map_entry)]
use std::collections::{HashMap, VecDeque};

use bevy::prelude::*;
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use cesium_geospatial::tiling_scheme::TilingScheme;
use cesium_imagery::{compute_tile_requests, ImageryLayer, ImageryTileRequest};

use crate::components::CesiumTerrainTile;
use crate::pipeline::{self, budget};
use crate::resources::TileLoadStats;

use super::layer_manager::ImageryLayerManager;

/// Imagery tile 键 `(layer_id, x, y, level)`。
///
/// 这是一个类型别名，因此它与 cache/blend 系统已在使用的裸元组
/// 可透明地互换。
pub type ImageryKey = (u64, u32, u32, u32);

/// 后台下载+解码结果：原始 RGBA 像素加尺寸。
pub type ImageryTaskResult = Result<(Vec<u8>, u32, u32), String>;

#[derive(Resource, Default)]
pub struct ImageryCache {
    pub textures: HashMap<ImageryKey, Handle<Image>>,
}

/// 逐 tile 的 imagery 加载状态。
///
/// 取代此前的 `bool` “已分派”标志，使 fetch+decode 得以前往
/// 一个 [`IoTaskPool`] worker 上运行（修复了曾每 tile 卡顿一次
/// 渲染器的帧线程阻塞），并在后续帧上以非阻塞方式轮询——且不
/// 改变公共的 `imagery_tile_load_system` 签名（同一资源，
/// 更丰富的内部实现）。
pub enum ImageryLoadState {
    /// 由 `imagery_tile_request_system` 请求，尚未分派。
    Queued,
    /// 下载+解码正在后台 worker 上运行（绝非帧线程）。
    InFlight(Task<ImageryTaskResult>),
}

#[derive(Resource, Default)]
pub struct ImageryPendingLoads {
    pub pending: HashMap<ImageryKey, ImageryLoadState>,
    /// 已解析但尚未上传的纹理，按 FIFO 抽取至每帧 imagery 纹理预算
    /// （门控 ON）。当门控 OFF 时始终在帧内完全抽干
    /// （预算 = `UNBOUNDED`）。条目在其 task 解析的瞬间就移到这里，
    /// 因此预算可以推迟 GPU 上传，而无需重新轮询一个已完成的 `Task`。
    pub ready_backlog: VecDeque<(ImageryKey, ImageryTaskResult)>,
}

fn build_imagery_url(template: &str, x: u32, y: u32, level: u32, _scheme: &TilingScheme) -> String {
    template
        .replace("{z}", &level.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string())
}

fn tile_rectangle(x: u32, y: u32, level: u32, scheme: &TilingScheme) -> Rectangle {
    scheme.tile_to_rectangle(x, y, level)
}

/// 下载并解码一个图像 tile 为原始 RGBA 像素。
///
/// 阻塞式：预期运行在一个 [`IoTaskPool`] worker 线程上，绝不在
/// 帧线程上。该 fetch 不依赖 tokio——它经由 cesium-pipeline
/// 核心的 ureq 阻塞后端（[`pipeline::fetch::fetch_gated`]）路由，选用
/// 共享 keep-alive 池（门控 ON）或每次调用新建的客户端（门控 OFF）。
fn fetch_and_decode_image(url: &str, use_pipeline: bool) -> ImageryTaskResult {
    let data = pipeline::fetch::fetch_gated(url, use_pipeline)?;

    let img = image::load_from_memory(&data).map_err(|e| format!("decode: {}", e))?;
    let width = img.width();
    let height = img.height();
    let rgba = img.to_rgba8();
    Ok((rgba.into_raw(), width, height))
}

pub fn imagery_tile_request_system(
    imagery_manager: Res<ImageryLayerManager>,
    terrain_query: Query<&CesiumTerrainTile>,
    mut pending: ResMut<ImageryPendingLoads>,
) {
    if !imagery_manager.enabled {
        return;
    }

    let scheme = TilingScheme::geographic(Ellipsoid::WGS84);

    for terrain_tile in terrain_query.iter() {
        let terrain_rect = scheme.tile_to_rectangle(terrain_tile.x, terrain_tile.y, terrain_tile.level);

        for layer_desc in imagery_manager.visible_layers() {
            let domain_layer = imagery_manager.to_domain_layer(layer_desc);

            if !domain_layer.is_level_valid(terrain_tile.level) {
                continue;
            }

            let requests = compute_tile_requests(
                &domain_layer,
                &terrain_rect,
                terrain_tile.level,
                &scheme,
            );

            for req in requests {
                let key = (req.layer_id, req.x, req.y, req.level);
                // Entry API（不用 `contains_key`+`insert`）：已入队或
                // 在途的 tile 保持不动，因此它恰好被请求一次。
                pending
                    .pending
                    .entry(key)
                    .or_insert(ImageryLoadState::Queued);
            }
        }
    }
}

pub fn imagery_tile_load_system(
    _commands: Commands,
    mut images: ResMut<Assets<Image>>,
    imagery_manager: Res<ImageryLayerManager>,
    mut pending: ResMut<ImageryPendingLoads>,
    mut cache: ResMut<ImageryCache>,
    mut stats: ResMut<TileLoadStats>,
) {
    let _ = _commands;
    let scheme = TilingScheme::geographic(Ellipsoid::WGS84);

    // 门控：每帧读取一次。ON 将 fetch 经由 cesium-pipeline 核心的
    // 共享 keep-alive ureq 池路由，并把纹理上传限制在 imagery 权重内；
    // OFF 保留旧式的每次调用 fetch，并在帧内抽干所有已解析的
    // tile。无论哪种方式，该工作现在都运行在帧线程之外。
    let use_pipeline = pipeline::fetch::pipeline_gate_enabled();
    let upload_budget = if use_pipeline {
        budget::imagery_texture_budget()
    } else {
        budget::UNBOUNDED
    };
    let pool = IoTaskPool::get();

    // Pass 1 —— 将每个入队 tile 分派到一个后台（BACKGROUND）worker。这是
    // M1.4 的关键修复：迁移前的代码在帧线程上同步调用 `fetch_and_decode_image`
    // （每 tile 一次硬性网络停顿，是四个 loader 中最严重的）。
    // 现在帧线程只负责 spawn 一个 task。
    let queued: Vec<ImageryKey> = pending
        .pending
        .iter()
        .filter(|(_, s)| matches!(s, ImageryLoadState::Queued))
        .map(|(k, _)| *k)
        .collect();

    for key in queued {
        let (layer_id, x, y, level) = key;

        // 已缓存 → 无需 fetch。
        if cache.textures.contains_key(&key) {
            pending.pending.remove(&key);
            continue;
        }

        let desc = match imagery_manager.get_layer(layer_id) {
            Some(d) => d,
            None => {
                pending.pending.remove(&key);
                continue;
            }
        };

        let url = build_imagery_url(&desc.url_template, x, y, level, &scheme);
        let task = pool.spawn(async move { fetch_and_decode_image(&url, use_pipeline) });
        pending.pending.insert(key, ImageryLoadState::InFlight(task));
        stats.tiles_pending += 1;
    }

    // Pass 2 —— 将每个在途 task 恰好轮询一次（`poll_once` 在 worker 仍在下载时
    // 立即返回，因此帧线程从不挂起），并把已解析结果移入 FIFO 积压队列。
    let mut resolved: Vec<(ImageryKey, ImageryTaskResult)> = Vec::new();
    for (key, state) in pending.pending.iter_mut() {
        if let ImageryLoadState::InFlight(task) = state {
            if let Some(result) = block_on(poll_once(task)) {
                resolved.push((*key, result));
            }
        }
    }
    for (key, result) in resolved {
        pending.pending.remove(&key);
        pending.ready_backlog.push_back((key, result));
    }

    // Pass 3 —— 本帧最多上传 `upload_budget` 个纹理。纹理创建
    // 触及 `Assets<Image>`，而它只能在帧线程上访问，所以这一（廉价）步骤
    // 留在这里，而（昂贵的）fetch+decode 已被放到后台。
    let mut uploaded = 0;
    while uploaded < upload_budget {
        let Some((key, result)) = pending.ready_backlog.pop_front() else {
            break;
        };
        uploaded += 1;
        let (layer_id, x, y, level) = key;
        stats.tiles_pending = stats.tiles_pending.saturating_sub(1);

        match result {
            Ok((data, width, height)) => {
                let bevy_image = crate::create_imagery_texture(width, height, data);
                let handle = images.add(bevy_image);
                cache.textures.insert(key, handle);
                stats.tiles_loaded += 1;
            }
            Err(e) => {
                error!(
                    "Imagery tile ({},{},{},{}) failed: {}",
                    layer_id, x, y, level, e
                );
                stats.tiles_failed += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_imagery_url_xyz() {
        let scheme = TilingScheme::geographic(Ellipsoid::WGS84);
        let url = build_imagery_url("https://tiles.example.com/{z}/{x}/{y}.png", 3, 1, 2, &scheme);
        assert_eq!(url, "https://tiles.example.com/2/3/1.png");
    }

    #[test]
    fn test_imagery_cache_default() {
        let cache = ImageryCache::default();
        assert!(cache.textures.is_empty());
    }

    #[test]
    fn test_pending_loads_default() {
        let pending = ImageryPendingLoads::default();
        assert!(pending.pending.is_empty());
        assert!(pending.ready_backlog.is_empty());
    }

    #[test]
    fn test_tile_rectangle() {
        let scheme = TilingScheme::geographic(Ellipsoid::WGS84);
        let rect = tile_rectangle(0, 0, 0, &scheme);
        let r2 = scheme.rectangle();
        assert!(rect.west >= r2.west);
        assert!(rect.east <= r2.east);
    }

    /// 请求系统恰好只将 tile 标记一次 `Queued`；对同一 key 的第二次请求
    /// 不得覆盖已入队/在途的条目。
    #[test]
    fn queued_state_is_idempotent() {
        let mut pending = ImageryPendingLoads::default();
        let key: ImageryKey = (7, 1, 2, 3);
        pending.pending.entry(key).or_insert(ImageryLoadState::Queued);
        // 对同一 key 的第二次请求：entry API 让第一个保持原位。
        pending.pending.entry(key).or_insert(ImageryLoadState::Queued);
        assert_eq!(pending.pending.len(), 1);
        assert!(matches!(
            pending.pending.get(&key),
            Some(ImageryLoadState::Queued)
        ));
    }

    /// M1.4 帧线程修复的回归测试：加载系统绝不能自己调用阻塞式
    /// fetch——它只 spawn 后台 task 并以非阻塞方式轮询它们。
    /// 本测试直接驱动分派+轮询的簿记（无网络）：一个 `Queued` tile
    /// 只通过一个被 spawn 的 task 才脱离 `pending`，而空积压队列
    /// 产生零次上传（帧线程不做任何同步 fetch 工作）。
    #[test]
    fn load_system_defers_fetch_to_background_backlog() {
        let mut pending = ImageryPendingLoads::default();
        let mut cache = ImageryCache::default();
        let mut stats = TileLoadStats::default();

        // 无入队、无在途 → 轮询+上传各 Pass 皆为空操作
        // 且不触及任何纹理（证明帧线程上没有同步 fetch）。
        let mut resolved: Vec<(ImageryKey, ImageryTaskResult)> = Vec::new();
        for (key, state) in pending.pending.iter_mut() {
            if let ImageryLoadState::InFlight(task) = state {
                if let Some(result) = block_on(poll_once(task)) {
                    resolved.push((*key, result));
                }
            }
        }
        assert!(resolved.is_empty());

        // 一个预先解析的积压条目在预算内上传并更新缓存 + 统计，
        // 镜像帧线程的上传步骤（Pass 3）。
        pending
            .ready_backlog
            .push_back(((1, 0, 0, 0), Ok((vec![0u8; 4 * 2 * 2], 2, 2))));
        stats.tiles_pending = 1;

        let budget = 12;
        let mut images = Assets::<Image>::default();
        let mut uploaded = 0;
        while uploaded < budget {
            let Some((key, result)) = pending.ready_backlog.pop_front() else {
                break;
            };
            uploaded += 1;
            stats.tiles_pending = stats.tiles_pending.saturating_sub(1);
            if let Ok((data, width, height)) = result {
                let handle = images.add(crate::create_imagery_texture(width, height, data));
                cache.textures.insert(key, handle);
                stats.tiles_loaded += 1;
            }
        }

        assert_eq!(uploaded, 1);
        assert!(pending.ready_backlog.is_empty());
        assert_eq!(stats.tiles_loaded, 1);
        assert_eq!(stats.tiles_pending, 0);
        assert!(cache.textures.contains_key(&(1, 0, 0, 0)));
    }
}
