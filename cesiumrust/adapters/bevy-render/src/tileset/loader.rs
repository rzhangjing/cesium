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

/// An in-flight `tileset.json` download running on the [`IoTaskPool`].
pub struct TilesetFetchRequest {
    pub entity: Entity,
    /// Background fetch + parse. Polled (never blocked on) from the frame thread.
    pub task: Task<Result<TilesetJson, String>>,
}

pub fn tileset_load_system(
    mut commands: Commands,
    tileset_query: Query<(Entity, &CesiumTilesetRoot)>,
    mut loaded: ResMut<LoadedTileset>,
    mut fetch_state: ResMut<TilesetFetchState>,
    mut stats: ResMut<TileLoadStats>,
) {
    // Harvest fetches that resolved since the previous frame. `poll_once` returns
    // immediately (`None` while the worker is still downloading), so the frame
    // thread never stalls on the network — previously a synchronous
    // `block_on(fetcher.fetch(..))` here could freeze the frame for up to 1s.
    poll_tileset_fetches(&mut commands, &mut fetch_state, &mut loaded, &mut stats);

    // Dispatch a background fetch for every root that has not started loading.
    // Gate: ON routes through the cesium-pipeline core's shared keep-alive ureq
    // pool; OFF uses a fresh per-call client. Both are tokio-free and run on the
    // IO task pool, so the frame thread never stalls on the network.
    let use_pipeline = pipeline::fetch::pipeline_gate_enabled();
    let pool = IoTaskPool::get();
    for (entity, root) in tileset_query.iter() {
        if !matches!(root.loading_state, TilesetLoadingState::NotLoaded) {
            continue;
        }

        // Another entity may already be fetching this exact URL; do not clobber
        // the in-flight task. This entity just waits in `Loading`.
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

/// Polls every pending `tileset.json` fetch exactly once and applies the ones
/// that resolved. Runs on the frame thread but never blocks.
fn poll_tileset_fetches(
    commands: &mut Commands,
    fetch_state: &mut TilesetFetchState,
    loaded: &mut LoadedTileset,
    stats: &mut TileLoadStats,
) {
    // Two passes: the map cannot be mutated while its values are borrowed, so
    // collect the resolved outcomes first, then drain them.
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

                // `try_insert`: the root may have been despawned while the fetch
                // was in flight.
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

/// Downloads and parses `tileset.json`.
///
/// Runs on an [`IoTaskPool`] worker thread. The fetch is tokio-free — it routes
/// through the cesium-pipeline core's ureq blocking backend
/// ([`pipeline::fetch::fetch_gated`]), so it never stalls the frame thread.
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
