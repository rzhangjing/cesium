use std::collections::HashMap;

use bevy::prelude::*;
use cesium_network::HttpTileFetcher;
use cesium_ports_driven::TileFetcher;
use cesium_tileset::tileset::{TilesetJson, TilesetState};

use crate::components::{CesiumTilesetRoot, TilesetLoadingState};
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

pub struct TilesetFetchRequest {
    pub entity: Entity,
}

pub fn tileset_load_system(
    mut commands: Commands,
    tileset_query: Query<(Entity, &CesiumTilesetRoot)>,
    mut loaded: ResMut<LoadedTileset>,
    mut fetch_state: ResMut<TilesetFetchState>,
    mut stats: ResMut<TileLoadStats>,
) {
    for (entity, root) in tileset_query.iter() {
        match &root.loading_state {
            TilesetLoadingState::NotLoaded => {
                let url = root.url.clone();
                fetch_state.pending_urls.insert(
                    url.clone(),
                    TilesetFetchRequest { entity },
                );
                commands.entity(entity).insert(CesiumTilesetRoot {
                    loading_state: TilesetLoadingState::Loading,
                    url: root.url.clone(),
                });
            }
            TilesetLoadingState::Loading => {
                if let Some(req) = fetch_state.pending_urls.get(&root.url) {
                    if req.entity != entity {
                        continue;
                    }
                }

                let fetcher = HttpTileFetcher::new(&root.url);
                let result = fetch_tileset_json_sync(&fetcher, &root.url);

                match result {
                    Ok(tileset_json) => {
                        let base_path = root
                            .url
                            .rsplit_once('/')
                            .map(|(base, _)| base.to_string())
                            .unwrap_or_default();

                        *loaded = LoadedTileset {
                            tileset_json: Some(tileset_json),
                            state: TilesetState::new(&base_path),
                            url: root.url.clone(),
                            root_entity: Some(entity),
                        };

                        commands.entity(entity).insert(CesiumTilesetRoot {
                            loading_state: TilesetLoadingState::Ready,
                            url: root.url.clone(),
                        });

                        stats.tiles_loaded += 1;
                        fetch_state.pending_urls.remove(&root.url);
                    }
                    Err(e) => {
                        error!("Failed to load tileset from {}: {:?}", root.url, e);
                        commands.entity(entity).insert(CesiumTilesetRoot {
                            loading_state: TilesetLoadingState::Failed(e.to_string()),
                            url: root.url.clone(),
                        });
                        stats.tiles_failed += 1;
                        fetch_state.pending_urls.remove(&root.url);
                    }
                }
            }
            _ => {}
        }
    }
}

fn fetch_tileset_json_sync(
    fetcher: &HttpTileFetcher,
    url: &str,
) -> Result<TilesetJson, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;

    let data = runtime
        .block_on(async { fetcher.fetch(url, 1.0).await })
        .map_err(|e| format!("Fetch error: {:?}", e))?;

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
