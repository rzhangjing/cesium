//! Web Mercator XYZ imagery provider with CPU reprojection into the globe's
//! geographic (equirectangular) tile grid.
//!
//! The globe imagery pipeline addresses every tile through a geographic tiling
//! scheme (2×1 root, row 0 = north) and samples whatever a provider returns
//! into that geographic tile (see
//! `globe_surface_tile_provider::compose_tile_imagery`). Mainstream satellite
//! tile services (Esri World Imagery, Bing, OSM, …) are Web Mercator, so a raw
//! Mercator tile cannot be dropped into a geographic tile without latitude
//! skew. This provider closes that gap (the "DEVIATION B4-4" reprojection
//! point): for each requested geographic tile it re-projects pixel by pixel —
//! every output texel's lon/lat is mapped into the Web Mercator tile grid,
//! bilinearly sampled from the covering Mercator tiles, and re-encoded as a
//! PNG in the geographic orientation the pipeline expects.
//!
//! Non-blocking + persistent (the "同步下载 / 磁盘缓存" upgrade):
//! - a background worker thread owns the HTTP client, so the render thread
//!   never blocks on the network;
//! - a two-level cache (memory ← disk): a tile already downloaded in a previous
//!   run is served from disk without any network access, so an offline run
//!   still shows everything it cached while online;
//! - an uncached tile triggers an async download and reports
//!   [`TileImageAvailability::Transient`] for this frame — the base map (a
//!   lower layer) shows through and the tile is retried next frame, once the
//!   worker has filled the cache. A tile that permanently failed (offline and
//!   never cached, or HTTP 404) contributes transparency, so the base map (a
//!   lower layer) shows through — the provider never reports `NoData`, which
//!   would otherwise make the whole composition yield and hide the base map.

use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

use cesium_core::rectangle::Rectangle;
use image::{ImageFormat, Rgba, RgbaImage};

use crate::imagery_provider::{ImageryProvider, TileImageAvailability};

/// Web Mercator latitude limit (°): beyond this the projection is undefined
/// and no tiles exist (`atan(sinh(π))`).
const MERCATOR_MAX_LAT: f64 = 85.0511287798066;

/// Upper bound on cached fully-resolved reprojections before the cache is
/// cleared. The reprojection (per-texel Mercator math + bilinear sample + PNG
/// encode) is the provider's hot path; caching the settled result keeps a
/// throttled re-compose from redoing it. Bounded so a long session cannot grow
/// it without limit.
const GEO_CACHE_MAX: usize = 512;

/// A Mercator tile identity: `(zoom, x, y)`.
type TileKey = (u32, u32, u32);

/// A decoded Mercator tile.
struct DecodedTile {
    /// Straight RGBA bytes, `size × size`.
    pixels: Vec<u8>,
    /// Edge length in pixels (256 for standard XYZ).
    size: u32,
}

/// State shared between the provider (lookups, enqueue) and the worker thread
/// (download results).
struct SharedState {
    /// `Some(tile)` = ready; `None` = permanently failed (404 / offline).
    cache: HashMap<TileKey, Option<Arc<DecodedTile>>>,
    /// Keys already handed to the worker but not yet resolved — prevents
    /// re-enqueueing the same tile every frame.
    pending: HashSet<TileKey>,
}

/// Result of a cache lookup for one Mercator tile.
enum TileStatus {
    /// Decoded pixels available (memory or disk).
    Ready(Arc<DecodedTile>),
    /// Known permanent failure — do not retry.
    Failed,
    /// Not cached anywhere yet — a download is needed.
    Missing,
}

/// A Web Mercator XYZ imagery provider that re-projects into geographic tiles,
/// downloads on a background thread, and caches to disk.
pub struct WebMercatorImageryProvider {
    /// URL template with `{z}`, `{x}`, `{y}` placeholders.
    url_template: String,
    /// Full-globe rectangle (geographic coverage).
    rectangle: Rectangle,
    /// Output (and source) tile edge in pixels.
    tile_size: u32,
    /// Highest Mercator zoom to request.
    maximum_level: u32,
    /// Memory + pending bookkeeping, shared with the worker.
    shared: Arc<Mutex<SharedState>>,
    /// On-disk tile cache root (raw encoded bytes), when enabled.
    disk_dir: Option<PathBuf>,
    /// Sends download requests to the worker thread. Wrapped in a `Mutex`
    /// because `mpsc::Sender` is `Send` but not `Sync`, while the provider
    /// must be `Sync` (DEVIATION B4-6: shared with compose threads).
    tx: Mutex<Sender<TileKey>>,
    /// Fully-resolved geographic reprojections, keyed by `(level, gx, gy)`.
    /// Only settled (non-`Transient`) results are stored, so a hit is stable.
    geo_cache: Mutex<HashMap<TileKey, Vec<u8>>>,
}

impl WebMercatorImageryProvider {
    /// Creates a provider over a `{z}/{x}/{y}` Mercator tile template.
    ///
    /// - `maximum_level` caps the Mercator zoom requested (bounds the download
    ///   count for a from-orbit view).
    /// - `disk_dir` enables a persistent tile cache; pass `None` for memory-only.
    pub fn new(url_template: &str, maximum_level: u32, disk_dir: Option<PathBuf>) -> Self {
        let shared = Arc::new(Mutex::new(SharedState {
            cache: HashMap::new(),
            pending: HashSet::new(),
        }));
        let (tx, rx) = channel::<TileKey>();
        {
            let worker_shared = Arc::clone(&shared);
            let worker_url = url_template.to_string();
            let worker_disk = disk_dir.clone();
            // Detached worker: it lives for the process and exits when the
            // provider's sender drops. The render thread never blocks on it.
            std::thread::spawn(move || download_worker(worker_shared, rx, worker_url, worker_disk));
        }
        Self {
            url_template: url_template.to_string(),
            rectangle: Rectangle::new(
                -PI,
                -std::f64::consts::FRAC_PI_2,
                PI,
                std::f64::consts::FRAC_PI_2,
            ),
            tile_size: 256,
            maximum_level,
            shared,
            disk_dir,
            tx: Mutex::new(tx),
            geo_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Path of a tile in the on-disk cache (raw encoded bytes, no extension).
    fn disk_path(&self, key: TileKey) -> Option<PathBuf> {
        let (z, x, y) = key;
        self.disk_dir.as_ref().map(|d| d.join(format!("{z}/{x}/{y}")))
    }

    /// Loads + decodes a tile from the on-disk cache, if present and valid.
    fn load_from_disk(&self, key: TileKey) -> Option<Arc<DecodedTile>> {
        let path = self.disk_path(key)?;
        let bytes = std::fs::read(path).ok()?;
        decode_tile(&bytes)
    }

    /// Resolves one tile through memory ← disk, without triggering a download.
    fn lookup(&self, key: TileKey) -> TileStatus {
        if let Ok(s) = self.shared.lock() {
            match s.cache.get(&key) {
                Some(Some(t)) => return TileStatus::Ready(t.clone()),
                Some(None) => return TileStatus::Failed,
                None => {}
            }
        }
        if let Some(t) = self.load_from_disk(key) {
            if let Ok(mut s) = self.shared.lock() {
                s.cache.insert(key, Some(t.clone()));
            }
            return TileStatus::Ready(t);
        }
        TileStatus::Missing
    }

    /// Enqueues a download for a missing tile (deduplicated via `pending`).
    fn request_download(&self, key: TileKey) {
        let mut enqueue = false;
        if let Ok(mut s) = self.shared.lock() {
            if !s.cache.contains_key(&key) && s.pending.insert(key) {
                enqueue = true;
            }
        }
        if enqueue {
            if let Ok(tx) = self.tx.lock() {
                let _ = tx.send(key);
            }
        }
    }
}

/// Decodes raw image bytes (PNG/JPEG/…) into a [`DecodedTile`].
fn decode_tile(bytes: &[u8]) -> Option<Arc<DecodedTile>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = img.width();
    Some(Arc::new(DecodedTile {
        pixels: img.into_raw(),
        size,
    }))
}

/// Background download loop: owns the blocking HTTP client, persists successful
/// fetches to disk, and publishes results into the shared memory cache.
fn download_worker(
    shared: Arc<Mutex<SharedState>>,
    rx: Receiver<TileKey>,
    url_template: String,
    disk_dir: Option<PathBuf>,
) {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("cesium-rs-viewer-demo")
        .build()
        .unwrap_or_default();
    while let Ok((z, x, y)) = rx.recv() {
        let url = url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string());
        let bytes_opt = match client.get(&url).send() {
            Ok(resp) if resp.status().is_success() => resp.bytes().ok().map(|b| b.to_vec()),
            Ok(resp) => {
                log::debug!("web mercator tile {z}/{x}/{y} ({url}) → http {}", resp.status());
                None
            }
            Err(e) => {
                // Offline / unreachable is an expected mode (the disk cache is
                // the fallback), so a failed fetch is debug-level, not a
                // warning; the tile is cached as a permanent failure and the
                // base map shows through.
                log::debug!("web mercator tile {z}/{x}/{y} ({url}) → fetch error: {e}");
                None
            }
        };
        let result = bytes_opt.and_then(|bytes| {
            if let Some(dir) = &disk_dir {
                let path = dir.join(format!("{z}/{x}/{y}"));
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&path, &bytes);
            }
            decode_tile(&bytes)
        });
        if let Ok(mut s) = shared.lock() {
            s.pending.remove(&(z, x, y));
            s.cache.insert((z, x, y), result);
        }
    }
}

/// Bilinearly samples a rectangular RGBA buffer (`w`×`h`) at the fractional
/// pixel coordinate `(fx, fy)` (pixel centers at integer + 0.5), clamping at
/// the buffer edges. Used for the stitched reprojection atlas, which is not
/// square (per-tile clamped sampling is what produced the seam grid).
fn sample_rect(pixels: &[u8], w: usize, h: usize, fx: f64, fy: f64) -> [u8; 4] {
    let (w_i, h_i) = (w as i64, h as i64);
    let gx = fx - 0.5;
    let gy = fy - 0.5;
    let x0 = gx.floor() as i64;
    let y0 = gy.floor() as i64;
    let frx = gx - x0 as f64;
    let fry = gy - y0 as f64;
    let tap = |ix: i64, iy: i64| -> [f32; 4] {
        let cx = ix.clamp(0, w_i - 1) as usize;
        let cy = iy.clamp(0, h_i - 1) as usize;
        let i = (cy * w + cx) * 4;
        [
            pixels[i] as f32,
            pixels[i + 1] as f32,
            pixels[i + 2] as f32,
            pixels[i + 3] as f32,
        ]
    };
    let c00 = tap(x0, y0);
    let c10 = tap(x0 + 1, y0);
    let c01 = tap(x0, y0 + 1);
    let c11 = tap(x0 + 1, y0 + 1);
    let w00 = ((1.0 - frx) * (1.0 - fry)) as f32;
    let w10 = (frx * (1.0 - fry)) as f32;
    let w01 = ((1.0 - frx) * fry) as f32;
    let w11 = (frx * fry) as f32;
    let mut out = [0u8; 4];
    for ch in 0..4 {
        let v = c00[ch] * w00 + c10[ch] * w10 + c01[ch] * w01 + c11[ch] * w11;
        out[ch] = v.round().clamp(0.0, 255.0) as u8;
    }
    out
}

impl ImageryProvider for WebMercatorImageryProvider {
    fn url(&self) -> &str {
        &self.url_template
    }
    fn proxy(&self) -> Option<&str> {
        None
    }
    fn rectangle(&self) -> &Rectangle {
        &self.rectangle
    }
    fn tile_width(&self) -> u32 {
        self.tile_size
    }
    fn tile_height(&self) -> u32 {
        self.tile_size
    }
    fn maximum_level(&self) -> Option<u32> {
        Some(self.maximum_level)
    }
    fn minimum_level(&self) -> Option<u32> {
        Some(0)
    }
    fn has_water_mask(&self) -> bool {
        false
    }
    fn is_ready(&self) -> bool {
        true
    }

    fn request_image(&self, x: u32, y: u32, level: u32) -> Option<Vec<u8>> {
        match self.request_tile_image_availability(x, y, level) {
            TileImageAvailability::Data(data) => Some(data),
            _ => None,
        }
    }

    fn request_tile_image_availability(
        &self,
        gx: u32,
        gy: u32,
        level: u32,
    ) -> TileImageAvailability {
        // A previously settled reprojection for this geographic tile is stable
        // (its covering Mercator tiles stay Ready), so reuse it and skip the
        // per-texel reprojection + PNG encode entirely.
        if let Ok(cache) = self.geo_cache.lock() {
            if let Some(bytes) = cache.get(&(level, gx, gy)) {
                return TileImageAvailability::Data(bytes.clone());
            }
        }

        // Geographic tile bounds (degrees): 2^(level+1) columns × 2^level rows,
        // row 0 at the north — matching the pipeline's addressing.
        let columns = (2u32 << level) as f64;
        let rows = (1u32 << level) as f64;
        let lon_min = -180.0 + (gx as f64) * (360.0 / columns);
        let lon_max = lon_min + 360.0 / columns;
        let lat_max = 90.0 - (gy as f64) * (180.0 / rows);
        let lat_min = lat_max - 180.0 / rows;

        // One Mercator zoom step finer than the geographic level keeps ground
        // resolution comparable (Mercator has 2^z columns vs geographic 2^(z+1)).
        let zm = (level + 1).min(self.maximum_level);
        let n = (1u32 << zm) as f64;
        let size = self.tile_size;

        // Continuous Mercator-tile coordinates for a lon/lat.
        let merc_x = |lon: f64| (lon + 180.0) / 360.0 * n;
        let merc_y = |lat: f64| {
            let lr = lat.clamp(-MERCATOR_MAX_LAT, MERCATOR_MAX_LAT).to_radians();
            (1.0 - (lr.tan() + 1.0 / lr.cos()).ln() / PI) / 2.0 * n
        };

        // Set of Mercator tiles covering this geographic tile (clamped to grid).
        let max_tile = (n - 1.0) as u32;
        let tx0 = (merc_x(lon_min).floor().max(0.0) as u32).min(max_tile);
        let tx1 = (merc_x(lon_max).floor().max(0.0) as u32).min(max_tile);
        let ty0 = (merc_y(lat_max).floor().max(0.0) as u32).min(max_tile); // north
        let ty1 = (merc_y(lat_min).floor().max(0.0) as u32).min(max_tile); // south

        let cols = (tx1 - tx0 + 1) as usize;
        let rws = (ty1 - ty0 + 1) as usize;
        let mut tiles: Vec<Option<Arc<DecodedTile>>> = Vec::with_capacity(cols * rws);
        let mut has_missing = false;
        for ty in ty0..=ty1 {
            for tx in tx0..=tx1 {
                match self.lookup((zm, tx, ty)) {
                    TileStatus::Ready(t) => tiles.push(Some(t)),
                    TileStatus::Failed => tiles.push(None),
                    TileStatus::Missing => {
                        self.request_download((zm, tx, ty));
                        has_missing = true;
                        tiles.push(None);
                    }
                }
            }
        }

        // Still downloading: Transient so the base map shows through this frame;
        // retried next frame (the pipeline never caches a Transient result).
        if has_missing {
            return TileImageAvailability::Transient;
        }
        // A fully-permanent failure (offline & never cached, or 404) leaves
        // every sampled tile empty, so the reprojection below yields an all-
        // transparent Data that lets the base map show through. We MUST NOT
        // return NoData here: the composition aggregates NoData by yielding
        // the whole tile (see compose_tile_imagery), which would hide the base
        // map that already rendered underneath this satellite layer.

        // Stitch the covering Mercator tiles into one contiguous atlas with a
        // 1-pixel replicated border. Sampling the atlas (rather than clamping
        // inside each individual tile) lets the bilinear filter cross internal
        // Mercator tile boundaries, removing the visible seam grid that
        // per-tile clamped sampling produced at every tile edge.
        let atlas_w = cols * size as usize + 2;
        let atlas_h = rws * size as usize + 2;
        let mut atlas = vec![0u8; atlas_w * atlas_h * 4];
        for (idx, tile) in tiles.iter().enumerate() {
            let col = idx % cols;
            let row = idx / cols;
            let Some(tile) = tile else { continue };
            for y in 0..size as usize {
                let src_row = (y * tile.size as usize) * 4;
                let dst_row = ((row * size as usize + y + 1) * atlas_w
                    + (col * size as usize + 1))
                    * 4;
                let copy_len = size as usize * 4;
                atlas[dst_row..dst_row + copy_len]
                    .copy_from_slice(&tile.pixels[src_row..src_row + copy_len]);
            }
        }
        // Replicate the outer ring so border taps repeat the edge pixel.
        for x in 0..atlas_w {
            for (dst_row, src_row) in [(0usize, 1usize), (atlas_h - 1, atlas_h - 2)] {
                let d = (dst_row * atlas_w + x) * 4;
                let s = (src_row * atlas_w + x) * 4;
                let px = atlas[s..s + 4].to_vec();
                atlas[d..d + 4].copy_from_slice(&px);
            }
        }
        for y in 0..atlas_h {
            for (dst_col, src_col) in [(0usize, 1usize), (atlas_w - 1, atlas_w - 2)] {
                let d = (y * atlas_w + dst_col) * 4;
                let s = (y * atlas_w + src_col) * 4;
                let px = atlas[s..s + 4].to_vec();
                atlas[d..d + 4].copy_from_slice(&px);
            }
        }

        // Re-project: bilinearly sample the atlas into the geographic output
        // (row 0 = north, matching the pipeline's orientation).
        let mut out = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
        for py in 0..size {
            let frac_y = (py as f64 + 0.5) / size as f64;
            let lat = lat_max - frac_y * (lat_max - lat_min);
            let ty_f = merc_y(lat);
            let ay = (ty_f - ty0 as f64) * size as f64 + 1.0;
            for px in 0..size {
                let frac_x = (px as f64 + 0.5) / size as f64;
                let lon = lon_min + frac_x * (lon_max - lon_min);
                let tx_f = merc_x(lon);
                let ax = (tx_f - tx0 as f64) * size as f64 + 1.0;
                let [r, g, b, a] = sample_rect(&atlas, atlas_w, atlas_h, ax, ay);
                out.put_pixel(px, py, Rgba([r, g, b, a]));
            }
        }

        let mut buf = Cursor::new(Vec::new());
        match out.write_to(&mut buf, ImageFormat::Png) {
            Ok(()) => {
                let bytes = buf.into_inner();
                if let Ok(mut cache) = self.geo_cache.lock() {
                    if cache.len() >= GEO_CACHE_MAX {
                        cache.clear();
                    }
                    cache.insert((level, gx, gy), bytes.clone());
                }
                TileImageAvailability::Data(bytes)
            }
            Err(_) => TileImageAvailability::Transient,
        }
    }
}
