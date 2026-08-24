//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetTraversal.js`.
//!
//! Traversal helpers shared by the concrete traversal strategies.

use crate::cesium3_d_tile::Cesium3DTile;

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
    _private: (),
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
}

impl Default for Cesium3DTilesetTraversal {
    fn default() -> Self { Self { _private: () } }
}
