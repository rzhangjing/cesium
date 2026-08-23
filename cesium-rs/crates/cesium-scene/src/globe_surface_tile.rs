//! Ported from `packages/engine/Source/Scene/GlobeSurfaceTile.js`.
//!
//! Data associated with a single tile on the globe surface.

use crate::quadtree_tile::QuadtreeTile;

/// Data associated with a single tile on the globe surface.
///
/// Holds the terrain mesh, imagery textures, and render state for one quadtree tile.
pub struct GlobeSurfaceTile {
    /// The quadtree tile this surface tile corresponds to.
    pub quad_tile: Option<QuadtreeTile>,

    /// Whether the terrain geometry has been loaded.
    pub terrain_loaded: bool,

    /// Whether the imagery textures have been loaded.
    pub imagery_loaded: bool,

    /// The number of imagery layers that still need to be loaded.
    pub pending_imagery_count: i32,

    /// Whether this tile has been rendered at least once.
    pub has_been_rendered: bool,

    /// The water mask texture coordinates (if applicable).
    pub water_mask: Option<Vec<u8>>,
}

impl GlobeSurfaceTile {
    /// Creates a new GlobeSurfaceTile.
    pub fn new() -> Self {
        Self {
            quad_tile: None,
            terrain_loaded: false,
            imagery_loaded: false,
            pending_imagery_count: 0,
            has_been_rendered: false,
            water_mask: None,
        }
    }

    /// Returns whether this tile is ready to render (terrain + imagery loaded).
    pub fn is_ready(&self) -> bool {
        self.terrain_loaded && self.imagery_loaded && self.pending_imagery_count == 0
    }

    /// Resets per-frame state.
    pub fn reset(&mut self) {
        self.has_been_rendered = false;
    }
}

impl Default for GlobeSurfaceTile {
    fn default() -> Self { Self::new() }
}
