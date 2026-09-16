//! Texture utilities extracted from the dynamic_globe golden path (M1.5).
//!
//! Shared by both the thin-shell `dynamic_globe.rs` and the frozen
//! `dynamic_globe_legacy.rs`. Every function is a **byte-identical** lift of
//! the original monolith — no logic changes, only module boundary changes.

use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;

/// Append a box-filtered mip chain (2x2 average per level) to the base
/// level, returning the full data blob and the mip level count.
///
/// Original: `dynamic_globe.rs:1824-1859` (逐字节保留).
pub fn build_mip_chain(base: Vec<u8>, width: u32, height: u32) -> (Vec<u8>, u32) {
    let mut data = base;
    let mut levels = 1u32;
    let (mut cw, mut ch) = (width, height);
    let mut src_off = 0usize;
    while cw > 1 || ch > 1 {
        let nw = (cw / 2).max(1);
        let nh = (ch / 2).max(1);
        let mut mip = vec![0u8; (nw * nh * 4) as usize];
        for ry in 0..nh {
            for rx in 0..nw {
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

/// Create a GPU texture from worker-prepared RGBA data (base level + mip
/// chain already built off the frame thread) with the CesiumJS imagery
/// sampler (trilinear mipmap + anisotropic filtering), so minified horizon
/// tiles don't shimmer and oblique tiles stay crisp.
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
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: 8,
        ..default()
    });
    images.add(img)
}

/// Smoothness + palette metric: average / max RGB difference between
/// sampled pixel pairs 4px apart, plus mean channel levels.
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
    while x + 4 < w {
        for y in (0..h).step_by(8) {
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

/// True when the decoded image is Bing's smooth bright cool-tinted no-imagery
/// placeholder rather than real satellite imagery.
///
/// Original: `dynamic_globe.rs:1811-1819` (逐字节保留).
pub fn is_placeholder_tile(rgba: &image::RgbaImage) -> (bool, f64, u32) {
    let (w, h) = rgba.dimensions();
    if w < 16 || h < 16 {
        return (false, f64::MAX, u32::MAX);
    }
    let (avg, maxd, r, _g, b) = smoothness_stats(rgba);
    let bright = (r + b) / 2 > 170;
    let cool = b >= r;
    (avg < 1.5 && maxd <= 12 && bright && cool, avg, maxd)
}
