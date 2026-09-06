//! Ported from `packages/engine/Source/Scene/TileCoordinatesImageryProvider.js`.

/// Tile coordinates imagery provider for debugging.
///
/// Renders tile x/y/level coordinates on each tile.
pub struct TileCoordinatesImageryProvider {
    /// Whether the provider is ready.
    pub ready: bool,
    /// The tile width.
    pub tile_width: u32,
    /// The tile height.
    pub tile_height: u32,
}

impl TileCoordinatesImageryProvider {
    /// Creates a new TileCoordinatesImageryProvider.
    pub fn new() -> Self { Self { ready: true, tile_width: 256, tile_height: 256 } }
}

impl Default for TileCoordinatesImageryProvider {
    fn default() -> Self { Self::new() }
}
