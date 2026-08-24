//! Ported from `packages/engine/Source/Scene/QuadtreeTile.js`.
//!
//! A single tile in the quadtree used for globe surface rendering.

use cesium_core::bounding_sphere::BoundingSphere;
use cesium_core::cartesian3::Cartesian3;
use cesium_core::cartographic::Cartographic;
use cesium_core::ellipsoid::Ellipsoid;
use cesium_core::rectangle::Rectangle;
use cesium_core::tiling_scheme::TilingScheme;

use crate::quadtree_tile_load_state::QuadtreeTileLoadState;

/// Child-tile slot indices: mirrors the CesiumJS `northwestChild` /
/// `northeastChild` / `southwestChild` / `southeastChild` getters.
pub const CHILD_NORTHWEST: usize = 0;
pub const CHILD_NORTHEAST: usize = 1;
pub const CHILD_SOUTHWEST: usize = 2;
pub const CHILD_SOUTHEAST: usize = 3;

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

    /// The maximum geometric error (in meters) for this tile's level.
    ///
    /// Mirrors `GlobeSurfaceTileProvider#getLevelMaximumGeometricError`:
    /// `levelZeroMaximumGeometricError / (1 << level)`.
    pub geometric_error: f64,

    /// Child tiles in `[NW, NE, SW, SE]` order. Empty until
    /// [`QuadtreeTile::ensure_children`] creates them during refinement.
    ///
    /// DEVIATION (B4-2): CesiumJS holds children lazily via four getters
    /// with parent back-links; the Rust port owns them in a `Vec` and drops
    /// the parent pointer (traversal owns the stack instead).
    pub children: Vec<QuadtreeTile>,
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
            geometric_error: 0.0,
            children: Vec::new(),
        }
    }

    /// Creates a tile positioned by a tiling scheme (mirrors the CesiumJS
    /// `new QuadtreeTile({ tilingScheme, level, x, y })` constructor path).
    pub fn from_tiling_scheme(
        tiling_scheme: &dyn TilingScheme,
        level: i32,
        x: i32,
        y: i32,
        geometric_error: f64,
    ) -> Self {
        let mut rectangle = Rectangle::default();
        tiling_scheme.tile_xy_to_rectangle(x, y, level, &mut rectangle);
        let mut tile = Self::new(x, y, level, rectangle);
        tile.geometric_error = geometric_error;
        tile.update_bounding_sphere(tiling_scheme.ellipsoid());
        tile
    }

    /// Creates the level-zero tiles for a tiling scheme.
    ///
    /// Mirrors CesiumJS `QuadtreeTile.createLevelZeroTiles`: one tile per
    /// level-zero column (e.g. 2 tiles for the default GeographicTilingScheme,
    /// 1 tile for the default WebMercatorTilingScheme).
    pub fn create_level_zero_tiles(
        tiling_scheme: &dyn TilingScheme,
        level_zero_geometric_error: f64,
    ) -> Vec<QuadtreeTile> {
        let number_of_tiles_x = tiling_scheme.get_number_of_x_tiles_at_level(0);
        let mut tiles = Vec::with_capacity(number_of_tiles_x as usize);
        for i in 0..number_of_tiles_x {
            tiles.push(Self::from_tiling_scheme(
                tiling_scheme,
                0,
                i,
                0,
                level_zero_geometric_error,
            ));
        }
        tiles
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
            geometric_error: 0.0,
            children: Vec::new(),
        }
    }

    /// Lazily creates the four child tiles (NW/NE/SW/SE), mirroring the
    /// CesiumJS child getters: `NW = (2x, 2y)`, `NE = (2x+1, 2y)`,
    /// `SW = (2x, 2y+1)`, `SE = (2x+1, 2y+1)` at `level + 1`.
    pub fn ensure_children(&mut self, tiling_scheme: &dyn TilingScheme) {
        if !self.children.is_empty() {
            return;
        }
        let child_level = self.level + 1;
        let child_error = self.geometric_error * 0.5;
        let coords = [
            (self.x * 2, self.y * 2),         // NW
            (self.x * 2 + 1, self.y * 2),     // NE
            (self.x * 2, self.y * 2 + 1),     // SW
            (self.x * 2 + 1, self.y * 2 + 1), // SE
        ];
        self.children = coords
            .into_iter()
            .map(|(cx, cy)| {
                Self::from_tiling_scheme(tiling_scheme, child_level, cx, cy, child_error)
            })
            .collect();
    }

    /// Recomputes the bounding sphere from the tile rectangle on the ellipsoid.
    ///
    /// DEVIATION (B4-2): CesiumJS derives a tight `BoundingRegion` from the
    /// terrain/imagery availability; the pure-logic traversal needs only a
    /// conservative sphere, so we bound the rectangle corners and center.
    pub fn update_bounding_sphere(&mut self, ellipsoid: &Ellipsoid) {
        let mut center_carto = Rectangle::center(&self.rectangle);
        center_carto.height = 0.0;
        let mut center = Cartesian3::default();
        ellipsoid.cartographic_to_cartesian(&center_carto, &mut center);

        let corners = [
            Rectangle::southwest(&self.rectangle),
            Cartographic {
                longitude: self.rectangle.east,
                latitude: self.rectangle.south,
                height: 0.0,
            },
            Cartographic {
                longitude: self.rectangle.west,
                latitude: self.rectangle.north,
                height: 0.0,
            },
            Rectangle::northeast(&self.rectangle),
        ];
        let mut radius = 0.0;
        for corner in &corners {
            let mut point = Cartesian3::default();
            ellipsoid.cartographic_to_cartesian(corner, &mut point);
            let d = Cartesian3::distance(&point, &center);
            if d > radius {
                radius = d;
            }
        }
        self.bounding_sphere.center = center;
        self.bounding_sphere.radius = radius;
    }

    /// Returns a shallow copy of the tile (without the child subtree), used to
    /// collect render-list entries without duplicating the whole tree.
    pub fn snapshot(&self) -> QuadtreeTile {
        QuadtreeTile {
            x: self.x,
            y: self.y,
            level: self.level,
            rectangle: self.rectangle,
            bounding_sphere: self.bounding_sphere.clone(),
            load_state: self.load_state,
            screen_space_error: self.screen_space_error,
            camera_distance: self.camera_distance,
            was_rendered: self.was_rendered,
            loading_descendant_count: self.loading_descendant_count,
            renderable: self.renderable,
            pick_bounding_sphere: self.pick_bounding_sphere.clone(),
            geometric_error: self.geometric_error,
            children: Vec::new(),
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
