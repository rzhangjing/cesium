//! Ported from `packages/engine/Source/Scene/TileImagery.js`.
//!
//! The imagery texture coordinates and references for a single terrain tile.

/// The imagery texture coordinates and references for a single terrain tile.
///
/// Each TileImagery holds the texture coordinates for mapping one imagery
/// texture onto one terrain tile, along with a reference to the imagery data.
pub struct TileImagery {
    /// The texture coordinates for the imagery on this tile (u0, v0, u1, v1).
    pub texture_coordinates: [f64; 4],
    /// Whether this tile imagery is using a texture from an ancestor tile.
    pub using_ancestor_texture: bool,
    /// The index of the imagery layer.
    pub layer_index: usize,
    /// Whether this tile imagery is ready (texture uploaded to GPU).
    pub ready: bool,
}

impl TileImagery {
    /// Creates a new TileImagery.
    pub fn new(layer_index: usize) -> Self {
        Self {
            texture_coordinates: [0.0, 0.0, 1.0, 1.0],
            using_ancestor_texture: false,
            layer_index,
            ready: false,
        }
    }
}

impl Default for TileImagery {
    fn default() -> Self { Self::new(0) }
}
