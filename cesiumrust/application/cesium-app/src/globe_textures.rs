//! 从 dynamic_globe 黄金路径抽取的纹理工具（M1.5）。
//!
//! 由薄壳 `dynamic_globe.rs` 黄金路径共享。每个函数都是原单体
//! 的逐字节提升（冻结的 `dynamic_globe_legacy.rs` A/B 臂已于
//! 2026-09-27 退役，因 G4 已证明薄壳/legacy 像素中性）—— 无逻辑变更，
//! 仅为模块边界变更。

use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;

/// 向基层追加一个盒滤波的 mip 链（每级 2x2 平均），返回完整数据
/// 块与 mip 层级数。
///
/// Original: `dynamic_globe.rs:1824-1859` (逐字节保留).
pub fn build_mip_chain(base: Vec<u8>, width: u32, height: u32) -> (Vec<u8>, u32) {
    // 从基础层出发，逐级折半生成 mip，追加到同一 data 块尾部。
    let mut data = base;
    let mut levels = 1u32;
    let (mut cw, mut ch) = (width, height);
    let mut src_off = 0usize;
    // 当前层尺寸缩到 1×1 前持续细分。
    while cw > 1 || ch > 1 {
        let nw = (cw / 2).max(1);
        let nh = (ch / 2).max(1);
        let mut mip = vec![0u8; (nw * nh * 4) as usize];
        for ry in 0..nh {
            for rx in 0..nw {
                // 目标 mip 每像素 = 源 2×2 邻域的平均。
                let mut acc = [0u32; 4];
                for dy in 0..2u32 {
                    for dx in 0..2u32 {
                        let sx = ((rx * 2 + dx).min(cw - 1)) as usize;
                        let sy = ((ry * 2 + dy).min(ch - 1)) as usize;
                        let i = src_off + (sy * cw as usize + sx) * 4;
                        for c in 0..4 {
                            acc[c] += data[i + c] as u32;
                        }
                    }
                }
                let o = ((ry * nw + rx) * 4) as usize;
                for c in 0..4 {
                    mip[o + c] = (acc[c] / 4) as u8;
                }
            }
        }
        src_off += (cw * ch) as usize * 4;
        data.extend_from_slice(&mip);
        cw = nw;
        ch = nh;
        levels += 1;
    }
    (data, levels)
}

/// 从 worker 预先准备好的 RGBA 数据（基础层 + mip 链已在帧线程外
/// 构建完毕）创建一张 GPU 纹理，并使用 CesiumJS 影像采样器（三线性
/// mipmap + 各向异性过滤），使缩小的地平线瓦片不闪烁、斜视瓦片保持清晰。
///
/// Original: `dynamic_globe.rs:1866-1898` (逐字节保留).
/// sRGB format: `Rgba8UnormSrgb` + `base_color = WHITE` (硬约束).
pub fn make_image(
    images: &mut Assets<Image>,
    data: Vec<u8>,
    width: u32,
    height: u32,
    levels: u32,
) -> Handle<Image> {
    // 基础层占据 data 前 width*height*4 字节，其余为 mip 链。
    let base_len = (width * height * 4) as usize;
    let mut img = Image::new(
        bevy::render::render_resource::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data[..base_len].to_vec(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    img.data = data;
    img.texture_descriptor.mip_level_count = levels;
    // 装配三线性 mipmap + 8x 各向异性采样器。
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    images.add(img)
}

/// 平滑度 + 调色板度量：相隔 4px 的采样像素对之间的平均 / 最大 RGB
/// 差异，加上各通道均值。
///
/// Original: `dynamic_globe.rs:1765-1801` (逐字节保留).
pub fn smoothness_stats(rgba: &image::RgbaImage) -> (f64, u32, u32, u32, u32) {
    let (w, h) = rgba.dimensions();
    let mut sum: f64 = 0.0;
    let mut maxd: u32 = 0;
    let mut count: u32 = 0;
    let mut acc_r: u64 = 0;
    let mut acc_g: u64 = 0;
    let mut acc_b: u64 = 0;
    let mut x = 0;
    // 水平每隔 8px、垂直每隔 8 行做稀疏采样。
    while x + 4 < w {
        for y in (0..h).step_by(8) {
            // 比较相隔 4px 的像素对，累加 RGB 绝对差。
            let p = rgba.get_pixel(x, y).0;
            let q = rgba.get_pixel(x + 4, y).0;
            let d = ((p[0] as i32 - q[0] as i32).abs()
                + (p[1] as i32 - q[1] as i32).abs()
                + (p[2] as i32 - q[2] as i32).abs()) as u32;
            sum += d as f64;
            if d > maxd {
                maxd = d;
            }
            acc_r += p[0] as u64;
            acc_g += p[1] as u64;
            acc_b += p[2] as u64;
            count += 1;
        }
        x += 8;
    }
    // 无采样点时返回哨兵值（视为最不平滑）。
    if count == 0 {
        return (f64::MAX, u32::MAX, 0, 0, 0);
    }
    (
        sum / count as f64,
        maxd,
        (acc_r / count as u64) as u32,
        (acc_g / count as u64) as u32,
        (acc_b / count as u64) as u32,
    )
}

/// 当解码后的图像是 Bing 那种平滑、明亮、冷色调的无影像占位图，
/// 而非真实卫星影像时返回 true。
///
/// Original: `dynamic_globe.rs:1811-1819` (逐字节保留).
pub fn is_placeholder_tile(rgba: &image::RgbaImage) -> (bool, f64, u32) {
    let (w, h) = rgba.dimensions();
    // 过小图像统计不可靠，直接判定非占位图。
    if w < 16 || h < 16 {
        return (false, f64::MAX, u32::MAX);
    }
    let (avg, maxd, r, _g, b) = smoothness_stats(rgba);
    // 占位图特征：平滑、明亮、偏冷色调。
    let bright = (r + b) / 2 > 170;
    let cool = b >= r;
    (avg < 1.5 && maxd <= 12 && bright && cool, avg, maxd)
}
