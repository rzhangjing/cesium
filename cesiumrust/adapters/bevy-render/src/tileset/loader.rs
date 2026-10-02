//! 3D Tiles 瓦片集加载：异步下载并解析根 tileset.json。
//!
//! [`tileset_load_system`] 为每个未加载的 root 分派一次后台 fetch（跑在
//! [`IoTaskPool`] 上，帧线程从不阻塞），并由 [`poll_tileset_fetches`] 逐帧
//! 收获已完成的解析结果，写入 [`LoadedTileset`] 资源。
use std::collections::HashMap;

use bevy::prelude::*;
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_tileset::tileset::{TilesetJson, TilesetState};

use crate::components::{CesiumTilesetRoot, TilesetLoadingState};
use crate::pipeline;
use crate::resources::TileLoadStats;

/// 已加载的瓦片集资源（根 JSON、领域状态与根实体）。
#[derive(Resource, Default)]
pub struct LoadedTileset {
    /// 解析后的 tileset.json（未就绪为 None）。
    pub tileset_json: Option<TilesetJson>,
    /// 领域层瓦片集状态（持有 base 路径等）。
    pub state: TilesetState,
    /// 来源 URL。
    pub url: String,
    /// 对应的根实体。
    pub root_entity: Option<Entity>,
}

/// 飞行中的 fetch 集合资源（按 URL 去重）。
#[derive(Resource, Default)]
pub struct TilesetFetchState {
    /// URL → 待处理 fetch 请求。
    pub pending_urls: HashMap<String, TilesetFetchRequest>,
}

/// 一个处于飞行中的 `tileset.json` 下载，运行于 [`IoTaskPool`] 上。
pub struct TilesetFetchRequest {
    pub entity: Entity,
    /// 后台 fetch + 解析。从帧线程轮询（从不阻塞等待）。
    pub task: Task<Result<TilesetJson, String>>,
}

/// 主加载系统：收获已完成的 fetch，并为新出现的未加载 root 分派后台下载。
///
/// # 参数
/// - `commands`：实体命令（插入 Loading 状态）
/// - `tileset_query`：所有瓦片集 root
/// - `loaded`：已加载瓦片集资源（可写）
/// - `fetch_state`：飞行中 fetch 集合（可写）
/// - `stats`：加载统计（可写）
pub fn tileset_load_system(
    mut commands: Commands,
    tileset_query: Query<(Entity, &CesiumTilesetRoot)>,
    mut loaded: ResMut<LoadedTileset>,
    mut fetch_state: ResMut<TilesetFetchState>,
    mut stats: ResMut<TileLoadStats>,
) {
    // 收获自上一帧以来已解析的 fetch。`poll_once` 立即返回
    // （worker 仍在下载时返回 `None`），所以帧线程从不在网络上停车 —— 此前一个
    // 同步的 `block_on(fetcher.fetch(..))` 在此可能冻结帧长达 1 秒。
    poll_tileset_fetches(&mut commands, &mut fetch_state, &mut loaded, &mut stats);

    // 为每一个尚未开始加载的 root 分派一个后台 fetch。
    // 门控：ON 把路由经 cesium-pipeline core 的共享 keep-alive ureq
    // 池；OFF 使用一个全新的每次调用客户端。两者都不依赖 tokio 并运行于
    // IO task pool 上，所以帧线程从不会在网络上停车。
    let use_pipeline = pipeline::fetch::pipeline_gate_enabled();
    let pool = IoTaskPool::get();
    for (entity, root) in tileset_query.iter() {
        if !matches!(root.loading_state, TilesetLoadingState::NotLoaded) {
            continue;
        }

        // 另一个 entity 可能已在 fetch 这个确切 URL；不要推翻
        // 那个飞行中的 task。这个 entity 只在 `Loading` 中等待。
        if fetch_state.pending_urls.contains_key(&root.url) {
            commands.entity(entity).insert(CesiumTilesetRoot {
                loading_state: TilesetLoadingState::Loading,
                url: root.url.clone(),
            });
            continue;
        }

        let url = root.url.clone();
        let task_url = url.clone();
        let task = pool.spawn(async move { fetch_tileset_json(&task_url, use_pipeline) });
        fetch_state
            .pending_urls
            .insert(url, TilesetFetchRequest { entity, task });

        commands.entity(entity).insert(CesiumTilesetRoot {
            loading_state: TilesetLoadingState::Loading,
            url: root.url.clone(),
        });
    }
}

/// 对每个待处理（pending）的 `tileset.json` fetch 恰好轮询一次，并应用那些
/// 已解析的。运行于帧线程但从不阻塞。
fn poll_tileset_fetches(
    commands: &mut Commands,
    fetch_state: &mut TilesetFetchState,
    loaded: &mut LoadedTileset,
    stats: &mut TileLoadStats,
) {
    // 两遍：map 在其值被借用时无法被修改，所以
    // 先收集已解析的结果，再把它们 drain 掉。
    let mut resolved: Vec<(String, Entity, Result<TilesetJson, String>)> = Vec::new();
    for (url, request) in fetch_state.pending_urls.iter_mut() {
        if let Some(result) = block_on(poll_once(&mut request.task)) {
            resolved.push((url.clone(), request.entity, result));
        }
    }

    for (url, entity, result) in resolved {
        fetch_state.pending_urls.remove(&url);
        match result {
            Ok(tileset_json) => {
                let base_path = url
                    .rsplit_once('/')
                    .map(|(base, _)| base.to_string())
                    .unwrap_or_default();

                *loaded = LoadedTileset {
                    tileset_json: Some(tileset_json),
                    state: TilesetState::new(&base_path),
                    url: url.clone(),
                    root_entity: Some(entity),
                };

                // `try_insert`：那个 root 可能在 fetch 飞行途中已被 despawn。
                commands.entity(entity).try_insert(CesiumTilesetRoot {
                    loading_state: TilesetLoadingState::Ready,
                    url: url.clone(),
                });

                stats.tiles_loaded += 1;
            }
            Err(e) => {
                error!("Failed to load tileset from {}: {}", url, e);
                commands.entity(entity).try_insert(CesiumTilesetRoot {
                    loading_state: TilesetLoadingState::Failed(e),
                    url: url.clone(),
                });
                stats.tiles_failed += 1;
            }
        }
    }
}

/// 下载并解析 `tileset.json`。
///
/// 运行于一个 [`IoTaskPool`] worker 线程。该 fetch 不依赖 tokio —— 它经由
/// cesium-pipeline core 的 ureq 阻塞后端
/// （[`pipeline::fetch::fetch_gated`]），所以它从不停发帧线程。
fn fetch_tileset_json(url: &str, use_pipeline: bool) -> Result<TilesetJson, String> {
    let data = pipeline::fetch::fetch_gated(url, use_pipeline)?;

    let json_str = String::from_utf8(data).map_err(|e| format!("Invalid UTF-8: {}", e))?;
    TilesetJson::from_json(&json_str).map_err(|e| format!("JSON parse error: {}", e))
}

/// 初始化瓦片集加载插件：注册相关资源。
///
/// # 参数
/// - `app`：Bevy 应用
pub fn tileset_load_plugin(app: &mut App) {
    app.init_resource::<LoadedTileset>()
        .init_resource::<TilesetFetchState>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 应能从内联 JSON 字符串解析出瓦片集字段。
    fn test_fetch_tileset_json_from_string() {
        let json = r#"{
            "asset": { "version": "1.0" },
            "geometricError": 240,
            "root": {
                "boundingVolume": { "sphere": [0, 0, 0, 100] },
                "geometricError": 70,
                "content": { "uri": "tile.b3dm" }
            }
        }"#;
        let tileset = TilesetJson::from_json(json).unwrap();
        assert_eq!(tileset.asset.version, "1.0");
        assert_eq!(tileset.geometric_error, 240.0);
        assert_eq!(tileset.root.geometric_error, 70.0);
    }

    #[test]
    /// 默认已加载瓦片集应为空（无 JSON/实体、URL 为空）。
    fn test_loaded_tileset_defaults() {
        let loaded = LoadedTileset::default();
        assert!(loaded.tileset_json.is_none());
        assert!(loaded.root_entity.is_none());
        assert!(loaded.url.is_empty());
    }

    #[test]
    /// 应从 URL 去掉最后一段得到 base 路径。
    fn test_tileset_state_base_path() {
        let url = "https://example.com/tiles/tileset.json";
        let base_path = url.rsplit_once('/').map(|(base, _)| base.to_string()).unwrap();
        assert_eq!(base_path, "https://example.com/tiles");
    }
}
