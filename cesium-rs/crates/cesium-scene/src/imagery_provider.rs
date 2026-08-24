//! Ported from `packages/engine/Source/Scene/ImageryProvider.js`.
//!
//! Base trait for all imagery providers.

use cesium_core::rectangle::Rectangle;

/// Availability outcome of a single imagery tile request.
///
/// Mirrors the CesiumJS failed/placeholder discipline (cesiumrust pitfall
/// checkpoint): a deterministic absence of data (e.g. the tile file does not
/// exist) must be distinguished from a transient failure (e.g. an IO error or
/// network hiccup). Deterministic [`TileImageAvailability::NoData`] lets the
/// globe fall back to the ancestor tile texture permanently for this tile;
/// [`TileImageAvailability::Transient`] must be retried on a later frame and
/// must NEVER be stamped as a permanent no-data hole.
pub enum TileImageAvailability {
    /// The tile image is available (encoded image bytes, e.g. PNG/JPEG).
    Data(Vec<u8>),
    /// The tile deterministically has no data (e.g. the file does not exist).
    /// Callers may inherit the ancestor tile texture.
    NoData,
    /// The tile is temporarily unavailable (transient IO/network failure).
    /// Callers must retry on a later frame — never treat as permanent.
    Transient,
}

/// Base trait for all imagery providers.
///
/// An imagery provider loads image tiles for a specific imagery service.
pub trait ImageryProvider {
    /// Returns the URL of the imagery service.
    fn url(&self) -> &str;

    /// Returns the proxy used by this provider.
    fn proxy(&self) -> Option<&str>;

    /// Returns the rectangle of the imagery.
    fn rectangle(&self) -> &Rectangle;

    /// Returns the width of each tile in pixels.
    fn tile_width(&self) -> u32;

    /// Returns the height of each tile in pixels.
    fn tile_height(&self) -> u32;

    /// Returns the maximum tile level.
    fn maximum_level(&self) -> Option<u32>;

    /// Returns the minimum tile level.
    fn minimum_level(&self) -> Option<u32>;

    /// Returns whether this provider has a watermark.
    fn has_water_mask(&self) -> bool;

    /// Returns whether this provider is ready.
    fn is_ready(&self) -> bool;

    /// Requests a tile image at the given coordinates.
    fn request_image(&self, x: u32, y: u32, level: u32) -> Option<Vec<u8>>;

    /// Requests a tile image with an explicit availability classification
    /// (deterministic no-data vs. transient failure).
    ///
    /// The default implementation adapts the legacy [`ImageryProvider::request_image`]
    /// (`Some` → `Data`, `None` → `NoData`); providers that can distinguish a
    /// missing file from a transient read error override this method.
    fn request_tile_image_availability(
        &self,
        x: u32,
        y: u32,
        level: u32,
    ) -> TileImageAvailability {
        match self.request_image(x, y, level) {
            Some(data) => TileImageAvailability::Data(data),
            None => TileImageAvailability::NoData,
        }
    }
}
