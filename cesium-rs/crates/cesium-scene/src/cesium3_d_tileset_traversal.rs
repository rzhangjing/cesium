//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetTraversal.js`.
//!
//! Traversal helpers shared by the concrete traversal strategies.

use crate::cesium3_d_tile::Cesium3DTile;
use crate::cesium3_d_tile_optimization_hint::Cesium3DTileOptimizationHint;
use crate::cesium3_d_tile_refine::Cesium3DTileRefine;

/// Priority range tracked across tiles during a traversal pass.
///
/// Mirrors the `_minimumPriority` / `_maximumPriority` objects on the
/// CesiumJS tileset.
#[derive(Debug, Clone, Default)]
pub struct TilePriorityRange {
    /// Distance from camera.
    pub distance: f64,
    /// Depth in the tile tree.
    pub depth: i32,
    /// Foveated factor.
    pub foveated_factor: f64,
    /// Reverse screen space error.
    pub reverse_screen_space_error: f64,
}

/// Traverses a [`Cesium3DTileset`](crate::cesium3_d_tileset::Cesium3DTileset)
/// to determine which tiles to load and render.
///
/// This type describes an interface and is not intended to be instantiated
/// directly.
///
/// DEVIATION: the CesiumJS `selectTiles(tileset, frameState)` entry point
/// throws an instantiation error (abstract); the concrete strategies
/// (base/skip/most-detailed) are wired up with the renderer track. The
/// pure helpers below are the CPU-portable parts of the module.
pub struct Cesium3DTilesetTraversal {
    /// Whether the traversal is active.
    pub active: bool,
}

impl Cesium3DTilesetTraversal {
    /// Instantiation is not allowed (abstract type).
    ///
    /// Mirrors `Cesium3DTilesetTraversal.selectTiles` raising
    /// `DeveloperError.throwInstantiationError()`.
    ///
    /// # Panics
    /// Always panics with the instantiation error message.
    pub fn select_tiles() -> ! {
        panic!(
            "This function should not be called. This is an abstract class. \
             Use one of the concrete traversal classes instead."
        );
    }

    /// Sort comparator: farthest child first since this is going on a
    /// stack.
    ///
    /// Mirrors `sortChildrenByDistanceToCamera(a, b)`; returns an
    /// `Ordering`-compatible value (`> 0` means `b` sorts before `a`).
    pub fn sort_children_by_distance_to_camera(a: &Cesium3DTile, b: &Cesium3DTile) -> f64 {
        if b.distance_to_camera == 0.0 && a.distance_to_camera == 0.0 {
            return b.center_z_depth - a.center_z_depth;
        }
        b.distance_to_camera - a.distance_to_camera
    }

    /// Determines if a tile can and should be traversed for children
    /// tiles that would contribute to rendering the current view.
    ///
    /// Mirrors `canTraverse(tile)` with the tileset's
    /// `memoryAdjustedScreenSpaceError` passed in.
    pub fn can_traverse(tile: &Cesium3DTile, memory_adjusted_screen_space_error: f64) -> bool {
        if tile.children.is_empty() {
            return false;
        }
        if tile.has_tileset_content || tile.has_implicit_content {
            // Traverse external tileset to visit its root tile.
            // Don't traverse if the subtree is expired because it will be
            // destroyed; expiration is not tracked on the CPU port yet, so
            // content is never expired here.
            // DEVIATION: `tile.contentExpired` is always false until the
            // expiration clock is wired up.
            return true;
        }
        tile.screen_space_error > memory_adjusted_screen_space_error
    }

    /// Marks the tile as visited for the current frame.
    ///
    /// Mirrors `visitTile(tile, frameState)`; the statistics counter is
    /// incremented on the passed-in counter (the tileset owns it).
    pub fn visit_tile(tile: &mut Cesium3DTile, visited_counter: &mut i32, frame_number: u64) {
        *visited_counter += 1;
        tile.visited_frame = frame_number;
    }

    /// Prevents another pass from touching the tile again in the same
    /// frame.
    ///
    /// Mirrors `touchTile(tile, frameState)`; returns whether the tile was
    /// actually touched (cache touching is handled by the caller).
    pub fn touch_tile(tile: &mut Cesium3DTile, frame_number: u64) -> bool {
        if tile.touched_frame == frame_number {
            // Prevents another pass from touching the frame again
            return false;
        }
        tile.touched_frame = frame_number;
        true
    }

    /// Marks a tile as selected for the current frame.
    ///
    /// Mirrors `selectTile(tile, frameState)`. Returns `true` if the tile
    /// was newly selected (not selected in the previous frame).
    ///
    /// The caller is responsible for pushing the tile onto the selected
    /// list; `was_selected_last_frame` is set to track re-style needs.
    pub fn select_tile(tile: &mut Cesium3DTile, frame_number: u64) -> bool {
        let is_newly_selected = tile.selected_frame < frame_number.saturating_sub(1);
        tile.was_selected_last_frame = true;
        tile.selected_frame = frame_number;
        is_newly_selected
    }

    /// Adds a tile to the request list if appropriate.
    ///
    /// Mirrors `loadTile(tile, frameState)`. Returns `true` when the tile
    /// should be requested.
    ///
    /// `memory_adjusted_sse` is not used here but kept for symmetry with
    /// `can_traverse`; the real gating is done by `is_on_screen_long_enough`
    /// and the foveated delay check.
    pub fn load_tile(
        tile: &Cesium3DTile,
        frame_number: u64,
        cull_requests_while_moving: bool,
        cull_multiplier: f64,
        camera_delta_magnitude: f64,
    ) -> bool {
        // Already requested this frame, or content is already loaded.
        if tile.requested_frame == frame_number
            || (!tile.has_unloaded_renderable_content())
        {
            return false;
        }

        if !Self::is_on_screen_long_enough(
            tile,
            cull_requests_while_moving,
            cull_multiplier,
            camera_delta_magnitude,
        ) {
            return false;
        }

        // The `priorityDeferred` flag is not tracked on the CPU port yet;
        // DEVIATION: always treated as false.
        true
    }

    /// Prevents unnecessary loads while the camera is moving by comparing
    /// travel distance to tile size.
    ///
    /// Mirrors `isOnScreenLongEnough(tile, frameState)`.
    pub fn is_on_screen_long_enough(
        tile: &Cesium3DTile,
        cull_requests_while_moving: bool,
        cull_multiplier: f64,
        camera_delta_magnitude: f64,
    ) -> bool {
        if !cull_requests_while_moving {
            return true;
        }

        let diameter = (tile.bounding_sphere_radius() * 2.0).max(1.0);
        let movement_ratio = (cull_multiplier * camera_delta_magnitude) / diameter;
        movement_ratio < 1.0
    }

    /// Resets tile flags and re-evaluates visibility and priority.
    ///
    /// Mirrors `updateTile(tile, frameState)`. Sets:
    /// - `was_min_priority_child = false`
    /// - `should_select = false`
    /// - `final_resolution = true`
    ///
    /// Returns the values that should be written to the tile's
    /// `was_min_priority_child`, `should_select`, and `final_resolution`
    /// fields.
    pub fn update_tile_flags() -> UpdateTileFlags {
        UpdateTileFlags {
            was_min_priority_child: false,
            should_select: false,
            final_resolution: true,
        }
    }

    /// Updates the minimum/maximum priority range with the given tile's
    /// priority values.
    ///
    /// Mirrors `updateMinimumMaximumPriority(tile)`.
    pub fn update_minimum_maximum_priority(
        min_priority: &mut TilePriorityRange,
        max_priority: &mut TilePriorityRange,
        distance_to_camera: f64,
        depth: i32,
        foveated_factor: f64,
        reverse_screen_space_error: f64,
    ) {
        max_priority.distance = max_priority.distance.max(distance_to_camera);
        min_priority.distance = min_priority.distance.min(distance_to_camera);
        max_priority.depth = max_priority.depth.max(depth);
        min_priority.depth = min_priority.depth.min(depth);
        max_priority.foveated_factor = max_priority.foveated_factor.max(foveated_factor);
        min_priority.foveated_factor = min_priority.foveated_factor.min(foveated_factor);
        max_priority.reverse_screen_space_error = max_priority
            .reverse_screen_space_error
            .max(reverse_screen_space_error);
        min_priority.reverse_screen_space_error = min_priority
            .reverse_screen_space_error
            .min(reverse_screen_space_error);
    }

    /// Determines whether a tile meets the screen space error early,
    /// using the parent's geometric error with the child's bounding
    /// volume.
    ///
    /// Mirrors `meetsScreenSpaceErrorEarly(tile, frameState)`.
    /// Returns `false` when there is no parent or the parent is not ADD
    /// refinement.
    pub fn meets_screen_space_error_early(
        has_parent: bool,
        parent_has_tileset_content: bool,
        parent_has_implicit_content: bool,
        parent_refine: Cesium3DTileRefine,
        tile_screen_space_error: f64,
        memory_adjusted_sse: f64,
    ) -> bool {
        if !has_parent
            || parent_has_tileset_content
            || parent_has_implicit_content
            || parent_refine != Cesium3DTileRefine::Add
        {
            return false;
        }
        tile_screen_space_error <= memory_adjusted_sse
    }

    /// Checks whether the REPLACE refinement optimization applies and
    /// no children are visible.
    ///
    /// Mirrors the optimization block in `updateTileVisibility`.
    pub fn should_cull_by_children_union(
        refine: Cesium3DTileRefine,
        optim_hint: Cesium3DTileOptimizationHint,
        has_children: bool,
    ) -> bool {
        refine == Cesium3DTileRefine::Replace
            && optim_hint == Cesium3DTileOptimizationHint::UseOptimization
            && has_children
    }
}

/// Flags returned by [`Cesium3DTilesetTraversal::update_tile_flags`].
#[derive(Debug, Clone, Copy)]
pub struct UpdateTileFlags {
    /// Reset `wasMinPriorityChild` to false.
    pub was_min_priority_child: bool,
    /// Reset `shouldSelect` to false.
    pub should_select: bool,
    /// Set `finalResolution` to true.
    pub final_resolution: bool,
}

impl Default for Cesium3DTilesetTraversal {
    fn default() -> Self { Self { active: false } }
}
