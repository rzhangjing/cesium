//! Per-tile Bing Maps imagery loader plugin.
//!
//! Downloads Bing Maps Aerial tiles individually and applies each
//! tile's texture directly to its corresponding globe tile entity.
//!
//! Bing Maps uses a quadkey tiling system, which we convert from standard XYZ tiles.

use bevy::prelude::*;
use std::io::Read;
use std::sync::mpsc;
use std::sync::Mutex;

use crate::tile_mesh::GlobeTile;

/// Plugin that loads Bing Maps satellite tiles per-tile and applies them to globe entities.
pub struct BingTileLoaderPlugin;

impl Plugin for BingTileLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BingTileLoadState>()
            .add_systems(Startup, spawn_bing_tile_downloads)
            .add_systems(Update, apply_bing_tile_textures);
    }
}

/// Zoom level for globe view (3 = 8×8 tiles = 64 requests).
const ZOOM: u32 = 3;

/// Resource tracking the per-tile loading progress.
#[derive(Resource)]
struct BingTileLoadState {
    receiver: Mutex<Option<mpsc::Receiver<BingTileResult>>>,
    tiles_received: u32,
    total_tiles: u32,
}

impl Default for BingTileLoadState {
    fn default() -> Self {
        let num_tiles = 1u32 << ZOOM;
        Self {
            receiver: Mutex::new(None),
            tiles_received: 0,
            total_tiles: num_tiles * num_tiles,
        }
    }
}

/// Result of downloading a single Bing tile.
struct BingTileResult {
    x: u32,
    y: u32,
    z: u32,
    /// RGBA pixel data (256x256).
    rgba_data: Vec<u8>,
    width: u32,
    height: u32,
}

/// Converts tile coordinates (x, y, z) to Bing Maps quadkey.
fn tile_to_quadkey(x: u32, y: u32, level: u32) -> String {
    let mut quadkey = String::with_capacity(level as usize);

    for i in (0..level).rev() {
        let mut digit = 0u8;
        let mask = 1 << i;

        if (x & mask) != 0 {
            digit |= 1;
        }
        if (y & mask) != 0 {
            digit |= 2;
        }

        quadkey.push_str(&digit.to_string());
    }

    quadkey
}

/// Spawns a background thread to download all Bing Maps satellite tiles at the configured zoom.
fn spawn_bing_tile_downloads(state: ResMut<BingTileLoadState>) {
    let (tx, rx) = mpsc::channel();
    *state.receiver.lock().unwrap() = Some(rx);

    std::thread::spawn(move || {
        let num_tiles = 1u32 << ZOOM;

        let agent = ureq::AgentBuilder::new()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) CesiumRust/0.1")
            .timeout(std::time::Duration::from_secs(15))
            .build();

        let mut success_count = 0u32;
        let total = num_tiles * num_tiles;

        for ty in 0..num_tiles {
            for tx_px in 0..num_tiles {
                // Convert XYZ tile coordinates to Bing Maps quadkey
                let quadkey = tile_to_quadkey(tx_px, ty, ZOOM);

                // Bing Maps Aerial imagery (no API key required for basic usage)
                // Using subdomain rotation for load balancing
                let subdomain = (tx_px + ty) % 8;
                let url = format!(
                    "https://ecn.t{}.tiles.virtualearth.net/tiles/a{}.jpeg?g=14393",
                    subdomain, quadkey
                );

                match agent.get(&url).call() {
                    Ok(response) => {
                        let mut reader = response.into_reader();
                        let mut data = Vec::new();
                        if reader.read_to_end(&mut data).is_ok() {
                            if let Ok(img) = image::load_from_memory(&data) {
                                let rgba_img = img.to_rgba8();
                                let (w, h) = rgba_img.dimensions();
                                // Send the tile immediately (progressive loading)
                                let _ = tx.send(BingTileResult {
                                    x: tx_px,
                                    y: ty,
                                    z: ZOOM,
                                    rgba_data: rgba_img.into_raw(),
                                    width: w,
                                    height: h,
                                });
                                success_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[BingTileLoader] Failed to fetch tile ({},{},{}): {}",
                            tx_px, ty, ZOOM, e
                        );
                    }
                }
            }
        }

        println!(
            "[BingTileLoader] Downloaded {}/{} Bing Maps tiles at zoom {}",
            success_count, total, ZOOM
        );
    });
}

/// System that receives downloaded tiles and applies textures to globe tile entities.
fn apply_bing_tile_textures(
    mut state: ResMut<BingTileLoadState>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tile_query: Query<(&GlobeTile, &MeshMaterial3d<StandardMaterial>)>,
) {
    // Try to receive all available results (non-blocking, batch)
    let results: Vec<BingTileResult> = {
        let guard = state.receiver.lock().unwrap();
        match &*guard {
            Some(rx) => {
                let mut batch = Vec::new();
                while let Ok(r) = rx.try_recv() {
                    batch.push(r);
                }
                batch
            }
            None => return,
        }
    };

    if results.is_empty() {
        return;
    }

    for result in results {
        // Create a Bevy Image from the tile's RGBA data
        let texture = Image::new(
            bevy::render::render_resource::Extent3d {
                width: result.width,
                height: result.height,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            result.rgba_data,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        let texture_handle = images.add(texture);

        // Find the matching globe tile entity and update its material
        for (globe_tile, mat_handle) in tile_query.iter() {
            if globe_tile.x == result.x && globe_tile.y == result.y && globe_tile.z == result.z {
                if let Some(material) = materials.get_mut(mat_handle) {
                    material.base_color_texture = Some(texture_handle.clone());
                    // Reset base_color to white: Bevy multiplies base_color with
                    // base_color_texture, so the initial ocean-blue fallback would
                    // otherwise tint the satellite imagery dark blue.
                    material.base_color = Color::WHITE;
                }
                break;
            }
        }

        state.tiles_received += 1;
        if state.tiles_received % 8 == 0 || state.tiles_received == state.total_tiles {
            println!(
                "[BingTileLoader] Applied {}/{} tile textures",
                state.tiles_received, state.total_tiles
            );
        }
    }
}
