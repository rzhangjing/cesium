use std::collections::HashMap;

use bevy::prelude::*;
use cesium_geospatial::rectangle::Rectangle;
use cesium_network::HttpTileFetcher;
use cesium_ports_driven::TileFetcher;
use cesium_terrain::QuantizedMeshTerrainData;

use crate::components::{CesiumTerrainTile, TileContentState};
use crate::resources::{GlobeConfig, TileLoadStats};

use super::lod_system::TerrainSelection;

#[derive(Resource, Default)]
pub struct TerrainLoadState {
    pub loaded_count: u32,
    pub failed_count: u32,
}

#[derive(Resource, Default)]
pub struct TerrainPendingLoads {
    pub pending: HashMap<(u32, u32, u32), PendingLoad>,
}

pub struct PendingLoad {
    pub in_progress: bool,
}

fn build_terrain_url(config: &GlobeConfig, x: u32, y: u32, level: u32) -> Option<String> {
    let base = config.terrain_provider_url.as_ref()?;
    Some(
        base.replace("{z}", &level.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string()),
    )
}

fn tile_rectangle(x: u32, y: u32, level: u32) -> Rectangle {
    let n = 2u32.pow(level.max(1)) as f64;
    let west = (x as f64 / n) * std::f64::consts::TAU - std::f64::consts::PI;
    let east = ((x as f64 + 1.0) / n) * std::f64::consts::TAU - std::f64::consts::PI;
    let south = (y as f64 / n) * std::f64::consts::PI - std::f64::consts::FRAC_PI_2;
    let north = ((y as f64 + 1.0) / n) * std::f64::consts::PI - std::f64::consts::FRAC_PI_2;
    Rectangle::from_radians(west, south, east, north)
}

fn fetch_and_decode_terrain(url: &str) -> Result<QuantizedMeshTerrainData, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {}", e))?;

    let fetcher = HttpTileFetcher::new(url);

    let data = runtime
        .block_on(async { fetcher.fetch(url, 0.5).await })
        .map_err(|e| format!("fetch: {:?}", e))?;

    serde_json::from_slice(&data).map_err(|e| format!("deserialize: {}", e))
}

pub fn terrain_tile_load_system(
    mut commands: Commands,
    config: Option<Res<GlobeConfig>>,
    mut selection: ResMut<TerrainSelection>,
    mut pending: ResMut<TerrainPendingLoads>,
    mut load_state: ResMut<TerrainLoadState>,
    mut stats: ResMut<TileLoadStats>,
    terrain_query: Query<(Entity, &CesiumTerrainTile)>,
) {
    let config = match config {
        Some(c) => c,
        None => return,
    };

    for (x, y, level) in selection.tiles_to_load.drain(..) {
        if pending.pending.contains_key(&(x, y, level)) {
            continue;
        }

        let url = match build_terrain_url(&config, x, y, level) {
            Some(u) => u,
            None => continue,
        };

        pending.pending.insert(
            (x, y, level),
            PendingLoad {
                in_progress: true,
            },
        );

        match fetch_and_decode_terrain(&url) {
            Ok(qm) => {
                let rect = tile_rectangle(x, y, level);
                let terrain_mesh =
                    qm.create_mesh_with_skirts(&rect, &config.ellipsoid, 1.0);

                let existing = terrain_query
                    .iter()
                    .find(|(_, t)| t.x == x && t.y == y && t.level == level)
                    .map(|(e, _)| e);

                if let Some(entity) = existing {
                    commands.entity(entity).insert((
                        CesiumTerrainTile { x, y, level },
                        TerrainTileReady {
                            terrain_mesh: Some(terrain_mesh),
                            state: TileContentState::Ready,
                        },
                    ));
                } else {
                    commands.spawn((
                        CesiumTerrainTile { x, y, level },
                        TerrainTileReady {
                            terrain_mesh: Some(terrain_mesh),
                            state: TileContentState::Ready,
                        },
                        Transform::default(),
                        Visibility::default(),
                    ));
                }

                load_state.loaded_count += 1;
                stats.tiles_loaded += 1;
            }
            Err(e) => {
                error!("Terrain tile ({},{},{}) failed: {}", x, y, level, e);
                load_state.failed_count += 1;
                stats.tiles_failed += 1;

                let existing = terrain_query
                    .iter()
                    .find(|(_, t)| t.x == x && t.y == y && t.level == level)
                    .map(|(e, _)| e);

                if let Some(entity) = existing {
                    commands.entity(entity).insert((
                        CesiumTerrainTile { x, y, level },
                        TerrainTileReady {
                            terrain_mesh: None,
                            state: TileContentState::Failed,
                        },
                    ));
                } else {
                    commands.spawn((
                        CesiumTerrainTile { x, y, level },
                        TerrainTileReady {
                            terrain_mesh: None,
                            state: TileContentState::Failed,
                        },
                    ));
                }
            }
        }

        pending.pending.remove(&(x, y, level));
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
}
