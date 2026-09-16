//! Screen-space-error quadtree LOD extracted from the dynamic_globe golden
//! path (M1.5). Byte-identical logic — only module boundary changes.
//!
//! Original locations in `dynamic_globe.rs`:
//! - Constants: L39-44, L75
//! - `compute_segments`: L2066-2069
//! - `focal_pixels`: L1507-1513
//! - `compute_sub_camera_point`: L1719-1730
//! - `Visit` enum: L1563-1567
//! - `compute_visible_tiles`: L1528-1552
//! - `visit_tile`: L1569-1714
//! - Display-set stability (refine_cover / blocked / coarsening): L505-577

// frozen legacy golden-path style debt; local allow to satisfy strict CI clippy gate
#![allow(clippy::type_complexity, clippy::unnecessary_map_or)]

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::orbit_camera::{OrbitState, CAMERA_FOV_Y};

// ── Constants (golden-path verbatim) ─────────────────────────────────────

pub const MIN_ZOOM: u32 = 3;
pub const MAX_ZOOM: u32 = 19;
pub const BASE_SEGMENTS: u32 = 48;
/// WGS84 semi-major axis (meters), for screen-space-error math.
/// METERS_PER_RENDER_UNIT = 6378137 (硬约束).
pub const EARTH_RADIUS_M: f64 = 6378137.0;
pub const MAX_TILE_SCREEN_PX: f64 = 288.0;
/// Coarsest levels kept resident as a permanent global fallback layer.
pub const BASE_LAYER_ZOOM: u32 = 3;

// ── LodContext trait ─────────────────────────────────────────────────────

/// Abstracts the TileManager state that the LOD traversal queries.
/// Both thin-shell and legacy TileManager implement this.
pub trait LodContext {
    fn has_entity(&self, key: &(u32, u32, u32)) -> bool;
    fn tex_size(&self, key: &(u32, u32, u32)) -> Option<u32>;
}

// ── Tile key type ────────────────────────────────────────────────────────

pub type TileKey = (u32, u32, u32);

// ── Functions ────────────────────────────────────────────────────────────

/// Original: `dynamic_globe.rs:2066-2069`.
pub fn compute_segments(zoom: u32) -> u32 {
    (BASE_SEGMENTS >> zoom.saturating_sub(MIN_ZOOM)).max(8)
}

/// Vertical focal length in pixels: (H/2) / tan(fov/2).
/// Original: `dynamic_globe.rs:1507-1513`.
pub fn focal_pixels(windows: &Query<&Window>) -> f64 {
    let h = windows
        .get_single()
        .map(|w| w.height() as f64)
        .unwrap_or(720.0);
    (h * 0.5) / ((CAMERA_FOV_Y as f64) * 0.5).tan()
}

/// Original: `dynamic_globe.rs:1719-1730`.
pub fn compute_sub_camera_point(orbit: &OrbitState) -> (f64, f64) {
    let cos_pitch = orbit.pitch.cos();
    let sin_pitch = orbit.pitch.sin();
    let dir_x = cos_pitch * orbit.heading.cos();
    let dir_y = cos_pitch * orbit.heading.sin();
    let dir_z = sin_pitch;

    let len = (dir_x * dir_x + dir_y * dir_y + dir_z * dir_z).sqrt();
    let lat = (dir_z / len).asin() as f64;
    let lon = (dir_y as f64).atan2(dir_x as f64);
    (lat, lon)
}

/// Traversal outcome — mirror of CesiumJS `TraversalDetails.allAreRenderable`.
/// Original: `dynamic_globe.rs:1563-1567`.
pub enum Visit {
    Ready,
    NotReady,
    Culled,
}

/// CesiumJS-style KICK-aware quadtree partition.
/// Original: `dynamic_globe.rs:1528-1552`.
pub fn compute_visible_tiles<C: LodContext>(
    lat_rad: f64,
    lon_rad: f64,
    distance: f64,
    focal_px: f64,
    ctx: &C,
) -> (Vec<(TileKey, f32)>, Vec<(TileKey, f32)>) {
    let d = distance.max(1.001);
    let cx = lat_rad.cos() * lon_rad.cos();
    let cy = lat_rad.cos() * lon_rad.sin();
    let cz = lat_rad.sin();
    let cap = (1.0 / d).acos();

    let mut render = Vec::new();
    let mut load = Vec::new();
    let n0 = 1u32 << MIN_ZOOM;
    for y in 0..n0 {
        for x in 0..n0 {
            visit_tile(x, y, MIN_ZOOM, cx, cy, cz, d, cap, focal_px, ctx, &mut render, &mut load);
        }
    }
    (render, load)
}

/// Original: `dynamic_globe.rs:1569-1714` (逐字节保留).
#[allow(clippy::too_many_arguments)]
pub fn visit_tile<C: LodContext>(
    x: u32,
    y: u32,
    z: u32,
    cx: f64,
    cy: f64,
    cz: f64,
    d: f64,
    cap: f64,
    focal_px: f64,
    ctx: &C,
    render: &mut Vec<(TileKey, f32)>,
    load: &mut Vec<(TileKey, f32)>,
) -> Visit {
    let n = 1u64 << z;
    let lon = (x as f64 + 0.5) / n as f64 * 2.0 * std::f64::consts::PI
        - std::f64::consts::PI;
    let lat = (std::f64::consts::PI * (1.0 - 2.0 * (y as f64 + 0.5) / n as f64))
        .sinh()
        .atan()
        .clamp(-1.4844, 1.4844);

    let tx = lat.cos() * lon.cos();
    let ty = lat.cos() * lon.sin();
    let tz = lat.sin();

    let dot = (tx * cx + ty * cy + tz * cz).clamp(-1.0, 1.0);
    let theta = dot.acos();
    let margin = 2.0 * std::f64::consts::PI / n as f64;
    if theta > cap + margin {
        return Visit::Culled;
    }

    let ex = tx - cx * d;
    let ey = ty - cy * d;
    let ez = tz - cz * d;
    let dist_m = (ex * ex + ey * ey + ez * ez).sqrt() * EARTH_RADIUS_M;

    let w_m = 2.0 * std::f64::consts::PI * EARTH_RADIUS_M / n as f64;
    let screen_px = w_m / dist_m * focal_px;

    let has_ent = ctx.has_entity(&(x, y, z));

    if screen_px <= MAX_TILE_SCREEN_PX || z >= MAX_ZOOM {
        render.push(((x, y, z), screen_px as f32));
        if has_ent {
            Visit::Ready
        } else {
            Visit::NotReady
        }
    } else {
        let start = render.len();
        let (x2, y2, z1) = (x * 2, y * 2, z + 1);
        let children = [
            (x2, y2, z1),
            (x2 + 1, y2, z1),
            (x2, y2 + 1, z1),
            (x2 + 1, y2 + 1, z1),
        ];
        let outcomes = [
            visit_tile(x2, y2, z1, cx, cy, cz, d, cap, focal_px, ctx, render, load),
            visit_tile(x2 + 1, y2, z1, cx, cy, cz, d, cap, focal_px, ctx, render, load),
            visit_tile(x2, y2 + 1, z1, cx, cy, cz, d, cap, focal_px, ctx, render, load),
            visit_tile(x2 + 1, y2 + 1, z1, cx, cy, cz, d, cap, focal_px, ctx, render, load),
        ];
        let any_selected = outcomes
            .iter()
            .any(|o| matches!(o, Visit::Ready | Visit::NotReady));
        if !any_selected {
            render.push(((x, y, z), screen_px as f32));
            return if ctx.has_entity(&(x, y, z)) {
                Visit::Ready
            } else {
                Visit::NotReady
            };
        }
        let all_ready = outcomes
            .iter()
            .all(|o| matches!(o, Visit::Ready | Visit::Culled));
        if !all_ready {
            render.truncate(start);
            for (c, o) in children.iter().zip(outcomes.iter()) {
                if !matches!(o, Visit::Culled)
                    && (!ctx.has_entity(c) || ctx.tex_size(c).map_or(false, |s| s < 256))
                {
                    load.push((*c, (screen_px * 0.5) as f32));
                }
            }
            render.push(((x, y, z), screen_px as f32));
            if has_ent {
                Visit::Ready
            } else {
                Visit::NotReady
            }
        } else {
            Visit::Ready
        }
    }
}

// ── Display-set stability (CesiumJS allAreRenderable) ────────────────────

/// Compute the stable display set from old/new partitions.
/// Original: `dynamic_globe.rs:505-577` (逐字节保留).
///
/// Returns `(display_set, partition_changed)`.
pub fn compute_display_set(
    old_set: &HashSet<TileKey>,
    new_set: &HashSet<TileKey>,
    new_load_set: &HashSet<TileKey>,
    prev_partition: &HashSet<TileKey>,
    prev_load: &HashSet<TileKey>,
    replacement_ready: &dyn Fn(&TileKey) -> bool,
) -> (HashSet<TileKey>, bool) {
    let mut refine_cover: HashMap<TileKey, Vec<TileKey>> = HashMap::new();
    for n in new_set {
        let (mut ax, mut ay, mut az) = *n;
        while az > 0 {
            ax >>= 1;
            ay >>= 1;
            az -= 1;
            let a = (ax, ay, az);
            if old_set.contains(&a) && !new_set.contains(&a) {
                refine_cover.entry(a).or_default().push(*n);
                break;
            }
        }
    }
    let mut display: HashSet<TileKey> = HashSet::new();
    let mut blocked: HashSet<TileKey> = HashSet::new();
    for old in old_set.iter() {
        if new_set.contains(old) {
            display.insert(*old);
            continue;
        }
        let (mut ax, mut ay, mut az) = *old;
        let mut ancestor: Option<TileKey> = None;
        while az > 0 {
            ax >>= 1;
            ay >>= 1;
            az -= 1;
            if new_set.contains(&(ax, ay, az)) {
                ancestor = Some((ax, ay, az));
                break;
            }
        }
        if let Some(a) = ancestor {
            if replacement_ready(&a) {
                display.insert(a);
            } else {
                display.insert(*old);
                blocked.insert(a);
            }
            continue;
        }
        if let Some(desc) = refine_cover.get(old) {
            if desc.iter().all(replacement_ready) {
                display.extend(desc.iter().copied());
            } else {
                display.insert(*old);
                blocked.extend(desc.iter().copied());
            }
            continue;
        }
    }
    for n in new_set {
        if blocked.contains(n) || display.contains(n) {
            continue;
        }
        let (mut ax, mut ay, mut az) = *n;
        let mut covered = old_set.contains(n);
        while !covered && az > 0 {
            ax >>= 1;
            ay >>= 1;
            az -= 1;
            covered = old_set.contains(&(ax, ay, az));
        }
        if !covered {
            display.insert(*n);
        }
    }
    let partition_changed = new_set != prev_partition || new_load_set != prev_load;
    (display, partition_changed)
}
