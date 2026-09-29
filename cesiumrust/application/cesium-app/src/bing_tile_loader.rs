//! 逐瓦片的 Bing Maps 影像加载器插件。
//!
//! 逐个下载 Bing Maps Aerial 瓦片，并将每块瓦片的纹理直接
//! 应用到其对应的地球瓦片实体。
//!
//! Bing Maps 使用 quadkey 切分系统，我们从标准 XYZ 瓦片转换而来。

use bevy::prelude::*;
use std::io::Read;
use std::sync::mpsc;
use std::sync::Mutex;

use crate::tile_mesh::GlobeTile;

/// 逐瓦片加载 Bing Maps 卫星瓦片并应用到地球实体的插件。
pub struct BingTileLoaderPlugin;

impl Plugin for BingTileLoaderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BingTileLoadState>()
            .add_systems(Startup, spawn_bing_tile_downloads)
            .add_systems(Update, apply_bing_tile_textures);
    }
}

/// 地球视图的缩放级别（3 = 8×8 瓦片 = 64 个请求）。
const ZOOM: u32 = 3;

/// 追踪逐瓦片加载进度（per-tile）的资源。
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

/// 单块 Bing 瓦片下载的结果。
struct BingTileResult {
    x: u32,
    y: u32,
    z: u32,
    /// RGBA 像素数据（256x256）。
    rgba_data: Vec<u8>,
    width: u32,
    height: u32,
}

/// 将瓦片坐标 (x, y, z) 转换为 Bing Maps quadkey。
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

/// 生成一个后台线程，在配置的缩放级别下载所有 Bing Maps 卫星瓦片。
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
                // 将 XYZ 瓦片坐标转换为 Bing Maps quadkey
                let quadkey = tile_to_quadkey(tx_px, ty, ZOOM);

                // Bing Maps Aerial 影像（基本使用无需 API key）
                // 使用子域名轮换做负载均衡
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
                                // 立即发送该瓦片（渐进式加载）
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

/// 接收已下载瓦片并将纹理应用到地球瓦片实体的系统。
fn apply_bing_tile_textures(
    mut state: ResMut<BingTileLoadState>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tile_query: Query<(&GlobeTile, &MeshMaterial3d<StandardMaterial>)>,
) {
    // 尝试接收所有可用结果（非阻塞，批量）
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
        // 从瓦片的 RGBA 数据创建一个 Bevy Image
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

        // 找到匹配的地球瓦片实体并更新其材质
        for (globe_tile, mat_handle) in tile_query.iter() {
            if globe_tile.x == result.x && globe_tile.y == result.y && globe_tile.z == result.z {
                if let Some(material) = materials.get_mut(mat_handle) {
                    material.base_color_texture = Some(texture_handle.clone());
                    // 将 base_color 重置为白色：Bevy 会将 base_color 与
                    // base_color_texture 相乘，否则初始的海洋蓝回退色
                    // 会把卫星影像染成暗蓝。
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
