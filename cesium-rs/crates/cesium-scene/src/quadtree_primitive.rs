//! Ported from `packages/engine/Source/Scene/QuadtreePrimitive.js`.
//!
//! A quadtree used for adaptive level-of-detail rendering of the globe surface.

use crate::frame_state::FrameState;
use crate::globe_surface_tile_provider::GlobeSurfaceTileProvider;
use crate::quadtree_tile::QuadtreeTile;

/// A quadtree used for adaptive level-of-detail rendering of the globe surface.
///
/// Mirrors CesiumJS `QuadtreePrimitive.js` — manages tile selection, load queues
/// (high/medium/low priority), and time-sliced processing.
pub struct QuadtreePrimitive {
    // ---- Configuration ----
    maximum_screen_space_error: f64,
    tile_cache_size: i32,
    loading_descendant_limit: i32,
    preload_ancestors: bool,
    preload_siblings: bool,

    // ---- Tile storage ----
    tiles_to_render: Vec<QuadtreeTile>,
    tile_load_queue_high: Vec<QuadtreeTile>,
    tile_load_queue_medium: Vec<QuadtreeTile>,
    tile_load_queue_low: Vec<QuadtreeTile>,

    // ---- State ----
    tiles_loaded: bool,
    invalidated: bool,
    last_selection_time: f64,
    is_destroyed: bool,
}

impl QuadtreePrimitive {
    /// Creates a new QuadtreePrimitive.
    pub fn new() -> Self {
        Self {
            maximum_screen_space_error: 2.0,
            tile_cache_size: 100,
            loading_descendant_limit: 20,
            preload_ancestors: true,
            preload_siblings: false,
            tiles_to_render: Vec::new(),
            tile_load_queue_high: Vec::new(),
            tile_load_queue_medium: Vec::new(),
            tile_load_queue_low: Vec::new(),
            tiles_loaded: false,
            invalidated: true,
            last_selection_time: 0.0,
            is_destroyed: false,
        }
    }

    // ---- Configuration setters/getters ----

    pub fn maximum_screen_space_error(&self) -> f64 { self.maximum_screen_space_error }
    pub fn set_maximum_screen_space_error(&mut self, value: f64) { self.maximum_screen_space_error = value; }

    pub fn tile_cache_size(&self) -> i32 { self.tile_cache_size }
    pub fn set_tile_cache_size(&mut self, value: i32) { self.tile_cache_size = value; }

    pub fn loading_descendant_limit(&self) -> i32 { self.loading_descendant_limit }
    pub fn set_loading_descendant_limit(&mut self, value: i32) { self.loading_descendant_limit = value; }

    pub fn preload_ancestors(&self) -> bool { self.preload_ancestors }
    pub fn set_preload_ancestors(&mut self, value: bool) { self.preload_ancestors = value; }

    pub fn preload_siblings(&self) -> bool { self.preload_siblings }
    pub fn set_preload_siblings(&mut self, value: bool) { self.preload_siblings = value; }

    /// Returns true when the tile load queue is empty.
    pub fn tiles_loaded(&self) -> bool {
        self.tile_load_queue_high.is_empty()
            && self.tile_load_queue_medium.is_empty()
            && self.tile_load_queue_low.is_empty()
    }

    // ---- Frame lifecycle ----

    /// Updates the quadtree for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // In full port:
        // 1. Walk quadtree from root, computing screen-space error per tile
        // 2. Refine tiles that exceed maximum_screen_space_error
        // 3. Populate load queues (high/medium/low priority)
        // 4. Process load queues within time slice (5ms budget)
        // 5. Collect tiles_to_render for the render pass
    }

    /// Called at the beginning of each frame.
    pub fn begin_frame(&mut self, _frame_state: &FrameState) {
        // In full port: clear per-frame state, invalidate if needed
        self.tiles_to_render.clear();
        self.tile_load_queue_high.clear();
        self.tile_load_queue_medium.clear();
        self.tile_load_queue_low.clear();
    }

    /// Renders all selected tiles.
    pub fn render(&self, _frame_state: &FrameState) {
        // In full port: issue draw commands for each tile in tiles_to_render
    }

    /// Called at the end of each frame.
    pub fn end_frame(&mut self, _frame_state: &FrameState) {
        // In full port: free tiles not needed, update tile cache
    }

    /// Invalidates the quadtree, forcing re-evaluation next frame.
    pub fn invalidate(&mut self) {
        self.invalidated = true;
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) {
        self.tiles_to_render.clear();
        self.tile_load_queue_high.clear();
        self.tile_load_queue_medium.clear();
        self.tile_load_queue_low.clear();
        self.is_destroyed = true;
    }
}

impl Default for QuadtreePrimitive {
    fn default() -> Self { Self::new() }
}
