//! Web Mercator XYZ imagery provider with CPU reprojection into the globe's
//! geographic (equirectangular) tile grid, plus synchronous HTTP fetch.
//!
//! The globe imagery pipeline addresses every tile through a geographic tiling
//! scheme (2×1 root, row 0 = north) and nearest-samples whatever a provider
//! returns into that geographic tile (see
//! `globe_surface_tile_provider::compose_tile_imagery`). Mainstream satellite
//! tile services (Esri World Imagery, Bing, OSM, …) are Web Mercator, so a raw
//! Mercator tile cannot be dropped into a geographic tile without latitude
//! skew.
//!
//! This provider closes that gap (the "DEVIATION B4-4" reprojection point):
//! for each requested geographic tile it re-projects pixel by pixel — every
//! output texel's lon/lat is mapped into the Web Mercator tile grid, sampled
//! from a cached, lazily-downloaded Mercator tile, and re-encoded as a PNG in
//! the geographic orientation the pipeline expects.
//!
//! Download failures are reported per-tile as [`TileImageAvailability::Transient`]
//! so a missing/unreachable tile never stamps a permanent hole; layered above a
//! local base map it simply reveals that base wherever the network has no data.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use cesium_core::rectangle::Rectangle;
use image::{ImageFormat, Rgba, RgbaImage};

use crate::imagery_provider::{ImageryProvider, TileImageAvailability};

/// Web Mercator latitude limit (°): beyond this the projection is undefined
/// and no tiles exist (`atan(sinh(π))`).
const MERCATOR_MAX_LAT: f64 = 85.0511287798066;

/// A decoded Mercator tile held in the provider cache.
struct DecodedTile {
    /// Straight RGBA bytes, `size × size`.
    pixels: Vec<u8>,
    /// Edge length in pixels (256 for standard XYZ).
    size: u32,
}

/// A Web Mercator XYZ imagery provider that re-projects into geographic tiles.
pub struct WebMercatorImageryProvider {
    /// URL template with `{z}`, `{x}`, `{y}` placeholders.
    url_template: String,
    /// Full-globe rectangle (geographic coverage).
    rectangle: Rectangle,
    /// Output (and source) tile edge in pixels.
    tile_size: u32,
    /// Highest Mercator zoom to request.
    maximum_level: u32,
    /// Lazily-populated tile cache; `None` marks a known-failed fetch so a
    /// dead tile is not re-requested every frame.
    cache: Mutex<HashMap<(u32, u32, u32), Option<Arc<DecodedTile>>>>,
    /// Blocking HTTP client with a bounded timeout.
    client: reqwest::blocking::Client,
    /// Diagnostic counter: cache-miss fetches that succeeded.
    fetched_ok: AtomicU32,
    /// Diagnostic counter: cache-miss fetches that failed.
    fetched_fail: AtomicU32,
}

impl WebMercatorImageryProvider {
    /// Creates a provider over a `{z}/{x}/{y}` Mercator tile template.
    ///
    /// `maximum_level` caps the Mercator zoom requested (keeps the download
    /// count bounded for a from-orbit view).
    pub fn new(url_template: &str, maximum_level: u32) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .user_agent("cesium-rs-viewer-demo")
            .build()
            .unwrap_or_default();
        Self {
            url_template: url_template.to_string(),
            rectangle: Rectangle::new(
                -std::f64::consts::PI,
                -std::f64::consts::FRAC_PI_2,
                std::f64::consts::PI,
                std::f64::consts::FRAC_PI_2,
            ),
            tile_size: 256,
            maximum_level,
            cache: Mutex::new(HashMap::new()),
            client,
            fetched_ok: AtomicU32::new(0),
            fetched_fail: AtomicU32::new(0),
        }
    }

    /// Downloads + decodes one Mercator tile, caching the outcome (including a
    /// negative result). Returns `None` on any failure.
    fn fetch_tile(&self, z: u32, x: u32, y: u32) -> Option<Arc<DecodedTile>> {
        if let Some(hit) = self.cache.lock().ok()?.get(&(z, x, y)) {
            return hit.clone();
        }
        let url = self
            .url_template
            .replace("{z}", &z.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string());
        let decoded = (|| -> Option<DecodedTile> {
            let bytes = self.client.get(&url).send().ok()?.bytes().ok()?;
            let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
            let size = img.width();
            Some(DecodedTile { pixels: img.into_raw(), size })
        })();
        let result = decoded.map(Arc::new);
        match &result {
            Some(_) => {
                self.fetched_ok.fetch_add(1, Ordering::Relaxed);
            }
            None => {
                let n = self.fetched_fail.fetch_add(1, Ordering::Relaxed) + 1;
                if n <= 3 {
                    log::warn!("Web Mercator tile fetch failed: {url}");
                }
            }
        }
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert((z, x, y), result.clone());
        }
        result
    }

    /// Nearest-samples one pixel from a decoded tile.
    fn sample(tile: &DecodedTile, fx: u32, fy: u32) -> [u8; 4] {
        let x = fx.min(tile.size - 1);
        let y = fy.min(tile.size - 1);
        let i = ((y * tile.size + x) * 4) as usize;
        [
            tile.pixels[i],
            tile.pixels[i + 1],
            tile.pixels[i + 2],
            tile.pixels[i + 3],
        ]
    }
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

        let mut out = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
        let mut any = false;
        let mut any_fetched = false;

        for py in 0..size {
            // py = 0 → north (lat_max): keeps the PNG's row-0 = north, the
            // orientation the geographic pipeline nearest-samples.
            let frac_y = (py as f64 + 0.5) / size as f64;
            let lat = (lat_max - frac_y * (lat_max - lat_min))
                .clamp(-MERCATOR_MAX_LAT, MERCATOR_MAX_LAT);
            let lat_rad = lat.to_radians();
            let merc_y =
                (1.0 - (lat_rad.tan() + 1.0 / lat_rad.cos()).ln() / std::f64::consts::PI) / 2.0;
            let ty_f = merc_y * n;
            let ty = ty_f.floor();
            if ty < 0.0 || ty >= n {
                continue;
            }
            for px in 0..size {
                let frac_x = (px as f64 + 0.5) / size as f64;
                let lon = lon_min + frac_x * (lon_max - lon_min);
                let merc_x = (lon + 180.0) / 360.0;
                let tx_f = merc_x * n;
                let tx = tx_f.floor();
                if tx < 0.0 || tx >= n {
                    continue;
                }
                let tile = match self.fetch_tile(zm, tx as u32, ty as u32) {
                    Some(t) => t,
                    None => continue,
                };
                any_fetched = true;
                let fx = ((tx_f - tx) * size as f64)
                    .floor()
                    .clamp(0.0, (size - 1) as f64) as u32;
                let fy = ((ty_f - ty) * size as f64)
                    .floor()
                    .clamp(0.0, (size - 1) as f64) as u32;
                let [r, g, b, a] = Self::sample(&tile, fx, fy);
                out.put_pixel(px, py, Rgba([r, g, b, a]));
                any = true;
            }
        }

        if !any {
            // Nothing resolved. If every needed fetch failed, ask for a retry
            // (transient); if it was only polar/no-data, defer to the base map.
            return if any_fetched {
                TileImageAvailability::NoData
            } else {
                TileImageAvailability::Transient
            };
        }

        let mut buf = Cursor::new(Vec::new());
        match out.write_to(&mut buf, ImageFormat::Png) {
            Ok(()) => TileImageAvailability::Data(buf.into_inner()),
            Err(_) => TileImageAvailability::Transient,
        }
    }
}
