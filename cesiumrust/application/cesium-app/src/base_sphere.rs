//! Base-sphere module extracted from the dynamic_globe golden path (M1.5).
//!
//! Contains the `BaseSphereMarker` component, the whole-globe composite
//! resource/helpers, and the image utilities for the non-LOD fallback sphere.
//! Shared by both the thin-shell `dynamic_globe.rs` and the frozen
//! `dynamic_globe_legacy.rs` — **byte-identical logic**, only module boundary.
//!
//! Original locations in `dynamic_globe.rs`:
//! - `BaseSphereMarker`: L1904-1905
//! - `COMPOSITE_TILE`/`COMPOSITE_SIZE`: L1907-1908
//! - `BaseSphereComposite`: L1910-1914
//! - `box_downsample`: L1995-2017
//! - `ocean_block`: L2021-2027
//! - `make_clamped_image`: L2031-2062

// frozen legacy golden-path style debt; local allow to satisfy strict CI clippy gate
#![allow(clippy::type_complexity)]

use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use std::sync::{mpsc, Mutex};

use crate::globe_textures::build_mip_chain;

// ── Component ────────────────────────────────────────────────────────────

/// Marker for the non-LOD base sphere so the composite system can find its
/// material and drape the baked whole-globe texture over it.
///
/// Original: `dynamic_globe.rs:1904-1905` (逐字节保留).
#[derive(Component)]
pub struct BaseSphereMarker;

// ── Constants ────────────────────────────────────────────────────────────

/// Per-tile block size inside the composite (128 px).
pub const COMPOSITE_TILE: u32 = 128;
/// Composite texture size: z=3 → 8×8 tiles × 128 px = 1024 px.
pub const COMPOSITE_SIZE: u32 = 8 * COMPOSITE_TILE;

// ── Resource ─────────────────────────────────────────────────────────────

/// Tracks the async whole-globe composite bake state.
///
/// Original: `dynamic_globe.rs:1910-1914` (逐字节保留).
#[derive(Resource, Default)]
pub struct BaseSphereComposite {
    pub rx: Option<Mutex<mpsc::Receiver<(Vec<u8>, u32)>>>,
    pub done: bool,
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Box-downsample an RGBA tile of width `w` (power-of-two multiple of the
/// target) to `target` × `target`.
///
/// Original: `dynamic_globe.rs:1995-2017` (逐字节保留).
pub fn box_downsample(src: &[u8], w: u32, target: u32) -> Vec<u8> {
    let factor = (w / target).max(1);
    let mut out = vec![0u8; (target * target * 4) as usize];
    for y in 0..target {
        for x in 0..target {
            let mut acc = [0u32; 4];
            for dy in 0..factor {
                for dx in 0..factor {
                    let i = (((y * factor + dy) * w + (x * factor + dx)) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += src[i + c] as u32;
                    }
                }
            }
            let n = factor * factor;
            let o = ((y * target + x) * 4) as usize;
            for c in 0..4 {
                out[o + c] = (acc[c] / n) as u8;
            }
        }
    }
    out
}

/// Raw (unlit) deep-ocean blue close to Bing imagery water pixels, for
/// composite blocks whose tile has no imagery.
///
/// Original: `dynamic_globe.rs:2021-2027` (逐字节保留).
pub fn ocean_block() -> Vec<u8> {
    let mut v = Vec::with_capacity((COMPOSITE_TILE * COMPOSITE_TILE * 4) as usize);
    for _ in 0..COMPOSITE_TILE * COMPOSITE_TILE {
        v.extend_from_slice(&[12, 28, 44, 255]);
    }
    v
}

/// Like `make_image` but clamped at the edges: the composite is a single
/// whole-globe Mercator image, not a repeating tile.
///
/// Original: `dynamic_globe.rs:2031-2062` (逐字节保留).
/// sRGB: `Rgba8UnormSrgb` (硬约束).
pub fn make_clamped_image(
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
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        ..default()
    });
    images.add(img)
}

/// Assemble the 1024×1024 whole-globe composite from 8×8 base-layer blocks
/// on a background thread, returning the channel receiver.
///
/// Original: `dynamic_globe.rs:1974-1990` (the thread-spawn portion of
/// `base_sphere_composite_system`). Extracted so both thin-shell and legacy
/// share the identical bake logic.
pub fn spawn_composite_bake(blocks: Vec<(u32, u32, Vec<u8>)>) -> Mutex<mpsc::Receiver<(Vec<u8>, u32)>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut comp = vec![0u8; (COMPOSITE_SIZE * COMPOSITE_SIZE * 4) as usize];
        for (bx, by, blk) in blocks {
            let x0 = bx * COMPOSITE_TILE;
            let y0 = by * COMPOSITE_TILE;
            for row in 0..COMPOSITE_TILE {
                let src = (row * COMPOSITE_TILE * 4) as usize;
                let dst = ((y0 + row) * COMPOSITE_SIZE * 4 + x0 * 4) as usize;
                comp[dst..dst + (COMPOSITE_TILE * 4) as usize]
                    .copy_from_slice(&blk[src..src + (COMPOSITE_TILE * 4) as usize]);
            }
        }
        let (chain, levels) = build_mip_chain(comp, COMPOSITE_SIZE, COMPOSITE_SIZE);
        let _ = tx.send((chain, levels));
    });
    Mutex::new(rx)
}
