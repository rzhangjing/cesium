use std::collections::HashMap;

use bevy::prelude::*;
use bevy::tasks::futures_lite::future::{block_on, poll_once};
use bevy::tasks::{IoTaskPool, Task};
use cesium_tileset::tileset::{TilesetJson, TilesetState};

use crate::components::{CesiumTilesetRoot, TilesetLoadingState};
use crate::pipeline;
use crate::resources::TileLoadStats;

#[derive(Resource, Default)]
pub struct LoadedTileset {
    pub tileset_json: Option<TilesetJson>,
    pub state: TilesetState,
    pub url: String,
    pub root_entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct TilesetFetchState {
    pub pending_urls: HashMap<String, TilesetFetchRequest>,
}

/// 一个处于飞行中的 `tileset.json` 下载，运行于 [`IoTaskPool`] 上。
pub struct TilesetFetchRequest {
    pub entity: Entity,
    /// 后台 fetch + 解析。从帧线程轮询（从不阻塞等待）。
    pub task: Task<Result<TilesetJson, String>>,
}

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

pub fn tileset_load_plugin(app: &mut App) {
    app.init_resource::<LoadedTileset>()
        .init_resource::<TilesetFetchState>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
    fn test_loaded_tileset_defaults() {
        let loaded = LoadedTileset::default();
        assert!(loaded.tileset_json.is_none());
        assert!(loaded.root_entity.is_none());
        assert!(loaded.url.is_empty());
    }

    #[test]
    fn test_tileset_state_base_path() {
        let url = "https://example.com/tiles/tileset.json";
        let base_path = url.rsplit_once('/').map(|(base, _)| base.to_string()).unwrap();
        assert_eq!(base_path, "https://example.com/tiles");
    }
}
