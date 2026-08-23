//! Ported from `packages/engine/Source/Scene/TerrainProvider.js`.
//!
//! Base trait for all terrain providers.

use cesium_core::rectangle::Rectangle;

/// Base trait for all terrain providers.
///
/// A terrain provider loads terrain tiles for a specific terrain service.
pub trait TerrainProvider {
    /// Returns whether this provider is ready.
    fn is_ready(&self) -> bool;

    /// Returns whether this provider has water mask data.
    fn has_water_mask(&self) -> bool;

    /// Returns whether this provider has vertex normals.
    fn has_vertex_normals(&self) -> bool;

    /// Returns the availability of tiles.
    fn availability(&self) -> Option<&str>;

    /// Requests a terrain tile at the given coordinates.
    fn request_tile(&self, x: u32, y: u32, level: u32) -> Option<Vec<u8>>;

    /// Gets the maximum geometric error for a tile.
    fn get_level_maximum_geometric_error(&self, level: u32) -> f64;

    /// Returns the tile data width.
    fn tile_width(&self) -> u32;

    /// Returns the tile data height.
    fn tile_height(&self) -> u32;
}
