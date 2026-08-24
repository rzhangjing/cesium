//! Ported from `packages/engine/Source/Scene/QuadtreePrimitive.js`.
//!
//! A quadtree used for adaptive level-of-detail rendering of the globe surface.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::terrain_provider;
use cesium_core::tiling_scheme::TilingScheme;

use crate::frame_state::FrameState;
use crate::quadtree_tile::QuadtreeTile;

/// Default heightmap tile image width.
///
/// Mirrors CesiumJS `GlobeSurfaceTileProvider`'s default heightmap width (65),
/// which feeds `TerrainProvider.getEstimatedLevelZeroGeometricErrorForAHeightmap`.
pub const DEFAULT_TILE_IMAGE_WIDTH: i32 = 65;

/// Per-frame traversal inputs and accumulators.
///
/// DEVIATION (B4-2): CesiumJS threads `primitive`/`frameState` globals through
/// module-level functions; the Rust port bundles the same inputs into one
/// context so the traversal is a pure, testable function.
struct TraversalContext<'a> {
    tiling_scheme: &'a dyn TilingScheme,
    camera_position: Cartesian3,
    drawing_buffer_height: f64,
    sse_denominator: f64,
    maximum_screen_space_error: f64,
    maximum_level: Option<i32>,
    tiles_to_render: &'a mut Vec<QuadtreeTile>,
    tiles_visited: usize,
    max_depth_visited: i32,
}

/// A quadtree used for adaptive level-of-detail rendering of the globe surface.
///
/// Mirrors CesiumJS `QuadtreePrimitive.js` — manages tile selection, load queues
/// (high/medium/low priority), and time-sliced processing.
///
/// DEVIATION (B4-2): this batch implements single-frame synchronous loading
/// semantics (every visited tile is treated as immediately renderable). The
/// three-tier load queues and the 5ms time slice are reserved structurally
/// (`tile_load_queue_*` fields, `queueTileLoad` call sites) for a later batch.
pub struct QuadtreePrimitive {
    // ---- Configuration ----
    maximum_screen_space_error: f64,
    tile_cache_size: i32,
    loading_descendant_limit: i32,
    preload_ancestors: bool,
    preload_siblings: bool,
    /// Deepest level traversal may refine to (`None` = unbounded, like CesiumJS
    /// where the provider's `canRefine` decides).
    maximum_level: Option<i32>,
    /// Default heightmap tile image width used to estimate the level-zero
    /// geometric error (CesiumJS GlobeSurfaceTileProvider default: 65).
    tile_image_width: i32,

    // ---- Tile storage ----
    tiling_scheme: Box<dyn TilingScheme>,
    /// The level-zero tiles (2 for Geographic, 1 for WebMercator defaults).
    root_tiles: Vec<QuadtreeTile>,
    /// Level-zero maximum geometric error, meters.
    level_zero_maximum_geometric_error: f64,
    tiles_to_render: Vec<QuadtreeTile>,
    /// RESERVED: high-priority load queue (blocker tiles in CesiumJS).
    tile_load_queue_high: Vec<QuadtreeTile>,
    /// RESERVED: medium-priority load queue (rendered tiles in CesiumJS).
    tile_load_queue_medium: Vec<QuadtreeTile>,
    /// RESERVED: low-priority load queue (preloaded descendants in CesiumJS).
    tile_load_queue_low: Vec<QuadtreeTile>,

    // ---- State ----
    tiles_loaded: bool,
    invalidated: bool,
    last_selection_time: f64,
    is_destroyed: bool,

    // ---- Debug counters (mirror CesiumJS `_debug`) ----
    /// Number of tiles visited this frame.
    pub debug_tiles_visited: usize,
    /// Deepest tile level visited this frame.
    pub debug_max_depth_visited: i32,
}

impl QuadtreePrimitive {
    /// Creates a new QuadtreePrimitive with the default GeographicTilingScheme
    /// (two level-zero tiles), mirroring CesiumJS defaults.
    pub fn new() -> Self {
        Self::with_tiling_scheme(Box::new(GeographicTilingScheme::new(None, None, None, None)), None)
    }

    /// Creates a QuadtreePrimitive over an arbitrary tiling scheme.
    ///
    /// The level-zero geometric error is estimated exactly like CesiumJS:
    /// `TerrainProvider.getEstimatedLevelZeroGeometricErrorForAHeightmap(
    /// ellipsoid, tileImageWidth, numberOfTilesAtLevelZero)`.
    pub fn with_tiling_scheme(
        tiling_scheme: Box<dyn TilingScheme>,
        tile_image_width: Option<i32>,
    ) -> Self {
        let tile_image_width = tile_image_width.unwrap_or(DEFAULT_TILE_IMAGE_WIDTH);
        let level_zero_maximum_geometric_error =
            terrain_provider::get_estimated_level_zero_geometric_error_for_a_heightmap(
                tiling_scheme.ellipsoid(),
                tile_image_width as f64,
                tiling_scheme.get_number_of_x_tiles_at_level(0),
            );
        let root_tiles = QuadtreeTile::create_level_zero_tiles(
            tiling_scheme.as_ref(),
            level_zero_maximum_geometric_error,
        );
        Self {
            maximum_screen_space_error: 2.0,
            tile_cache_size: 100,
            loading_descendant_limit: 20,
            preload_ancestors: true,
            preload_siblings: false,
            maximum_level: None,
            tile_image_width,
            tiling_scheme,
            root_tiles,
            level_zero_maximum_geometric_error,
            tiles_to_render: Vec::new(),
            tile_load_queue_high: Vec::new(),
            tile_load_queue_medium: Vec::new(),
            tile_load_queue_low: Vec::new(),
            tiles_loaded: false,
            invalidated: true,
            last_selection_time: 0.0,
            is_destroyed: false,
            debug_tiles_visited: 0,
            debug_max_depth_visited: 0,
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

    /// Returns the deepest level traversal may refine to.
    pub fn maximum_level(&self) -> Option<i32> { self.maximum_level }
    /// Sets the deepest level traversal may refine to.
    pub fn set_maximum_level(&mut self, value: Option<i32>) { self.maximum_level = value; }

    /// Returns the default heightmap tile image width.
    pub fn tile_image_width(&self) -> i32 { self.tile_image_width }

    /// Returns the tiling scheme.
    pub fn tiling_scheme(&self) -> &dyn TilingScheme { self.tiling_scheme.as_ref() }

    /// Returns the root (level-zero) tiles.
    pub fn root_tiles(&self) -> &[QuadtreeTile] { &self.root_tiles }

    /// Returns the tiles selected for rendering this frame.
    pub fn tiles_to_render(&self) -> &[QuadtreeTile] { &self.tiles_to_render }

    /// Returns the level-zero maximum geometric error (meters).
    pub fn level_zero_maximum_geometric_error(&self) -> f64 {
        self.level_zero_maximum_geometric_error
    }

    /// Returns the maximum geometric error for a level, mirroring CesiumJS
    /// `GlobeSurfaceTileProvider#getLevelMaximumGeometricError`.
    pub fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        self.level_zero_maximum_geometric_error / ((1i32 << level) as f64)
    }

    /// Returns true when the tile load queue is empty.
    pub fn tiles_loaded(&self) -> bool {
        self.tile_load_queue_high.is_empty()
            && self.tile_load_queue_medium.is_empty()
            && self.tile_load_queue_low.is_empty()
    }

    // ---- Frame lifecycle ----

    /// Updates the quadtree for the current frame.
    ///
    /// Mirrors CesiumJS `QuadtreePrimitive#update` → `visitTile` recursion:
    /// compute each tile's camera distance and screen-space error, render the
    /// tile when `sse < maximumScreenSpaceError`, otherwise refine into its
    /// four children.
    ///
    /// DEVIATION (B4-2): single-frame synchronous semantics — visited tiles
    /// are immediately renderable, so the `ancestorMeetsSse` fall-through
    /// (keep rendering descendants while a blocker tile loads) and the load
    /// queue processing never trigger. Fog-based SSE attenuation and
    /// pixel-ratio scaling are not applied (the FrameState carries neither).
    pub fn update(&mut self, frame_state: &FrameState) {
        self.begin_frame(frame_state);

        let mut tiles_to_render = std::mem::take(&mut self.tiles_to_render);
        {
            let mut context = TraversalContext {
                tiling_scheme: self.tiling_scheme.as_ref(),
                camera_position: frame_state.camera_position,
                drawing_buffer_height: frame_state.drawing_buffer_height as f64,
                sse_denominator: frame_state.sse_denominator,
                maximum_screen_space_error: self.maximum_screen_space_error,
                maximum_level: self.maximum_level,
                tiles_to_render: &mut tiles_to_render,
                tiles_visited: 0,
                max_depth_visited: 0,
            };
            for root in self.root_tiles.iter_mut() {
                visit_tile(&mut context, root, false);
            }
            self.debug_tiles_visited = context.tiles_visited;
            self.debug_max_depth_visited = context.max_depth_visited;
        }
        self.tiles_to_render = tiles_to_render;

        // Synchronous loading: nothing is pending after traversal.
        let all_queues_empty = self.tile_load_queue_high.is_empty()
            && self.tile_load_queue_medium.is_empty()
            && self.tile_load_queue_low.is_empty();
        self.tiles_loaded = all_queues_empty;
        self.last_selection_time = frame_state.frame_number as f64;
        self.end_frame(frame_state);
    }

    /// Called at the beginning of each frame.
    pub fn begin_frame(&mut self, _frame_state: &FrameState) {
        // In full port: clear per-frame state, invalidate if needed
        self.tiles_to_render.clear();
        self.tile_load_queue_high.clear();
        self.tile_load_queue_medium.clear();
        self.tile_load_queue_low.clear();
        self.debug_tiles_visited = 0;
        self.debug_max_depth_visited = 0;
        self.invalidated = false;
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

/// Computes the distance from the camera to the tile's bounding sphere.
///
/// SSE distance-floor discipline (cesiumrust historical lesson): the distance
/// is clamped to zero only. Never introduce a floor above the camera's actual
/// minimum distance — that undersplits by one level and leaves the whole
/// screen blurry.
fn compute_tile_distance(tile: &QuadtreeTile, camera_position: &Cartesian3) -> f64 {
    let to_center = Cartesian3::subtract_new(camera_position, &tile.bounding_sphere.center);
    let distance = Cartesian3::magnitude(&to_center) - tile.bounding_sphere.radius;
    if distance > 0.0 { distance } else { 0.0 }
}

/// Computes the tile's screen-space error in pixels.
///
/// Mirrors CesiumJS `QuadtreePrimitive` `screenSpaceError` (3D perspective
/// branch, fog/pixelRatio omitted):
/// `error = (maxGeometricError * drawingBufferHeight) / (distance * sseDenominator)`.
fn compute_screen_space_error(context: &TraversalContext<'_>, tile: &QuadtreeTile) -> f64 {
    let distance = tile.camera_distance;
    if distance <= 0.0 {
        // Camera inside/on the bounding volume: infinite error forces refinement
        // down to the maximum allowed level, exactly like CesiumJS.
        return f64::MAX;
    }
    (tile.geometric_error * context.drawing_buffer_height)
        / (distance * context.sse_denominator)
}

/// Mirrors CesiumJS `visitTile` (simplified single-frame form).
fn visit_tile(
    context: &mut TraversalContext<'_>,
    tile: &mut QuadtreeTile,
    ancestor_meets_sse: bool,
) {
    context.tiles_visited += 1;
    if tile.level > context.max_depth_visited {
        context.max_depth_visited = tile.level;
    }

    tile.camera_distance = compute_tile_distance(tile, &context.camera_position);
    tile.screen_space_error = compute_screen_space_error(context, tile);

    // Strict `<`, matching CesiumJS: `screenSpaceError(...) < maximumScreenSpaceError`.
    let meets_sse = tile.screen_space_error < context.maximum_screen_space_error;

    if meets_sse || ancestor_meets_sse {
        // JS: queueTileLoad(tileLoadQueueMedium, tile) — RESERVED for the
        // async load-queue batch.
        tile.was_rendered = true;
        tile.renderable = true;
        context.tiles_to_render.push(tile.snapshot());
        return;
    }

    // SSE is not good enough: refine, unless the maximum level is reached
    // (JS: `tileProvider.canRefine(tile)` returns false → render the tile).
    let can_refine = match context.maximum_level {
        Some(maximum) => tile.level < maximum,
        None => true,
    };
    if !can_refine {
        tile.was_rendered = true;
        tile.renderable = true;
        context.tiles_to_render.push(tile.snapshot());
        return;
    }

    tile.ensure_children(context.tiling_scheme);
    for child in tile.children.iter_mut() {
        visit_tile(context, child, ancestor_meets_sse);
    }
}

impl Default for QuadtreePrimitive {
    fn default() -> Self { Self::new() }
}
