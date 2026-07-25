//! Per-tile map imagery loader plugin.
//!
//! Downloads Gaode (AutoNavi) satellite tiles individually and applies each
//! tile's texture directly to its corresponding globe tile entity.
//! No global resampling needed — each tile maps directly to its geographic extent.
//!
//! Architecture follows CesiumJS: each terrain tile has its own imagery texture,
//! mapped via UV coordinates normalized to [0,1] within the tile's bounds.

use bevy::prelude::*;
use std::io::Read;
use std::sync::mpsc;
use std::sync::Mutex;

use crate::tile_mesh::GlobeTile;

/// Plugin that loads real satellite map tiles per-tile and applies them to globe entities.
pub struct TileLoaderPlugin;

impl Plugin for TileLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileLoadState>()
            .add_systems(Startup, spawn_tile_downloads)
            .add_systems(Update, apply_tile_textures);
    }
}

/// Zoom level for globe view (3 = 8×8 tiles = 64 requests).
const ZOOM: u32 = 3;

/// Resource tracking the per-tile loading progress.
#[derive(Resource)]
struct TileLoadState {
    receiver: Mutex<Option<mpsc::Receiver<TileResult>>>,
    tiles_received: u32,
    total_tiles: u32,
}

impl Default for TileLoadState {
    fn default() -> Self {
        let num_tiles = 1u32 << ZOOM;
        Self {
            receiver: Mutex::new(None),
            tiles_received: 0,
            total_tiles: num_tiles * num_tiles,
        }
    }
}

/// Result of downloading a single tile.
struct TileResult {
    x: u32,
    y: u32,
    z: u32,
    /// RGBA pixel data (256x256).
    rgba_data: Vec<u8>,
    width: u32,
    height: u32,
}

/// Spawns a background thread to download all Gaode satellite tiles at the configured zoom.
fn spawn_tile_downloads(state: ResMut<TileLoadState>) {
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
                // Gaode satellite imagery (style=6), rotate subdomains
                let url = format!(
                    "https://webst0{}.is.autonavi.com/appmaptile?style=6&x={}&y={}&z={}",
                    (tx_px + ty) % 4 + 1,
                    tx_px,
                    ty,
                    ZOOM
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
                                let _ = tx.send(TileResult {
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
                            "[TileLoader] Failed to fetch tile ({},{}): {}",
                            tx_px, ty, e
                        );
                    }
                }
            }
        }

        println!(
            "[TileLoader] Downloaded {}/{} satellite tiles at zoom {}",
            success_count, total, ZOOM
        );
    });
}

/// System that receives downloaded tiles and applies textures to globe tile entities.
fn apply_tile_textures(
    mut state: ResMut<TileLoadState>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tile_query: Query<(&GlobeTile, &MeshMaterial3d<StandardMaterial>)>,
) {
    // Try to receive all available results (non-blocking, batch)
    let results: Vec<TileResult> = {
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
                "[TileLoader] Applied {}/{} tile textures",
                state.tiles_received, state.total_tiles
            );
        }
    }
}
