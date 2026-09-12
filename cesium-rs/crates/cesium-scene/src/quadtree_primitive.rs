//! Ported from `packages/engine/Source/Scene/QuadtreePrimitive.js`.
//!
//! A quadtree used for adaptive level-of-detail rendering of the globe surface.
//!
//! | CesiumJS                             | Rust port                                              |
//! | ------------------------------------ | ------------------------------------------------------ |
//! | `beginFrame`                         | [`QuadtreePrimitive::begin_frame`]                     |
//! | `update`                             | [`QuadtreePrimitive::update`]                          |
//! | `render` → `selectTilesForRendering` | [`QuadtreePrimitive::render`]                          |
//! | `endFrame` → `processTileLoadQueue`  | [`QuadtreePrimitive::end_frame`]                       |
//! | `visitTile`                          | [`visit_tile`]                                         |
//! | `visitVisibleChildrenNearToFar`      | [`visit_visible_children_near_to_far`]                 |
//! | `visitIfVisible`                     | [`visit_if_visible`]                                   |
//! | `screenSpaceError`                   | [`compute_screen_space_error`]                         |
//! | `containsNeededPosition`             | [`contains_needed_position`]                           |
//! | `compareDistanceToPoint`             | [`select_tiles_for_rendering`] (inline `sort_by`)      |
//! | `queueTileLoad`                      | [`queue_tile_load`]                                    |
//! | `processTileLoadQueue`               | [`QuadtreePrimitive::process_tile_load_queue`]         |
//! | `processSinglePriorityLoadQueue`     | [`QuadtreePrimitive::process_single_priority_load_queue`] |
//! | `updateTileLoadProgress`             | [`QuadtreePrimitive::update_tile_load_progress`]       |
//! | `invalidateAllTiles`                 | [`QuadtreePrimitive::invalidate_all_tiles`]            |
//! | `TraversalDetails`                   | [`TraversalDetails`]                                   |
//! | `TraversalQuadDetails`               | [`TraversalQuadDetails`]                               |
//!
//! # Not ported (tracked in `docs/deviations.md`)
//!
//! * `TileReplacementQueue` (`markTileRendered` / `trimTiles` /
//!   `markStartOfRenderFrame`) — LRU tile-cache eviction. The port never frees
//!   a tile, so every call site is a no-op.
//! * `updateHeights` and the `_tileToUpdateHeights` / `_addHeightCallbacks` /
//!   `_removeHeightCallbacks` machinery. The list is still maintained (and
//!   still truncated by the kick path) because the truncation indices are part
//!   of `visitTile`'s structure, but nothing consumes it.
//! * `tile.updateCustomData()`, `tileProvider.beginUpdate` / `endUpdate` /
//!   `updateForPick` / `initialize` / `cancelReprojections` — texture
//!   reprojection plumbing.
//! * `createRenderCommandsForSelectedTiles` — [`crate::globe::Globe::render`]
//!   issues the tile draws itself.
//! * Frustum and horizon culling: `FrameState` carries no `cullingVolume` and
//!   the primitive holds no `EllipsoidOccluder`, so [`compute_tile_visibility`]
//!   always returns `Visibility::Partial`. `screenSpaceError2D` is likewise
//!   absent (no orthographic frustum extents on `FrameState`).
//! * Fog: `FrameState` carries no `fog` block, so neither the fog cull in
//!   `computeTileVisibility` nor the `CesiumMath.fog` SSE attenuation in
//!   `screenSpaceError` is applied.
//! * `tileProvider.canRenderWithoutLosingDetail` (renderable condition 4) and
//!   `computeTileLoadPriority`. CesiumJS guards both with `defined(...)`, so an
//!   absent method leaves condition 4 `false` and skips the priority sort —
//!   which is what the port reproduces.
//! * `_tileLoadProgressEvent` / `_debug.enableDebugOutput` console logging.
//!   `debug_max_depth` and `debug_tiles_rendered` are computed unconditionally
//!   (CesiumJS gates them behind `enableDebugOutput`); both are pure reductions
//!   over the render list with no other effect.

use std::time::{Duration, Instant};

use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::geographic_tiling_scheme::GeographicTilingScheme;
use cesium_core::rectangle::Rectangle;
use cesium_core::terrain_provider;
use cesium_core::tiling_scheme::TilingScheme;
use cesium_core::visibility::Visibility;

use crate::frame_state::FrameState;
use crate::quadtree_tile::{
    QuadtreeTile, TileKey, CHILD_NORTHEAST, CHILD_NORTHWEST, CHILD_SOUTHEAST, CHILD_SOUTHWEST,
};
use crate::quadtree_tile_load_state::QuadtreeTileLoadState;
use crate::scene_mode::SceneMode;
use crate::tile_selection_result::TileSelectionResult;

/// Default heightmap tile image width.
///
/// Mirrors CesiumJS `GlobeSurfaceTileProvider`'s default heightmap width (65),
/// which feeds `TerrainProvider.getEstimatedLevelZeroGeometricErrorForAHeightmap`.
pub const DEFAULT_TILE_IMAGE_WIDTH: i32 = 65;

/// Which of the three load queues a tile is queued into.
///
/// Mirrors the CesiumJS constructor comments: *high priority tiles are
/// preventing refinement; medium priority tiles are being rendered; low
/// priority tiles were refined past or are non-visible parts of quads.*
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadQueue {
    High,
    Medium,
    Low,
}

/// Tracks details of traversing a tile while selecting tiles for rendering.
///
/// Mirrors CesiumJS `TraversalDetails`.
#[derive(Debug, Clone, Copy)]
struct TraversalDetails {
    /// True if all selected (i.e. not culled or refined) tiles in this tile's
    /// subtree are renderable. If the subtree is renderable, we'll render it;
    /// no drama.
    all_are_renderable: bool,

    /// True if any tiles in this tile's subtree were rendered last frame. If
    /// any were, we must render the subtree rather than this tile, because
    /// rendering this tile would cause detail to vanish that was visible last
    /// frame, and that's no good.
    any_were_rendered_last_frame: bool,

    /// Counts the number of selected tiles in this tile's subtree that are not
    /// yet ready to be rendered because they need more loading. Note that this
    /// value will *not* necessarily be zero when `all_are_renderable` is true,
    /// for subtle reasons: when `all_are_renderable` and
    /// `any_were_rendered_last_frame` are both false, the kick path renders
    /// this tile instead of any tiles in its subtree and `all_are_renderable`
    /// will then reflect only whether *this* tile is renderable, while
    /// `not_yet_renderable_count` still reflects the total number of tiles we
    /// are waiting on — including the ones we are no longer rendering. It is
    /// only reset when a subtree is removed from the render queue because
    /// `not_yet_renderable_count` exceeds `loading_descendant_limit`.
    not_yet_renderable_count: i32,
}

impl TraversalDetails {
    const fn new() -> Self {
        Self {
            all_are_renderable: true,
            any_were_rendered_last_frame: false,
            not_yet_renderable_count: 0,
        }
    }
}

/// Per-quadrant traversal details, mirroring CesiumJS `TraversalQuadDetails`.
///
/// DEVIATION: CesiumJS keeps a module-level `traversalQuadsByLevel` array of 31
/// reusable objects (level 30 tiles are ~2cm wide at the equator) and indexes it
/// by `southwest.level`. The port allocates one on the stack per
/// [`visit_visible_children_near_to_far`] call. Equivalent because every path
/// through `visitTile` and `visitIfVisible` assigns all three fields of the
/// details object before `combine` reads them, so no stale value can survive.
#[derive(Debug, Clone, Copy)]
struct TraversalQuadDetails {
    southwest: TraversalDetails,
    southeast: TraversalDetails,
    northwest: TraversalDetails,
    northeast: TraversalDetails,
}

impl TraversalQuadDetails {
    fn new() -> Self {
        Self {
            southwest: TraversalDetails::new(),
            southeast: TraversalDetails::new(),
            northwest: TraversalDetails::new(),
            northeast: TraversalDetails::new(),
        }
    }

    /// Mirrors `TraversalQuadDetails.prototype.combine`.
    fn combine(&self, result: &mut TraversalDetails) {
        result.all_are_renderable = self.southwest.all_are_renderable
            && self.southeast.all_are_renderable
            && self.northwest.all_are_renderable
            && self.northeast.all_are_renderable;
        result.any_were_rendered_last_frame = self.southwest.any_were_rendered_last_frame
            || self.southeast.any_were_rendered_last_frame
            || self.northwest.any_were_rendered_last_frame
            || self.northeast.any_were_rendered_last_frame;
        result.not_yet_renderable_count = self.southwest.not_yet_renderable_count
            + self.southeast.not_yet_renderable_count
            + self.northwest.not_yet_renderable_count
            + self.northeast.not_yet_renderable_count;
    }
}

/// The subset of [`QuadtreePrimitive`] and [`FrameState`] the traversal needs.
///
/// DEVIATION: CesiumJS threads `primitive`/`frameState`/`tileProvider` through
/// module-level functions. The port cannot: `root_tiles` and `tiles_to_render`
/// are both fields of the same struct, so the traversal would need two `&mut`
/// borrows of `self`. Splitting the accumulators out with `std::mem::take` and
/// copying the configuration in keeps `visit_tile` a free recursive function.
struct Traversal<'a> {
    // ---- Configuration (copied out of the primitive) ----
    tiling_scheme: &'a dyn TilingScheme,
    maximum_screen_space_error: f64,
    loading_descendant_limit: i32,
    preload_ancestors: bool,
    preload_siblings: bool,
    maximum_level: Option<i32>,
    level_zero_maximum_geometric_error: f64,
    synchronous_tile_provider: bool,

    // ---- Frame inputs (copied out of the frame state) ----
    camera_position: Cartesian3,
    /// `frameState.camera.positionCartographic`. `None` when the camera is not
    /// above the ellipsoid, which is how JS `undefined` behaves here: every
    /// `undefined < x` comparison is `false`, so the near-to-far ordering falls
    /// through to the "camera in northeast quadrant" branch.
    camera_position_cartographic: Option<Cartographic>,
    /// `primitive._cameraReferenceFrameOriginCartographic`.
    ///
    /// DEVIATION: CesiumJS derives this from
    /// `Matrix4.getTranslation(camera.transform)`; `FrameState` carries no
    /// camera reference-frame transform, so it is always `None`. For the default
    /// identity camera transform CesiumJS gets the ellipsoid centre, whose
    /// `cartesianToCartographic` is likewise `undefined`, so this matches the
    /// common case exactly.
    camera_reference_frame_origin_cartographic: Option<Cartographic>,
    drawing_buffer_height: f64,
    sse_denominator: f64,
    pixel_ratio: f64,
    frame_number: u64,
    last_selection_frame_number: Option<u64>,

    // ---- Accumulators (taken out of the primitive) ----
    tiles_to_render: &'a mut Vec<QuadtreeTile>,
    tiles_rendered_this_frame: &'a mut Vec<TileKey>,
    tiles_to_update_heights: &'a mut Vec<TileKey>,
    tile_load_queue_high: &'a mut Vec<TileKey>,
    tile_load_queue_medium: &'a mut Vec<TileKey>,
    tile_load_queue_low: &'a mut Vec<TileKey>,

    // ---- Debug counters (`primitive._debug`) ----
    tiles_visited: usize,
    tiles_culled: usize,
    max_depth_visited: i32,
    tiles_waiting_for_children: usize,
}

/// A quadtree used for adaptive level-of-detail rendering of the globe surface.
///
/// Mirrors CesiumJS `QuadtreePrimitive.js`: tile selection with anti-flicker
/// renderability bookkeeping, three-tier load queues, and a time-sliced
/// per-frame load budget.
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
    /// `_loadQueueTimeSlice` — per-frame budget, in milliseconds, for
    /// `processTileLoadQueue`. CesiumJS: 5.0.
    load_queue_time_slice: f64,
    /// See [`QuadtreePrimitive::set_synchronous_tile_provider`].
    synchronous_tile_provider: bool,

    // ---- Tile storage ----
    tiling_scheme: Box<dyn TilingScheme>,
    /// The level-zero tiles (2 for Geographic, 1 for WebMercator defaults).
    /// Mirrors `_levelZeroTiles`.
    root_tiles: Vec<QuadtreeTile>,
    /// Level-zero maximum geometric error, meters.
    level_zero_maximum_geometric_error: f64,
    tiles_to_render: Vec<QuadtreeTile>,
    /// `_tilesRenderedThisFrame`. CesiumJS uses a `Set` so that a tile added by
    /// several render passes in one frame is visited once while keeping
    /// insertion order; the port reproduces both with a deduplicating `Vec`.
    /// Unlike `_tilesToRender` it is *not* truncated by the kick path, so a
    /// kicked tile stays pickable — which is what
    /// [`crate::globe::Globe::pick_world_coordinates`] relies on.
    tiles_rendered_this_frame: Vec<TileKey>,
    /// `_tileToUpdateHeights`. Maintained (and truncated by the kick path) for
    /// structural fidelity; `updateHeights` itself is not ported.
    tiles_to_update_heights: Vec<TileKey>,
    /// High-priority load queue: tiles that are preventing refinement.
    tile_load_queue_high: Vec<TileKey>,
    /// Medium-priority load queue: tiles that are being rendered.
    tile_load_queue_medium: Vec<TileKey>,
    /// Low-priority load queue: tiles that were refined past, or non-visible
    /// parts of quads.
    tile_load_queue_low: Vec<TileKey>,

    // ---- State ----
    /// `_tilesInvalidated`.
    tiles_invalidated: bool,
    /// `_lastSelectionFrameNumber`; `undefined` until the first selection pass.
    last_selection_frame_number: Option<u64>,
    /// `_lastTileLoadQueueLength`.
    last_tile_load_queue_length: usize,
    is_destroyed: bool,

    // ---- Debug counters (mirror CesiumJS `_debug`) ----
    /// `_debug.tilesVisited` — tiles visited this frame.
    pub debug_tiles_visited: usize,
    /// `_debug.maxDepthVisited` — deepest tile level visited this frame.
    pub debug_max_depth_visited: i32,
    /// `_debug.tilesCulled` — tiles rejected by `computeTileVisibility`.
    pub debug_tiles_culled: usize,
    /// `_debug.tilesWaitingForChildren` — tiles rendered while a blocker loads.
    pub debug_tiles_waiting_for_children: usize,
    /// `_debug.tilesRendered`, set by `updateTileLoadProgress`.
    pub debug_tiles_rendered: usize,
    /// `_debug.maxDepth`, set by `updateTileLoadProgress`.
    pub debug_max_depth: i32,
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
            load_queue_time_slice: 5.0,
            synchronous_tile_provider: true,
            tiling_scheme,
            root_tiles,
            level_zero_maximum_geometric_error,
            tiles_to_render: Vec::new(),
            tiles_rendered_this_frame: Vec::new(),
            tiles_to_update_heights: Vec::new(),
            tile_load_queue_high: Vec::new(),
            tile_load_queue_medium: Vec::new(),
            tile_load_queue_low: Vec::new(),
            tiles_invalidated: false,
            last_selection_frame_number: None,
            last_tile_load_queue_length: 0,
            is_destroyed: false,
            debug_tiles_visited: 0,
            debug_max_depth_visited: 0,
            debug_tiles_culled: 0,
            debug_tiles_waiting_for_children: 0,
            debug_tiles_rendered: 0,
            debug_max_depth: 0,
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

    /// Returns `_loadQueueTimeSlice` in milliseconds.
    pub fn load_queue_time_slice(&self) -> f64 { self.load_queue_time_slice }

    /// Sets `_loadQueueTimeSlice` in milliseconds.
    pub fn set_load_queue_time_slice(&mut self, value: f64) { self.load_queue_time_slice = value; }

    /// Returns whether tile loading collapses into the traversal frame.
    pub fn synchronous_tile_provider(&self) -> bool { self.synchronous_tile_provider }

    /// Controls how the tile-provider state machine is modelled.
    ///
    /// DEVIATION: CesiumJS's `GlobeSurfaceTileProvider.loadTile` drives
    /// `GlobeSurfaceTile.processStateMachine` across many frames before it sets
    /// `tile.renderable = true`, and `selectTilesForRendering` refuses to even
    /// *visit* a level-zero tile that is not renderable — it queues it High and
    /// bumps `tilesWaitingForChildren` instead. The port has no asynchronous
    /// terrain/imagery pipeline inside the traversal, so with this flag set
    /// (the default) a tile is collapsed to its terminal state
    /// (`Done` + `renderable`) the moment the traversal reaches it: roots
    /// before the renderable test, children right after `ensure_children`. That
    /// is exactly the fixed point the real provider reaches, and it keeps
    /// single-frame callers (tests, `Scene::render`) seeing a full render list.
    ///
    /// Clear the flag to exercise the real multi-frame path: frame 1 renders
    /// nothing and fills the High queue, `end_frame` loads it, and frame 2
    /// renders. The anti-flicker kick path only becomes reachable this way,
    /// since with synchronous loading every visited tile is renderable.
    pub fn set_synchronous_tile_provider(&mut self, value: bool) {
        self.synchronous_tile_provider = value;
    }

    /// Returns the tiling scheme.
    pub fn tiling_scheme(&self) -> &dyn TilingScheme { self.tiling_scheme.as_ref() }

    /// Returns the root (level-zero) tiles.
    pub fn root_tiles(&self) -> &[QuadtreeTile] { &self.root_tiles }

    /// Returns the tiles selected for rendering this frame.
    ///
    /// DEVIATION: entries are [`QuadtreeTile::snapshot`]s, not live references —
    /// the tree is owned by `root_tiles`. All selection bookkeeping mutates the
    /// real tiles; the snapshot is taken after those mutations so it carries the
    /// final `last_selection_result`.
    pub fn tiles_to_render(&self) -> &[QuadtreeTile] { &self.tiles_to_render }

    /// Returns the tiles rendered in the most recent frame.
    ///
    /// Mirrors `_tilesRenderedThisFrame`, the set `Globe.pickWorldCoordinates`
    /// and `QuadtreePrimitive.forEachRenderedTile` iterate. Cleared by
    /// [`QuadtreePrimitive::begin_frame`], filled by the traversal.
    pub fn tiles_rendered_this_frame(&self) -> &[TileKey] {
        &self.tiles_rendered_this_frame
    }

    /// Invokes a closure for each tile rendered in the most recent frame.
    ///
    /// Mirrors `QuadtreePrimitive#forEachRenderedTile`. DEVIATION: the JS
    /// passes the live `QuadtreeTile`; the port passes its [`TileKey`] because
    /// the tree is owned by `root_tiles` and cannot be borrowed while the
    /// closure runs.
    pub fn for_each_rendered_tile(&self, tile_function: impl FnMut(TileKey)) {
        self.tiles_rendered_this_frame
            .iter()
            .copied()
            .for_each(tile_function);
    }

    /// Returns the level-zero maximum geometric error (meters).
    pub fn level_zero_maximum_geometric_error(&self) -> f64 {
        self.level_zero_maximum_geometric_error
    }

    /// Returns the maximum geometric error for a level, mirroring CesiumJS
    /// `GlobeSurfaceTileProvider#getLevelMaximumGeometricError`:
    /// `levelZeroMaximumGeometricError / (1 << level)`.
    ///
    /// `wrapping_shl` reproduces JavaScript's `1 << level`, which shifts by
    /// `level % 32` and yields a negative divisor at level 31.
    pub fn get_level_maximum_geometric_error(&self, level: i32) -> f64 {
        self.level_zero_maximum_geometric_error / (1i32.wrapping_shl(level as u32) as f64)
    }

    /// Returns true when all three load queues are empty.
    ///
    /// Mirrors the CesiumJS `tilesLoaded` getter. Note the queues are cleared in
    /// `beginFrame`, not drained by `processTileLoadQueue`, so this is only true
    /// once nothing needed loading during the frame — which is what makes
    /// `Scene` stop requesting further render ticks.
    pub fn tiles_loaded(&self) -> bool {
        self.tile_load_queue_high.is_empty()
            && self.tile_load_queue_medium.is_empty()
            && self.tile_load_queue_low.is_empty()
    }

    // ---- Frame lifecycle ----

    /// Initializes values for a new render frame and prepares the tile load
    /// queue.
    ///
    /// Mirrors `QuadtreePrimitive#beginFrame`.
    pub fn begin_frame(&mut self, frame_state: &FrameState) {
        // CesiumJS tests `frameState.passes.render`; the port's `FramePasses`
        // spells the main colour pass `main`.
        if !frame_state.passes.main {
            return;
        }

        if self.tiles_invalidated {
            self.invalidate_all_tiles_now();
            self.tiles_invalidated = false;
        }

        // CesiumJS: `this._tileProvider.initialize(frameState)` — texture
        // reprojection setup, not ported.

        self.clear_tile_load_queue();

        // CesiumJS: `if (this._debug.suspendLodUpdate) return;` then
        // `_tileReplacementQueue.markStartOfRenderFrame()` — the tile
        // replacement queue is not ported, so there is nothing left to do
        // before clearing the rendered-this-frame set.
        self.tiles_rendered_this_frame.clear();
    }

    /// Updates the tile provider imagery and continues to process the tile load
    /// queue.
    ///
    /// Mirrors `QuadtreePrimitive#update`, which does **not** traverse: it only
    /// forwards to `tileProvider.update(frameState)` when the provider defines
    /// it. DEVIATION: the port's provider work lives on
    /// [`crate::globe_surface_tile_provider::GlobeSurfaceTileProvider`], which
    /// [`crate::globe::Globe`] owns separately, so this is a no-op. Tile
    /// selection happens in [`QuadtreePrimitive::render`], matching CesiumJS.
    pub fn update(&mut self, _frame_state: &FrameState) {}

    /// Selects new tiles to load based on the frame state and creates render
    /// commands.
    ///
    /// Mirrors `QuadtreePrimitive#render`. DEVIATION:
    /// `createRenderCommandsForSelectedTiles` is not ported —
    /// [`crate::globe::Globe::render`] walks `tiles_to_render` and issues the
    /// wgpu draws itself.
    pub fn render(&mut self, frame_state: &FrameState) {
        if frame_state.passes.main {
            // CesiumJS: `tileProvider.beginUpdate(frameState)`.
            self.select_tiles_for_rendering(frame_state);
            // CesiumJS: `createRenderCommandsForSelectedTiles`, `endUpdate`.
        }

        // CesiumJS: `if (passes.pick && this._tilesToRender.length > 0)
        // tileProvider.updateForPick(frameState)` — not ported.
    }

    /// Mirrors `QuadtreePrimitive#endFrame`: process the load queue, update
    /// heights, and raise the tile-load-progress event.
    pub fn end_frame(&mut self, frame_state: &FrameState) {
        if !frame_state.passes.main || frame_state.mode == SceneMode::Morphing {
            // Only process the load queue for a single pass. Don't process the
            // load queue or update heights during the morph flights.
            return;
        }

        self.process_tile_load_queue(frame_state);
        // CesiumJS: `updateHeights(this, frameState)` — not ported.
        self.update_tile_load_progress();
    }

    /// Invalidates and frees all the tiles in the quadtree. The tiles must be
    /// reloaded before they can be displayed.
    ///
    /// Mirrors the public `QuadtreePrimitive#invalidateAllTiles`, which only
    /// sets the flag; the real work runs from `beginFrame`.
    pub fn invalidate_all_tiles(&mut self) {
        self.tiles_invalidated = true;
    }

    /// Alias for [`QuadtreePrimitive::invalidate_all_tiles`].
    pub fn invalidate(&mut self) {
        self.invalidate_all_tiles();
    }

    /// Returns true if this object was destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys the WebGL resources held by this object.
    pub fn destroy(&mut self) {
        self.tiles_to_render.clear();
        self.tiles_rendered_this_frame.clear();
        self.tiles_to_update_heights.clear();
        self.tile_load_queue_high.clear();
        self.tile_load_queue_medium.clear();
        self.tile_load_queue_low.clear();
        self.is_destroyed = true;
    }

    // ---- Internals ----

    /// Mirrors the module-level `invalidateAllTiles(primitive)`.
    ///
    /// DEVIATION: the replacement-queue reset, the `customData` re-registration
    /// and `tileProvider.cancelReprojections()` are not ported. Freeing the
    /// level-zero tiles is modelled by rebuilding them, which also drops every
    /// descendant (CesiumJS's `freeResources` walks the child getters the same
    /// way once `_levelZeroTiles` is set to `undefined`).
    fn invalidate_all_tiles_now(&mut self) {
        self.clear_tile_load_queue();
        self.root_tiles = QuadtreeTile::create_level_zero_tiles(
            self.tiling_scheme.as_ref(),
            self.level_zero_maximum_geometric_error,
        );
        self.tiles_to_update_heights.clear();
    }

    /// Mirrors the module-level `clearTileLoadQueue(primitive)`.
    fn clear_tile_load_queue(&mut self) {
        self.debug_max_depth = 0;
        self.debug_max_depth_visited = 0;
        self.debug_tiles_visited = 0;
        self.debug_tiles_culled = 0;
        self.debug_tiles_rendered = 0;
        self.debug_tiles_waiting_for_children = 0;

        self.tile_load_queue_high.clear();
        self.tile_load_queue_medium.clear();
        self.tile_load_queue_low.clear();
    }

    /// Mirrors `selectTilesForRendering(primitive, frameState)`.
    fn select_tiles_for_rendering(&mut self, frame_state: &FrameState) {
        // CesiumJS: `if (debug.suspendLodUpdate) return;` — not ported.

        // Clear the render list.
        self.tiles_to_render.clear();
        // CesiumJS clears `_tilesRenderedThisFrame` in `beginFrame`, not here;
        // the set spans every render pass of the frame, so this only guards the
        // case where `render` runs without a preceding `beginFrame`.
        self.tiles_rendered_this_frame.clear();

        // CesiumJS creates `_levelZeroTiles` lazily from
        // `tileProvider.tilingScheme`; the port builds them in the constructor
        // and rebuilds them on invalidation.

        let mut camera_position_cartographic = Cartographic::default();
        let camera_position_cartographic = if self
            .tiling_scheme
            .ellipsoid()
            .cartesian_to_cartographic(&frame_state.camera_position, &mut camera_position_cartographic)
        {
            Some(camera_position_cartographic)
        } else {
            None
        };

        // Sort the level zero tiles by the distance from the center to the
        // camera. The level zero tiles aren't necessarily a nice neat quad, so
        // we can't use the quadtree ordering we use elsewhere in the tree.
        //
        // DEVIATION: CesiumJS reads `comparisonPoint.longitude` unconditionally
        // and would raise a `TypeError` when `camera.positionCartographic` is
        // `undefined` (camera below the ellipsoid surface normal / at the
        // centre). The port keeps the existing order instead.
        if let Some(comparison_point) = camera_position_cartographic {
            self.root_tiles.sort_by(|a, b| {
                compare_distance_to_point(a, b, &comparison_point)
            });
        }

        // CesiumJS: `_cameraReferenceFrameOriginCartographic` from
        // `Matrix4.getTranslation(camera.transform)`; see `Traversal`.
        let camera_reference_frame_origin_cartographic = None;

        let mut tiles_to_render = std::mem::take(&mut self.tiles_to_render);
        let mut tiles_rendered_this_frame =
            std::mem::take(&mut self.tiles_rendered_this_frame);
        let mut tiles_to_update_heights = std::mem::take(&mut self.tiles_to_update_heights);
        let mut tile_load_queue_high = std::mem::take(&mut self.tile_load_queue_high);
        let mut tile_load_queue_medium = std::mem::take(&mut self.tile_load_queue_medium);
        let mut tile_load_queue_low = std::mem::take(&mut self.tile_load_queue_low);
        let mut root_tiles = std::mem::take(&mut self.root_tiles);

        {
            let mut ctx = Traversal {
                tiling_scheme: self.tiling_scheme.as_ref(),
                maximum_screen_space_error: self.maximum_screen_space_error,
                loading_descendant_limit: self.loading_descendant_limit,
                preload_ancestors: self.preload_ancestors,
                preload_siblings: self.preload_siblings,
                maximum_level: self.maximum_level,
                level_zero_maximum_geometric_error: self.level_zero_maximum_geometric_error,
                synchronous_tile_provider: self.synchronous_tile_provider,
                camera_position: frame_state.camera_position,
                camera_position_cartographic,
                camera_reference_frame_origin_cartographic,
                drawing_buffer_height: frame_state.drawing_buffer_height as f64,
                sse_denominator: frame_state.sse_denominator,
                pixel_ratio: frame_state.pixel_ratio,
                frame_number: frame_state.frame_number,
                last_selection_frame_number: self.last_selection_frame_number,
                tiles_to_render: &mut tiles_to_render,
                tiles_rendered_this_frame: &mut tiles_rendered_this_frame,
                tiles_to_update_heights: &mut tiles_to_update_heights,
                tile_load_queue_high: &mut tile_load_queue_high,
                tile_load_queue_medium: &mut tile_load_queue_medium,
                tile_load_queue_low: &mut tile_load_queue_low,
                tiles_visited: 0,
                tiles_culled: 0,
                max_depth_visited: 0,
                tiles_waiting_for_children: 0,
            };

            // Traverse in depth-first, near-to-far order.
            for root in root_tiles.iter_mut() {
                if ctx.synchronous_tile_provider {
                    mark_tile_loaded(root);
                }
                if !root.renderable {
                    queue_tile_load(&mut ctx, LoadQueue::High, root);
                    ctx.tiles_waiting_for_children += 1;
                } else {
                    let mut details = TraversalDetails::new();
                    visit_if_visible(&mut ctx, root, false, &mut details);
                }
            }

            self.debug_tiles_visited = ctx.tiles_visited;
            self.debug_tiles_culled = ctx.tiles_culled;
            self.debug_max_depth_visited = ctx.max_depth_visited;
            self.debug_tiles_waiting_for_children = ctx.tiles_waiting_for_children;
        }

        self.root_tiles = root_tiles;
        self.tiles_to_render = tiles_to_render;
        self.tiles_rendered_this_frame = tiles_rendered_this_frame;
        self.tiles_to_update_heights = tiles_to_update_heights;
        self.tile_load_queue_high = tile_load_queue_high;
        self.tile_load_queue_medium = tile_load_queue_medium;
        self.tile_load_queue_low = tile_load_queue_low;

        self.last_selection_frame_number = Some(frame_state.frame_number);
    }

    /// Mirrors `processTileLoadQueue(primitive, frameState)`.
    fn process_tile_load_queue(&mut self, _frame_state: &FrameState) {
        if self.tile_load_queue_high.is_empty()
            && self.tile_load_queue_medium.is_empty()
            && self.tile_load_queue_low.is_empty()
        {
            return;
        }

        // CesiumJS: `_tileReplacementQueue.trimTiles(tileCacheSize)` — not
        // ported (no tile is ever freed).

        let end_time = Instant::now() + Duration::from_secs_f64(self.load_queue_time_slice / 1000.0);

        let mut high = std::mem::take(&mut self.tile_load_queue_high);
        let mut medium = std::mem::take(&mut self.tile_load_queue_medium);
        let mut low = std::mem::take(&mut self.tile_load_queue_low);

        let did_some_loading = self.process_single_priority_load_queue(&mut high, end_time, false);
        let did_some_loading =
            self.process_single_priority_load_queue(&mut medium, end_time, did_some_loading);
        self.process_single_priority_load_queue(&mut low, end_time, did_some_loading);

        self.tile_load_queue_high = high;
        self.tile_load_queue_medium = medium;
        self.tile_load_queue_low = low;
    }

    /// Mirrors `processSinglePriorityLoadQueue`.
    ///
    /// The `getTimestamp() < endTime || !didSomeLoading` guard guarantees at
    /// least one tile is loaded per frame even when the time slice has already
    /// expired, so a starved queue can never deadlock.
    fn process_single_priority_load_queue(
        &mut self,
        load_queue: &mut Vec<TileKey>,
        end_time: Instant,
        did_some_loading: bool,
    ) -> bool {
        // CesiumJS sorts only when `tileProvider.computeTileLoadPriority !==
        // undefined`. The port has no priority function, so every tile keeps the
        // constructor's `_loadPriority` of 0.0 and the stable sort below is a
        // no-op — the same observable order CesiumJS produces when all
        // priorities tie.
        load_queue.sort_by(|a, b| {
            let pa = tile_load_priority(&self.root_tiles, *a);
            let pb = tile_load_priority(&self.root_tiles, *b);
            pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut did_some_loading = did_some_loading;
        let mut index = 0;
        let len = load_queue.len();
        while index < len && (Instant::now() < end_time || !did_some_loading) {
            let key = load_queue[index];
            // CesiumJS: `_tileReplacementQueue.markTileRendered(tile)` — not
            // ported.
            self.load_tile(key);
            did_some_loading = true;
            index += 1;
        }
        did_some_loading
    }

    /// Stand-in for `tileProvider.loadTile(frameState, tile)`.
    ///
    /// DEVIATION: CesiumJS's `GlobeSurfaceTile.processStateMachine` walks
    /// terrain and imagery through START → LOADING → PROCESSING → COMPLETE over
    /// many frames and only sets `tile.renderable` at COMPLETE. The port has no
    /// asynchronous imagery pipeline inside the quadtree, so this collapses
    /// straight to that terminal state.
    fn load_tile(&mut self, key: TileKey) {
        if let Some(tile) = find_tile_mut(&mut self.root_tiles, key) {
            tile.load_state = QuadtreeTileLoadState::Done;
            tile.renderable = true;
        }
    }

    /// Mirrors `updateTileLoadProgress(primitive, frameState)`.
    ///
    /// DEVIATION: the `_tileLoadProgressEvent` raise (queued onto
    /// `frameState.afterRender`) is not ported; `_lastTileLoadQueueLength` is
    /// still tracked so the change-detection semantics are visible.
    fn update_tile_load_progress(&mut self) {
        let current_load_queue_length = self.tile_load_queue_high.len()
            + self.tile_load_queue_medium.len()
            + self.tile_load_queue_low.len();

        if current_load_queue_length != self.last_tile_load_queue_length || self.tiles_invalidated {
            // CesiumJS: `frameState.afterRender.push(() => raiseEvent())`.
            self.last_tile_load_queue_length = current_load_queue_length;
        }

        // CesiumJS gates this on `_debug.enableDebugOutput`; both values are
        // pure reductions over the render list, so computing them always is
        // harmless.
        self.debug_max_depth = self
            .tiles_to_render
            .iter()
            .fold(-1, |max, tile| max.max(tile.level));
        self.debug_tiles_rendered = self.tiles_to_render.len();
    }
}

// ---- Module-level traversal functions ----

/// Mirrors `compareDistanceToPoint(a, b)` with `comparisonPoint` bound.
fn compare_distance_to_point(
    a: &QuadtreeTile,
    b: &QuadtreeTile,
    comparison_point: &Cartographic,
) -> std::cmp::Ordering {
    let center = Rectangle::center(&a.rectangle);
    let alon = center.longitude - comparison_point.longitude;
    let alat = center.latitude - comparison_point.latitude;

    let center = Rectangle::center(&b.rectangle);
    let blon = center.longitude - comparison_point.longitude;
    let blat = center.latitude - comparison_point.latitude;

    let difference = alon * alon + alat * alat - (blon * blon + blat * blat);
    // `Array.prototype.sort` treats a NaN comparator result as 0 (leaves the
    // pair in place), which is what `unwrap_or(Equal)` reproduces.
    difference.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
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

/// Stand-in for `tileProvider.computeTileVisibility(tile, frameState, occluders)`.
///
/// CesiumJS's `GlobeSurfaceTileProvider.computeTileVisibility` does two jobs:
/// it stores `tile._distance` (which `screenSpaceError` then reads) and it
/// frustum/horizon-culls. The port keeps the distance computation and returns
/// `Visibility::Partial` for the cull — `FrameState` carries no `cullingVolume`
/// and the primitive holds no `EllipsoidOccluder`. DEVIATION, tracked in
/// `docs/deviations.md`.
fn compute_tile_visibility(ctx: &Traversal<'_>, tile: &mut QuadtreeTile) -> Visibility {
    tile.camera_distance = compute_tile_distance(tile, &ctx.camera_position);
    Visibility::Partial
}

/// Mirrors `screenSpaceError(primitive, frameState, tile)`, 3D perspective
/// branch.
///
/// `error = (getLevelMaximumGeometricError(level) * drawingBufferHeight)
/// / (distance * sseDenominator)`, minus the fog term when fog is enabled,
/// then divided by `pixelRatio`.
///
/// The zero-distance case is deliberately *not* special-cased: CesiumJS lets
/// IEEE-754 produce `Infinity` from `positive / 0`, which fails the
/// `error < maximumScreenSpaceError` test and drives refinement to the ceiling.
fn compute_screen_space_error(ctx: &Traversal<'_>, tile: &QuadtreeTile) -> f64 {
    let max_geometric_error =
        ctx.level_zero_maximum_geometric_error / (1i32.wrapping_shl(tile.level as u32) as f64);
    let distance = tile.camera_distance;
    let mut error =
        (max_geometric_error * ctx.drawing_buffer_height) / (distance * ctx.sse_denominator);
    // CesiumJS: `if (frameState.fog.enabled) error -= CesiumMath.fog(...) *
    // frameState.fog.sse` — FrameState carries no fog block.
    error /= ctx.pixel_ratio;
    error
}

/// Stand-in for `tileProvider.canRefine(tile)`.
///
/// CesiumJS's `GlobeSurfaceTileProvider.canRefine` returns true when the tile's
/// terrain data is present and otherwise consults
/// `terrainProvider.getTileDataAvailable` for the children. The port's ceiling
/// is `maximum_level`, which [`crate::globe::Globe::begin_frame`] derives from
/// the terrain fetcher's own `maximumLevel`.
fn can_refine(ctx: &Traversal<'_>, tile: &QuadtreeTile) -> bool {
    match ctx.maximum_level {
        Some(maximum) => tile.level < maximum,
        None => true,
    }
}

/// Mirrors `containsNeededPosition(primitive, tile)`.
fn contains_needed_position(ctx: &Traversal<'_>, tile: &QuadtreeTile) -> bool {
    if let Some(position) = ctx.camera_position_cartographic {
        if Rectangle::contains(&tile.rectangle, &position) {
            return true;
        }
    }
    if let Some(origin) = ctx.camera_reference_frame_origin_cartographic {
        if Rectangle::contains(&tile.rectangle, &origin) {
            return true;
        }
    }
    false
}

/// Mirrors `queueTileLoad(primitive, queue, tile, frameState)`.
fn queue_tile_load(ctx: &mut Traversal<'_>, queue: LoadQueue, tile: &QuadtreeTile) {
    if !tile.needs_loading() {
        return;
    }

    // CesiumJS: `tile._loadPriority = tileProvider.computeTileLoadPriority(tile,
    // frameState)` when the provider defines it. Not ported, so the priority
    // keeps its constructor value and the sort in
    // `process_single_priority_load_queue` is a no-op.

    let key = tile.key();
    match queue {
        LoadQueue::High => ctx.tile_load_queue_high.push(key),
        LoadQueue::Medium => ctx.tile_load_queue_medium.push(key),
        LoadQueue::Low => ctx.tile_load_queue_low.push(key),
    }
}

/// Mirrors `addTileToRenderList(primitive, tile)`.
///
/// CesiumJS also adds the tile to `_tilesRenderedThisFrame`, a `Set` consumed
/// by `updateHeights` and `Globe.pickWorldCoordinates`. The `Set` keeps
/// insertion order and deduplicates, so a `Vec` with a `contains` guard is the
/// exact equivalent; `updateHeights` itself is not ported.
fn add_tile_to_render_list(ctx: &mut Traversal<'_>, tile: &QuadtreeTile) {
    ctx.tiles_to_render.push(tile.snapshot());
    let key = tile.key();
    if !ctx.tiles_rendered_this_frame.contains(&key) {
        ctx.tiles_rendered_this_frame.push(key);
    }
}

/// DEVIATION: collapses the CesiumJS tile-provider state machine to its
/// terminal state. See
/// [`QuadtreePrimitive::set_synchronous_tile_provider`].
fn mark_tile_loaded(tile: &mut QuadtreeTile) {
    tile.load_state = QuadtreeTileLoadState::Done;
    tile.renderable = true;
}

/// The `lastFrameSelectionResult` expression CesiumJS inlines in both
/// `visitTile` and `visitIfVisible`.
///
/// On the very first frame `tile._lastSelectionResultFrame` and
/// `primitive._lastSelectionFrameNumber` are *both* `undefined`, and
/// `undefined === undefined` is `true`, so a never-visited tile reads back its
/// stored `NONE` rather than being forced to `NONE`. Both are `NONE` anyway, so
/// the distinction only matters from frame two on — but it is what makes
/// condition 2 ("culled or not visited last frame") fire on frame one.
fn last_frame_selection_result(ctx: &Traversal<'_>, tile: &QuadtreeTile) -> TileSelectionResult {
    if tile.last_selection_result_frame == ctx.last_selection_frame_number {
        tile.last_selection_result
    } else {
        TileSelectionResult::NONE
    }
}

/// Mirrors `visitTile(primitive, frameState, tile, ancestorMeetsSse, traversalDetails)`.
///
/// When this is called with a tile:
///
/// * the tile has been determined to be visible (possibly based on a bounding
///   volume that is not very tight-fitting)
/// * its parent tile does *not* meet the SSE (unless `ancestor_meets_sse`, see
///   below)
/// * the tile may or may not be renderable
///
/// `ancestor_meets_sse` is true when a tile higher in the tree already met the
/// SSE and we are refining further only to maintain detail while that higher
/// tile loads.
#[allow(clippy::too_many_arguments)]
fn visit_tile(
    ctx: &mut Traversal<'_>,
    tile: &mut QuadtreeTile,
    mut ancestor_meets_sse: bool,
    details: &mut TraversalDetails,
) {
    ctx.tiles_visited += 1;

    // CesiumJS: `_tileReplacementQueue.markTileRendered(tile)` and
    // `tile.updateCustomData()` — neither subsystem is ported.

    if tile.level > ctx.max_depth_visited {
        ctx.max_depth_visited = tile.level;
    }

    tile.screen_space_error = compute_screen_space_error(ctx, tile);
    let meets_sse = tile.screen_space_error < ctx.maximum_screen_space_error;

    let last_frame_selection_result = last_frame_selection_result(ctx, tile);

    if meets_sse || ancestor_meets_sse {
        // This tile (or an ancestor) is the one we want to render this frame,
        // but we'll do different things depending on the state of this tile and
        // on what we did _last_ frame.
        //
        // We can render it if _any_ of the following are true:
        // 1. We rendered it (or kicked it) last frame.
        // 2. This tile was culled last frame, or it wasn't even visited because
        //    an ancestor was culled.
        // 3. The tile is completely done loading.
        // 4. a) Terrain is ready, and
        //    b) All necessary imagery is ready. Necessary imagery is imagery
        //       that was rendered with this tile or any descendants last frame.
        //       Such imagery is required because rendering this tile without it
        //       would cause detail to disappear.
        //
        // Determining condition 4 is more expensive, so we check the others
        // first.
        //
        // Note that even if we decide to render a tile here, it may later get
        // "kicked" in favor of an ancestor.
        let one_rendered_last_frame =
            last_frame_selection_result.original_result() == TileSelectionResult::RENDERED;
        let two_culled_or_not_visited = last_frame_selection_result.original_result()
            == TileSelectionResult::CULLED
            || last_frame_selection_result == TileSelectionResult::NONE;
        let three_completely_loaded = tile.load_state == QuadtreeTileLoadState::Done;

        let renderable =
            one_rendered_last_frame || two_culled_or_not_visited || three_completely_loaded;

        // Condition 4 delegates to `tileProvider.canRenderWithoutLosingDetail`,
        // guarded by `defined(...)`. Not ported, so it stays absent and
        // `renderable` keeps the value of conditions 1–3 — exactly what CesiumJS
        // does for a provider that omits the method.

        if renderable {
            // Only load this tile if it (not just an ancestor) meets the SSE.
            if meets_sse {
                queue_tile_load(ctx, LoadQueue::Medium, tile);
            }

            tile.last_selection_result_frame = Some(ctx.frame_number);
            tile.last_selection_result = TileSelectionResult::RENDERED;
            // CesiumJS pushes to `_tilesToRender` before writing
            // `_lastSelectionResult`; the port stores snapshots, so the write
            // happens first and the snapshot carries the final value.
            add_tile_to_render_list(ctx, tile);

            details.all_are_renderable = tile.renderable;
            details.any_were_rendered_last_frame =
                last_frame_selection_result == TileSelectionResult::RENDERED;
            details.not_yet_renderable_count = if tile.renderable { 0 } else { 1 };
            if !details.any_were_rendered_last_frame {
                ctx.tiles_to_update_heights.push(tile.key());
            }
            return;
        }

        // This tile is the one we want to render, but it isn't ready yet. Keep
        // refining so that whatever detail was visible last frame stays
        // visible, and load this blocker with high priority.
        ancestor_meets_sse = true;
        if meets_sse {
            queue_tile_load(ctx, LoadQueue::High, tile);
        }
    }

    if can_refine(ctx, tile) {
        tile.ensure_children(ctx.tiling_scheme);
        if ctx.synchronous_tile_provider {
            for child in tile.children.iter_mut() {
                mark_tile_loaded(child);
            }
        }

        let all_are_upsampled = tile.children.len() == 4
            && tile.children[CHILD_SOUTHWEST].upsampled_from_parent
            && tile.children[CHILD_SOUTHEAST].upsampled_from_parent
            && tile.children[CHILD_NORTHWEST].upsampled_from_parent
            && tile.children[CHILD_NORTHEAST].upsampled_from_parent;

        if all_are_upsampled {
            // If all four children were upsampled from this tile then rendering
            // them is pointless — render this tile instead.
            tile.last_selection_result_frame = Some(ctx.frame_number);
            tile.last_selection_result = TileSelectionResult::RENDERED;
            add_tile_to_render_list(ctx, tile);
            queue_tile_load(ctx, LoadQueue::Medium, tile);
            // CesiumJS: `markTileRendered` on each of the four children — not
            // ported.

            details.all_are_renderable = tile.renderable;
            details.any_were_rendered_last_frame =
                last_frame_selection_result == TileSelectionResult::RENDERED;
            details.not_yet_renderable_count = if tile.renderable { 0 } else { 1 };
            if !details.any_were_rendered_last_frame {
                ctx.tiles_to_update_heights.push(tile.key());
            }
            return;
        }

        tile.last_selection_result_frame = Some(ctx.frame_number);
        tile.last_selection_result = TileSelectionResult::REFINED;

        let first_rendered_descendant_index = ctx.tiles_to_render.len();
        let load_index_low = ctx.tile_load_queue_low.len();
        let load_index_medium = ctx.tile_load_queue_medium.len();
        let load_index_high = ctx.tile_load_queue_high.len();
        let tiles_to_update_heights_index = ctx.tiles_to_update_heights.len();

        {
            // Split the four children into disjoint `&mut` borrows so the
            // near-to-far ordering can visit them through one context.
            let (north_half, south_half) = tile.children.split_at_mut(2);
            let (northwest, northeast) = north_half.split_at_mut(1);
            let (southwest, southeast) = south_half.split_at_mut(1);
            let mut quads = TraversalQuadDetails::new();
            visit_visible_children_near_to_far(
                ctx,
                (&mut southwest[0], &mut southeast[0], &mut northwest[0], &mut northeast[0]),
                ancestor_meets_sse,
                &mut quads,
                details,
            );
        }

        if first_rendered_descendant_index != ctx.tiles_to_render.len() {
            let all_are_renderable = details.all_are_renderable;
            let any_were_rendered_last_frame = details.any_were_rendered_last_frame;
            let not_yet_renderable_count = details.not_yet_renderable_count;
            let mut queued_for_load = false;

            if !all_are_renderable && !any_were_rendered_last_frame {
                // None of the descendants are renderable and none of them were
                // rendered last frame, so there is no detail to preserve: kick
                // the whole subtree out of the render list and render this tile
                // instead.
                //
                // CesiumJS walks `workTile.parent` from each rendered descendant
                // up to (but not including) this tile, kicking every tile on the
                // way. Its guard `workTile._lastSelectionResult !==
                // TileSelectionResult.KICKED` compares against an `undefined`
                // constant and is therefore always true; since `kick` is
                // idempotent the walk is equivalent to one without the guard,
                // which is what `kick_ancestors` implements.
                let kicked: Vec<TileKey> = ctx.tiles_to_render[first_rendered_descendant_index..]
                    .iter()
                    .map(|rendered| rendered.key())
                    .collect();
                ctx.tiles_to_render.truncate(first_rendered_descendant_index);
                ctx.tiles_to_update_heights
                    .truncate(tiles_to_update_heights_index);

                let stop = tile.key();
                for key in kicked {
                    kick_ancestors(tile, stop, key);
                }

                tile.last_selection_result = TileSelectionResult::RENDERED;
                add_tile_to_render_list(ctx, tile);

                let was_rendered_last_frame =
                    last_frame_selection_result == TileSelectionResult::RENDERED;
                if !was_rendered_last_frame
                    && not_yet_renderable_count > ctx.loading_descendant_limit
                {
                    // Too many descendants are still loading. Drop everything
                    // they queued this frame and load only this tile.
                    ctx.tile_load_queue_low.truncate(load_index_low);
                    ctx.tile_load_queue_medium.truncate(load_index_medium);
                    ctx.tile_load_queue_high.truncate(load_index_high);
                    queue_tile_load(ctx, LoadQueue::Medium, tile);
                    details.not_yet_renderable_count = if tile.renderable { 0 } else { 1 };
                    queued_for_load = true;
                }
                details.all_are_renderable = tile.renderable;
                details.any_were_rendered_last_frame = was_rendered_last_frame;
                if !was_rendered_last_frame {
                    ctx.tiles_to_update_heights.push(tile.key());
                }

                ctx.tiles_waiting_for_children += 1;
            }

            if ctx.preload_ancestors && !queued_for_load {
                queue_tile_load(ctx, LoadQueue::Low, tile);
            }
        }

        return;
    }

    // We'd like to refine but can't because we have no availability data for
    // this tile's children, so we have no idea if refining would involve a load
    // or an upsample. We'll have to finish loading this tile first in order to
    // find that out, so load this refinement blocker with high priority.
    tile.last_selection_result_frame = Some(ctx.frame_number);
    tile.last_selection_result = TileSelectionResult::RENDERED;
    add_tile_to_render_list(ctx, tile);
    queue_tile_load(ctx, LoadQueue::High, tile);

    details.all_are_renderable = tile.renderable;
    details.any_were_rendered_last_frame =
        last_frame_selection_result == TileSelectionResult::RENDERED;
    details.not_yet_renderable_count = if tile.renderable { 0 } else { 1 };
}

/// Kicks `start` and every ancestor up to (but not including) `stop`.
///
/// Mirrors the `while (workTile !== undefined && ... && workTile !== tile)`
/// loop in `visitTile`.
fn kick_ancestors(subtree_root: &mut QuadtreeTile, stop: TileKey, start: TileKey) {
    let mut current = Some(start);
    while let Some(key) = current {
        if key == stop {
            // `workTile === tile`
            break;
        }
        match descendant_mut(subtree_root, key) {
            Some(work_tile) => {
                work_tile.last_selection_result = work_tile.last_selection_result.kick();
            }
            // `workTile === undefined`
            None => break,
        }
        current = key.parent();
    }
}

/// Resolves a descendant `key` to a mutable reference within `tile`'s subtree.
fn descendant_mut<'a>(
    tile: &'a mut QuadtreeTile,
    key: TileKey,
) -> Option<&'a mut QuadtreeTile> {
    if tile.level == key.level && tile.x == key.x && tile.y == key.y {
        return Some(tile);
    }
    if tile.level >= key.level {
        return None;
    }
    let shift = tile.level;
    let slot = (((key.x >> shift) & 1) as usize) | ((((key.y >> shift) & 1) as usize) << 1);
    match tile.children.get_mut(slot) {
        Some(child) => descendant_mut(child, key),
        None => None,
    }
}

/// Resolves `key` against the level-zero tiles.
fn find_tile_mut<'a>(
    roots: &'a mut [QuadtreeTile],
    key: TileKey,
) -> Option<&'a mut QuadtreeTile> {
    for root in roots.iter_mut() {
        if let Some(found) = descendant_mut(root, key) {
            return Some(found);
        }
    }
    None
}

/// Reads `_loadPriority` for a queued key (used by the priority sort).
fn tile_load_priority(roots: &[QuadtreeTile], key: TileKey) -> f64 {
    fn descend(tile: &QuadtreeTile, key: TileKey) -> Option<f64> {
        if tile.level == key.level && tile.x == key.x && tile.y == key.y {
            return Some(tile.load_priority);
        }
        if tile.level >= key.level {
            return None;
        }
        let shift = tile.level;
        let slot = (((key.x >> shift) & 1) as usize) | ((((key.y >> shift) & 1) as usize) << 1);
        tile.children.get(slot).and_then(|child| descend(child, key))
    }
    roots
        .iter()
        .find_map(|root| descend(root, key))
        .unwrap_or(0.0)
}

/// Mirrors `visitVisibleChildrenNearToFar`.
///
/// Visits the four children nearest-to-camera first so the render list, and
/// therefore the load queues, are populated in the order the viewer will
/// actually notice.
fn visit_visible_children_near_to_far(
    ctx: &mut Traversal<'_>,
    children: (
        &mut QuadtreeTile,
        &mut QuadtreeTile,
        &mut QuadtreeTile,
        &mut QuadtreeTile,
    ),
    ancestor_meets_sse: bool,
    quads: &mut TraversalQuadDetails,
    details: &mut TraversalDetails,
) {
    let (southwest, southeast, northwest, northeast) = children;

    // `cameraPosition` is `frameState.camera.positionCartographic`. When it is
    // `undefined` every `<` comparison is `false`, so CesiumJS falls through to
    // the northeast-quadrant order.
    let (west_of_southwest_east, south_of_southwest_north) =
        match ctx.camera_position_cartographic {
            Some(camera_position) => (
                camera_position.longitude < southwest.rectangle.east,
                camera_position.latitude < southwest.rectangle.north,
            ),
            None => (false, false),
        };

    if west_of_southwest_east {
        if south_of_southwest_north {
            // Camera in southwest quadrant
            visit_if_visible(ctx, southwest, ancestor_meets_sse, &mut quads.southwest);
            visit_if_visible(ctx, southeast, ancestor_meets_sse, &mut quads.southeast);
            visit_if_visible(ctx, northwest, ancestor_meets_sse, &mut quads.northwest);
            visit_if_visible(ctx, northeast, ancestor_meets_sse, &mut quads.northeast);
        } else {
            // Camera in northwest quadrant
            visit_if_visible(ctx, northwest, ancestor_meets_sse, &mut quads.northwest);
            visit_if_visible(ctx, southwest, ancestor_meets_sse, &mut quads.southwest);
            visit_if_visible(ctx, northeast, ancestor_meets_sse, &mut quads.northeast);
            visit_if_visible(ctx, southeast, ancestor_meets_sse, &mut quads.southeast);
        }
    } else if south_of_southwest_north {
        // Camera in southeast quadrant
        visit_if_visible(ctx, southeast, ancestor_meets_sse, &mut quads.southeast);
        visit_if_visible(ctx, southwest, ancestor_meets_sse, &mut quads.southwest);
        visit_if_visible(ctx, northeast, ancestor_meets_sse, &mut quads.northeast);
        visit_if_visible(ctx, northwest, ancestor_meets_sse, &mut quads.northwest);
    } else {
        // Camera in northeast quadrant
        visit_if_visible(ctx, northeast, ancestor_meets_sse, &mut quads.northeast);
        visit_if_visible(ctx, northwest, ancestor_meets_sse, &mut quads.northwest);
        visit_if_visible(ctx, southeast, ancestor_meets_sse, &mut quads.southeast);
        visit_if_visible(ctx, southwest, ancestor_meets_sse, &mut quads.southwest);
    }

    quads.combine(details);
}

/// Mirrors `visitIfVisible`.
fn visit_if_visible(
    ctx: &mut Traversal<'_>,
    tile: &mut QuadtreeTile,
    ancestor_meets_sse: bool,
    details: &mut TraversalDetails,
) {
    if compute_tile_visibility(ctx, tile) != Visibility::None {
        visit_tile(ctx, tile, ancestor_meets_sse, details);
        return;
    }

    ctx.tiles_culled += 1;
    // CesiumJS: `_tileReplacementQueue.markTileRendered(tile)` — not ported.

    details.all_are_renderable = true;
    details.any_were_rendered_last_frame = false;
    details.not_yet_renderable_count = 0;

    if contains_needed_position(ctx, tile) {
        // Load the tile(s) that contain the camera's position and the origin of
        // its reference frame with medium priority. But we only need to load
        // until the terrain is available, no need to load imagery.
        //
        // CesiumJS guards this with `!defined(tile.data) ||
        // !defined(tile.data.vertexArray)`; the port has no per-tile `data`, and
        // `queue_tile_load` already no-ops once the tile stops needing loading,
        // so the guard collapses into that check.
        queue_tile_load(ctx, LoadQueue::Medium, tile);

        let last_frame_selection_result = last_frame_selection_result(ctx, tile);
        if last_frame_selection_result != TileSelectionResult::CULLED_BUT_NEEDED
            && last_frame_selection_result != TileSelectionResult::RENDERED
        {
            ctx.tiles_to_update_heights.push(tile.key());
        }

        tile.last_selection_result = TileSelectionResult::CULLED_BUT_NEEDED;
    } else if ctx.preload_siblings || tile.level == 0 {
        // Load culled level zero tiles with low priority. For all other levels,
        // only load culled tiles if preloadSiblings is enabled.
        queue_tile_load(ctx, LoadQueue::Low, tile);
        tile.last_selection_result = TileSelectionResult::CULLED;
    } else {
        tile.last_selection_result = TileSelectionResult::CULLED;
    }

    tile.last_selection_result_frame = Some(ctx.frame_number);
}

impl Default for QuadtreePrimitive {
    fn default() -> Self { Self::new() }
}
