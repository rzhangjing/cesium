// legacy CesiumJS-port style debt (deferred.md #18); revisit at M13 lint-cleanup 或本文件在其里程碑被重写时
//! 影像混合系统：把一个地形瓦片上多层影像逐像素合成为一张混合纹理。
//!
//! [`imagery_blend_compute_system`] 对尚未缓存的地形瓦片调用
//! [`blend_imagery_for_tile`] 进层合成并存入 [`ImageryBlendCache`]；
//! [`imagery_apply_system`] 为将混合纹理贴回地形材质预留接口。
//! 当前采样为占位实现（固定灰色），合成逻辑走领域层 `composite_layers`。
#![allow(clippy::too_many_arguments)]
use std::collections::HashMap;

use bevy::prelude::*;
use cesium_imagery::blending::{composite_layers, PixelColor};

use crate::components::{CesiumImageryLayer, CesiumTerrainTile};

use super::layer_manager::ImageryLayerManager;
use super::tile_loader::ImageryCache;

/// 已合成的分层纹理缓存（按 (x,y,level) 索引）。
#[derive(Resource, Default)]
pub struct ImageryBlendCache {
    /// 瓦片坐标→已合成的混合纹理 handle。
    pub layered_textures: HashMap<(u32, u32, u32), Handle<Image>>,
}

/// 在图像 (u,v) 处采样颜色（当前为占位实现，固定中灰）。
///
/// # 参数
/// - `_image`：源图像（未使用）
/// - `_u`/`_v`：归一化纹理坐标
///
/// # 返回
/// 不透明的中灰色像素。
fn sample_color(
    _image: &Image,
    _u: f32,
    _v: f32,
) -> PixelColor {
    PixelColor::opaque(0.5, 0.5, 0.5)
}

/// 为单个地形瓦片合成多层影像，返回 RGBA 字节缓冲。
///
/// # 参数
/// - `images`：图像资产读取器
/// - `cache`：影像纹理缓存（按层/瓦片索引起始 handle）
/// - `layer_entities`：参与混合的 (层号, 不透明度) 列表
/// - `x`/`y`/`level`：瓦片坐标
/// - `_output_size`：保留参数（当前固定 16×16）
///
/// # 返回
/// 合成后的 RGBA 字节；当前实现总是返回 `Some`。
fn blend_imagery_for_tile(
    images: &Assets<Image>,
    cache: &ImageryCache,
    _layer_manager: &ImageryLayerManager,
    layer_entities: &[(u64, f32)],
    x: u32,
    y: u32,
    level: u32,
    _output_size: u32,
) -> Option<Vec<u8>> {
    let _layers: Vec<&cesium_imagery::ImageryLayer> = Vec::new();

    // 低分辨率代理网格（16×16），逐像素采样并合成。
    let cell_count = 16;
    let mut output = Vec::with_capacity((cell_count * cell_count * 4) as usize);

    for v in 0..cell_count {
        for u in 0..cell_count {
            // 格点坐标→归一化 [0,1] 纹理坐标。
            let uf = u as f32 / (cell_count - 1) as f32;
            let vf = v as f32 / (cell_count - 1) as f32;

            // 逐层取当前瓦片的纹理并采样颜色。
            let mut colors = Vec::new();
            for (layer_id, _opacity) in layer_entities {
                let key = (*layer_id, x, y, level);
                if let Some(_handle) = cache.textures.get(&key) {
                    if let Some(image) = images.get(_handle) {
                        let color = sample_color(image, uf, vf);
                        colors.push(color);
                    }
                }
            }

            if colors.is_empty() {
                // 无任何可用层：填不透明灰色占位。
                let grey = (128u8, 128u8, 128u8, 255u8);
                output.extend_from_slice(&[grey.0, grey.1, grey.2, grey.3]);
            } else {
                // 单层直接取色，多层走领域合成。
                let composite = if colors.len() == 1 {
                    colors[0]
                } else {
                    let empty_layers: Vec<&cesium_imagery::ImageryLayer> = Vec::new();
                    composite_layers(
                        &empty_layers,
                        &colors,
                        true,
                        PixelColor::TRANSPARENT,
                    )
                };
                // 浮点 [0,1]→[0,255] 字节并追写到输出。
                let r = (composite.r.clamp(0.0, 1.0) * 255.0) as u8;
                let g = (composite.g.clamp(0.0, 1.0) * 255.0) as u8;
                let b = (composite.b.clamp(0.0, 1.0) * 255.0) as u8;
                let a = (composite.a.clamp(0.0, 1.0) * 255.0) as u8;
                output.extend_from_slice(&[r, g, b, a]);
            }
        }
    }

    Some(output)
}

/// 合成调度系统：遍历地形瓦片，对未缓存的瓦片计算分层混合纹理。
///
/// # 参数
/// - `images`：图像资产写入器
/// - `cache`：影像纹理缓存
/// - `layer_manager`：图层管理（总开关）
/// - `imagery_query`：影像层组件
/// - `terrain_query`：地形瓦片组件
/// - `blend_cache`：混合结果缓存（可写）
pub fn imagery_blend_compute_system(
    mut images: ResMut<Assets<Image>>,
    cache: Res<ImageryCache>,
    layer_manager: Res<ImageryLayerManager>,
    imagery_query: Query<&CesiumImageryLayer>,
    terrain_query: Query<&CesiumTerrainTile>,
    mut blend_cache: ResMut<ImageryBlendCache>,
) {
    // 总开关关闭：不做任何混合。
    if !layer_manager.enabled {
        return;
    }

    // 收集参与混合的层（层号 + 不透明度）。
    let layers: Vec<(u64, f32)> = imagery_query
        .iter()
        .map(|l| (l.layer_index as u64, l.opacity))
        .collect();

    for terrain in terrain_query.iter() {
        // 已有缓存的瓦片跳过（幂等）。
        let key = (terrain.x, terrain.y, terrain.level);
        if blend_cache.layered_textures.contains_key(&key) {
            continue;
        }

        // 合成成功则封装为纹理写回缓存。
        if let Some(data) = blend_imagery_for_tile(
            &images,
            &cache,
            &layer_manager,
            &layers,
            terrain.x,
            terrain.y,
            terrain.level,
            256,
        ) {
            let bevy_img = crate::create_imagery_texture(16, 16, data);
            let handle = images.add(bevy_img);
            blend_cache.layered_textures.insert(key, handle);
        }
    }
}

/// 应用系统：把混合纹理贴回地形瓦片材质（当前为预留接口）。
///
/// # 参数
/// - `blend_cache`：混合结果缓存
/// - `terrain_query`：地形瓦片实体与组件
pub fn imagery_apply_system(
    blend_cache: Res<ImageryBlendCache>,
    terrain_query: Query<(Entity, &CesiumTerrainTile)>,
    _commands: Commands,
    _materials: ResMut<Assets<StandardMaterial>>,
    _children_query: Query<&Children>,
    _mesh_query: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    for (_entity, terrain) in terrain_query.iter() {
        // 按瓦片坐标查找已合成的混合纹理。
        let key = (terrain.x, terrain.y, terrain.level);
        if let Some(_blend_handle) = blend_cache.layered_textures.get(&key) {
            // Would apply the texture to the terrain tile material here
            let _ = &_materials;
            let _ = &_mesh_query;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证默认缓存为空。
    fn test_blend_cache_default() {
        let cache = ImageryBlendCache::default();
        assert!(cache.layered_textures.is_empty());
    }

    #[test]
    /// 验证插入后按键可查到。
    fn test_blend_cache_insert() {
        let mut cache = ImageryBlendCache::default();
        cache.layered_textures.insert((0, 0, 0), Handle::Weak(AssetId::<Image>::invalid()));
        assert!(cache.layered_textures.contains_key(&(0, 0, 0)));
    }

    #[test]
    /// 验证占位采样器总返回不透明中灰。
    fn test_sample_color_default() {
        let img = Image::new(
            bevy::render::render_resource::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            vec![128, 128, 128, 255],
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        let color = sample_color(&img, 0.5, 0.5);
        assert!((color.r - 0.5).abs() < 1e-6);
        assert!((color.g - 0.5).abs() < 1e-6);
    }
}
