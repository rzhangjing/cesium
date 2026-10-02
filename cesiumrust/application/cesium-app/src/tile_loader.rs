//! 逐瓦片的地图影像加载器插件。
//!
//! 逐个下载高德（AutoNavi）卫星瓦片，并将每块瓦片的纹理直接
//! 应用到其对应的地球瓦片实体。无需全局重采样 —— 每块瓦片
//! 直接映射到其地理范围。
//!
//! 架构沿用 CesiumJS：每块地形瓦片拥有自己的影像纹理，
//! 通过归一化到 [0,1]（在瓦片边界内）的 UV 坐标映射。

use bevy::prelude::*;
use std::io::Read;
use std::sync::mpsc;
use std::sync::Mutex;

use crate::tile_mesh::GlobeTile;

/// 逐瓦片加载真实卫星地图瓦片并应用到地球实体的插件。
pub struct TileLoaderPlugin;

impl Plugin for TileLoaderPlugin {
    /// 插件装配入口：初始化加载状态资源，挂载启动下载与更新应纹理系统。
    ///
    /// # 参数
    /// - `app`：Bevy 应用。
    fn build(&self, app: &mut App) {
        app.init_resource::<TileLoadState>()
            .add_systems(Startup, spawn_tile_downloads)
            .add_systems(Update, apply_tile_textures);
    }
}

/// 地球视图的缩放级别（3 = 8×8 瓦片 = 64 个请求）。
const ZOOM: u32 = 3;

/// 追踪逐瓦片加载进度（per-tile）的资源。
#[derive(Resource)]
struct TileLoadState {
    /// 已下载瓦片结果的接收端（后台线程→主线程）；尚未启动时为 `None`。
    receiver: Mutex<Option<mpsc::Receiver<TileResult>>>,
    /// 已接收并应用的瓦片数。
    tiles_received: u32,
    /// 本层级应下载的总瓦片数。
    total_tiles: u32,
}

impl Default for TileLoadState {
    /// 默认：接收端置空，计数器归零，总瓦片数按 ZOOM 层级算为 (2^ZOOM)^2。
    fn default() -> Self {
        let num_tiles = 1u32 << ZOOM;
        Self {
            receiver: Mutex::new(None),
            tiles_received: 0,
            total_tiles: num_tiles * num_tiles,
        }
    }
}

/// 单块瓦片下载的结果。
struct TileResult {
    /// 瓦片列索引 x。
    x: u32,
    /// 瓦片行索引 y。
    y: u32,
    /// 瓦片层级 z。
    z: u32,
    /// RGBA 像素数据（256x256）。
    rgba_data: Vec<u8>,
    /// 纹理宽度（像素）。
    width: u32,
    /// 纹理高度（像素）。
    height: u32,
}

/// 生成一个后台线程，在配置的缩放级别下载所有高德卫星瓦片。
fn spawn_tile_downloads(state: ResMut<TileLoadState>) {
    // 创建无界通道，将接收端存回资源供主线程轮询。
    let (tx, rx) = mpsc::channel();
    *state.receiver.lock().unwrap() = Some(rx);

    std::thread::spawn(move || {
        let num_tiles = 1u32 << ZOOM;

        // 复用一个带 UA 与 15s 超时的 ureq agent，避免每请求重建连接。
        let agent = ureq::AgentBuilder::new()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) CesiumRust/0.1")
            .timeout(std::time::Duration::from_secs(15))
            .build();

        let mut success_count = 0u32;
        let total = num_tiles * num_tiles;

        for ty in 0..num_tiles {
            for tx_px in 0..num_tiles {
                // 高德卫星影像（style=6），轮换子域名
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
                                // 立即发送该瓦片（渐进式加载）
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

/// 接收已下载瓦片并将纹理应用到地球瓦片实体的系统。
fn apply_tile_textures(
    mut state: ResMut<TileLoadState>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    tile_query: Query<(&GlobeTile, &MeshMaterial3d<StandardMaterial>)>,
) {
    // 尝试接收所有可用结果（非阻塞，批量）
    // 未启动下载（receiver 为 None）时直接返回。
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
        // 按 (x,y,z) 匹配 GlobeTile 组件，命中则换纹理并跳出。
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
                "[TileLoader] Applied {}/{} tile textures",
                state.tiles_received, state.total_tiles
            );
        }
    }
}
