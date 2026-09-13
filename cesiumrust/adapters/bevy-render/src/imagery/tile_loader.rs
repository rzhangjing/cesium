use std::collections::HashMap;

use bevy::prelude::*;
use cesium_geospatial::ellipsoid::Ellipsoid;
use cesium_geospatial::rectangle::Rectangle;
use cesium_geospatial::tiling_scheme::TilingScheme;
use cesium_imagery::{compute_tile_requests, ImageryLayer, ImageryTileRequest};
use cesium_network::HttpTileFetcher;
use cesium_ports_driven::TileFetcher;

use crate::components::CesiumTerrainTile;
use crate::resources::TileLoadStats;

use super::layer_manager::ImageryLayerManager;

#[derive(Resource, Default)]
pub struct ImageryCache {
    pub textures: HashMap<(u64, u32, u32, u32), Handle<Image>>,
}

#[derive(Resource, Default)]
pub struct ImageryPendingLoads {
    pub pending: HashMap<(u64, u32, u32, u32), bool>,
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

fn fetch_and_decode_image(url: &str) -> Result<(Vec<u8>, u32, u32), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio: {}", e))?;

    let fetcher = HttpTileFetcher::new(url);

    let data = runtime
        .block_on(async { fetcher.fetch(url, 0.5).await })
        .map_err(|e| format!("fetch: {:?}", e))?;

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
                if !pending.pending.contains_key(&key) {
                    pending.pending.insert(key, false);
                }
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

    let keys: Vec<(u64, u32, u32, u32)> = pending
        .pending
        .iter()
        .filter(|(_, v)| !**v)
        .map(|(k, _)| *k)
        .collect();

    for (layer_id, x, y, level) in keys {
        *pending.pending.get_mut(&(layer_id, x, y, level)).unwrap() = true;

        if cache.textures.contains_key(&(layer_id, x, y, level)) {
            continue;
        }

        let desc = match imagery_manager.get_layer(layer_id) {
            Some(d) => d,
            None => {
                pending.pending.remove(&(layer_id, x, y, level));
                continue;
            }
        };

        let url = build_imagery_url(&desc.url_template, x, y, level, &scheme);

        match fetch_and_decode_image(&url) {
            Ok((data, width, height)) => {
                let bevy_image = crate::create_imagery_texture(width, height, data);
                let handle = images.add(bevy_image);
                cache.textures.insert((layer_id, x, y, level), handle);
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

        pending.pending.remove(&(layer_id, x, y, level));
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
    }

    #[test]
    fn test_tile_rectangle() {
        let scheme = TilingScheme::geographic(Ellipsoid::WGS84);
        let rect = tile_rectangle(0, 0, 0, &scheme);
        let r2 = scheme.rectangle();
        assert!(rect.west >= r2.west);
        assert!(rect.east <= r2.east);
    }
}
