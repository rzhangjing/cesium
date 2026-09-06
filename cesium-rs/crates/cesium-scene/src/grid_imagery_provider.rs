//! Ported from `packages/engine/Source/Scene/GridImageryProvider.js`.

/// Grid imagery provider for debugging.
///
/// Renders a coordinate grid overlay for tile boundary visualization.
pub struct GridImageryProvider {
    /// The number of cells per tile.
    pub cells: u32,
    /// Whether the provider is ready.
    pub ready: bool,
    /// The tile width.
    pub tile_width: u32,
    /// The tile height.
    pub tile_height: u32,
}

impl GridImageryProvider {
    /// Creates a new GridImageryProvider.
    pub fn new() -> Self {
        Self { cells: 8, ready: true, tile_width: 256, tile_height: 256 }
    }
}

impl Default for GridImageryProvider {
    fn default() -> Self { Self::new() }
}
