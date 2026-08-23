//! Ported from `packages/engine/Source/Scene/QuadtreeTile.js`.
//!
//! A single tile in the quadtree used for globe surface rendering.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::rectangle::Rectangle;

use crate::quadtree_tile_load_state::QuadtreeTileLoadState;

/// A single tile in the quadtree used for globe surface rendering.
///
/// Each tile has a level, x/y position, a bounding rectangle, a bounding sphere,
/// and tracks its load state and parent/child relationships.
pub struct QuadtreeTile {
    /// The x coordinate of the tile (column).
    pub x: i32,
    /// The y coordinate of the tile (row).
    pub y: i32,
    /// The level of the tile in the quadtree (0 = root).
    pub level: i32,

    /// The cartographic rectangle covered by this tile.
    pub rectangle: Rectangle,
    /// The bounding sphere of this tile in world coordinates.
    pub bounding_sphere: BoundingSphere,

    /// The current load state of this tile.
    pub load_state: QuadtreeTileLoadState,

    /// The screen-space error of this tile (in pixels).
    pub screen_space_error: f64,

    /// The distance from the camera to this tile (in meters).
    pub camera_distance: f64,

    /// Whether this tile was rendered last frame.
    pub was_rendered: bool,

    /// The number of loading descendants (used for load throttling).
    pub loading_descendant_count: i32,

    /// Whether this tile is eligible for rendering (all imagery loaded, etc.).
    pub renderable: bool,

    /// The pick bounding sphere (may differ from bounding_sphere for better picking).
    pub pick_bounding_sphere: BoundingSphere,
}

impl QuadtreeTile {
    /// Creates a new QuadtreeTile.
    pub fn new(x: i32, y: i32, level: i32, rectangle: Rectangle) -> Self {
        Self {
            x,
            y,
            level,
            rectangle,
            bounding_sphere: BoundingSphere::default(),
            load_state: QuadtreeTileLoadState::Start,
            screen_space_error: 0.0,
            camera_distance: 0.0,
            was_rendered: false,
            loading_descendant_count: 0,
            renderable: false,
            pick_bounding_sphere: BoundingSphere::default(),
        }
    }

    /// Creates the root tile for a geographic tiling scheme.
    pub fn create_root() -> Self {
        Self {
            x: 0,
            y: 0,
            level: 0,
            rectangle: Rectangle::new(
                -std::f64::consts::PI,
                -std::f64::consts::FRAC_PI_2,
                std::f64::consts::PI,
                std::f64::consts::FRAC_PI_2,
            ),
            bounding_sphere: BoundingSphere::default(),
            load_state: QuadtreeTileLoadState::Start,
            screen_space_error: 0.0,
            camera_distance: 0.0,
            was_rendered: false,
            loading_descendant_count: 0,
            renderable: false,
            pick_bounding_sphere: BoundingSphere::default(),
        }
    }

    /// Returns whether this tile is done loading (ready for rendering).
    pub fn is_done(&self) -> bool {
        self.load_state == QuadtreeTileLoadState::Done
    }

    /// Returns whether this tile is currently loading.
    pub fn is_loading(&self) -> bool {
        self.load_state == QuadtreeTileLoadState::Loading
    }

    /// Returns whether this tile has failed to load.
    pub fn is_failed(&self) -> bool {
        self.load_state == QuadtreeTileLoadState::Failed
    }

    /// Resets the tile state for a new frame.
    pub fn reset(&mut self) {
        self.was_rendered = false;
        self.loading_descendant_count = 0;
        self.renderable = false;
        self.screen_space_error = 0.0;
        self.camera_distance = 0.0;
    }
}

impl Default for QuadtreeTile {
    fn default() -> Self {
        Self::create_root()
    }
}
