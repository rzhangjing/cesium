//! Ported intent of `packages/engine/Source/Scene/UrlTemplateImageryProvider.js`
//! for the offline (no-network) path.
//!
//! A local-file XYZ imagery provider: reads tile images from a directory laid
//! out as `{root}/{level}/{x}/{y}.{ext}`. There is no network access; the
//! provider is fully deterministic, which makes it suitable for headless
//! acceptance tests and the viewer-demo offline globe.
//!
//! failed/placeholder discipline (cesiumrust pitfall checkpoint):
//! - file not found → [`TileImageAvailability::NoData`] (deterministic; the
//!   globe may inherit the ancestor tile texture permanently)
//! - any other IO/decode error → [`TileImageAvailability::Transient`]
//!   (retried on a later frame; NEVER stamped as permanent no-data)

use std::path::{Path, PathBuf};

use cesium_core::rectangle::Rectangle;

use crate::imagery_provider::{ImageryProvider, TileImageAvailability};

/// Candidate tile file extensions, in probe order.
const TILE_EXTENSIONS: [&str; 3] = ["png", "jpg", "jpeg"];

/// A local-file XYZ imagery provider (offline, no network).
///
/// Mirrors the CesiumJS `UrlTemplateImageryProvider` contract (url template,
/// tile size, level range, rectangle) while sourcing tiles from a local
/// directory instead of HTTP.
pub struct FileImageryProvider {
    /// Root directory of the tile pyramid (`{root}/{z}/{x}/{y}.{ext}`).
    root: PathBuf,
    /// The "url" of the provider (the root directory as a `file://` string),
    /// mirroring `ImageryProvider.url`.
    url: String,
    /// Width of each tile in pixels.
    tile_width: u32,
    /// Height of each tile in pixels.
    tile_height: u32,
    /// Minimum tile level available.
    minimum_level: u32,
    /// Maximum tile level available (derived from the directory contents
    /// unless overridden).
    maximum_level: u32,
    /// The rectangle covered by the imagery (full globe by default).
    rectangle: Rectangle,
}

impl FileImageryProvider {
    /// Creates a new provider rooted at `root`.
    ///
    /// `maximum_level_override` clamps the level range when supplied; the
    /// default maximum level is probed from the directory (highest
    /// `{root}/{z}` subdirectory containing tile files), falling back to 0
    /// when the directory is absent (the provider stays "ready" and every
    /// request deterministically reports [`TileImageAvailability::NoData`]).
    pub fn new(root: impl AsRef<Path>, maximum_level_override: Option<u32>) -> Self {
        let root = root.as_ref().to_path_buf();
        let probed = probe_maximum_level(&root);
        let maximum_level = maximum_level_override.unwrap_or(probed);
        let url = format!("file://{}", root.display());
        Self {
            root,
            url,
            tile_width: 256,
            tile_height: 256,
            minimum_level: 0,
            maximum_level,
            rectangle: Rectangle::new(
                -std::f64::consts::PI,
                -std::f64::consts::FRAC_PI_2,
                std::f64::consts::PI,
                std::f64::consts::FRAC_PI_2,
            ),
        }
    }

    /// Resolves the tile file path, probing candidate extensions in order.
    /// Returns `None` when no candidate exists (deterministic no-data).
    fn tile_path(&self, x: u32, y: u32, level: u32) -> Option<PathBuf> {
        for extension in TILE_EXTENSIONS {
            let path = self.root.join(format!("{level}/{x}/{y}.{extension}"));
            if path.is_file() {
                return Some(path);
            }
        }
        None
    }
}

/// Probes the directory for the deepest level folder that contains tile
/// files. Returns 0 when nothing is found.
fn probe_maximum_level(root: &Path) -> u32 {
    let mut maximum = 0u32;
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Ok(level) = name.parse::<u32>() {
            if level > maximum && entry.path().is_dir() {
                // Require at least one tile file under this level to count.
                let has_tile = std::fs::read_dir(entry.path())
                    .map(|sub| {
                        sub.flatten()
                            .any(|tile_dir| tile_dir.path().is_dir())
                    })
                    .unwrap_or(false);
                if has_tile {
                    maximum = level;
                }
            }
        }
    }
    maximum
}

impl ImageryProvider for FileImageryProvider {
    fn url(&self) -> &str {
        &self.url
    }

    fn proxy(&self) -> Option<&str> {
        None
    }

    fn rectangle(&self) -> &Rectangle {
        &self.rectangle
    }

    fn tile_width(&self) -> u32 {
        self.tile_width
    }

    fn tile_height(&self) -> u32 {
        self.tile_height
    }

    fn maximum_level(&self) -> Option<u32> {
        Some(self.maximum_level)
    }

    fn minimum_level(&self) -> Option<u32> {
        Some(self.minimum_level)
    }

    fn has_water_mask(&self) -> bool {
        false
    }

    fn is_ready(&self) -> bool {
        // A local directory provider is always ready; missing tiles are
        // reported per-request as deterministic no-data.
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
        x: u32,
        y: u32,
        level: u32,
    ) -> TileImageAvailability {
        if level < self.minimum_level || level > self.maximum_level {
            // Outside the advertised range: deterministic no-data (the
            // traversal should not request these, but be explicit).
            return TileImageAvailability::NoData;
        }
        let Some(path) = self.tile_path(x, y, level) else {
            // File absent: deterministic no-data → ancestor inheritance.
            return TileImageAvailability::NoData;
        };
        match std::fs::read(&path) {
            Ok(bytes) => TileImageAvailability::Data(bytes),
            Err(_) => {
                // File exists but could not be read (transient IO issue):
                // retry later, never stamp as permanent no-data.
                TileImageAvailability::Transient
            }
        }
    }
}
