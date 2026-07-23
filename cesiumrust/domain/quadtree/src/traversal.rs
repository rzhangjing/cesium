//! Quadtree traversal for terrain tile selection.
//!
//! Maps to CesiumJS `Scene/QuadtreePrimitive.js`:
//! - Quadtree tile traversal
//! - Screen-space error (SSE) based LOD selection
//! - Tile refinement decisions

use cesium_geospatial::bounding::BoundingSphere;
use glam::DVec3;

/// A tile in the quadtree.
#[derive(Debug, Clone, PartialEq)]
pub struct QuadtreeTile {
    /// Tile X coordinate.
    pub x: u32,
    /// Tile Y coordinate.
    pub y: u32,
    /// Tile level (zoom).
    pub level: u32,
    /// Bounding sphere of the tile.
    pub bounding_sphere: BoundingSphere,
    /// Geometric error of the tile (meters).
    pub geometric_error: f64,
    /// Whether the tile has renderable content.
    pub has_content: bool,
    /// Whether the tile is refineable (has children).
    pub refineable: bool,
    /// Tile state.
    pub state: TileState,
}

impl QuadtreeTile {
    /// Creates a new quadtree tile.
    pub fn new(
        x: u32,
        y: u32,
        level: u32,
        bounding_sphere: BoundingSphere,
        geometric_error: f64,
    ) -> Self {
        Self {
            x,
            y,
            level,
            bounding_sphere,
            geometric_error,
            has_content: true,
            refineable: true,
            state: TileState::Unloaded,
        }
    }

    /// Computes the screen-space error for this tile.
    ///
    /// # Arguments
    /// * `camera_position` - Camera position in world space
    /// * `viewport_height` - Viewport height in pixels
    /// * `fov_y` - Vertical field of view (radians)
    ///
    /// # Returns
    /// Screen-space error in pixels
    pub fn compute_screen_space_error(
        &self,
        camera_position: DVec3,
        viewport_height: f64,
        fov_y: f64,
    ) -> f64 {
        let distance = (camera_position - self.bounding_sphere.center).length()
            - self.bounding_sphere.radius;
        let distance = distance.max(1.0); // Avoid division by zero

        // SSE = (geometric_error * viewport_height) / (distance * 2 * tan(fov_y / 2))
        let sse_denominator = 2.0 * (fov_y / 2.0).tan();
        (self.geometric_error * viewport_height) / (distance * sse_denominator)
    }

    /// Returns the children tile coordinates.
    pub fn children_coords(&self) -> [(u32, u32); 4] {
        let child_x = self.x * 2;
        let child_y = self.y * 2;
        [
            (child_x, child_y),
            (child_x + 1, child_y),
            (child_x, child_y + 1),
            (child_x + 1, child_y + 1),
        ]
    }
}

/// Tile loading/rendering state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileState {
    /// Tile is not loaded.
    #[default]
    Unloaded,
    /// Tile is being loaded.
    Loading,
    /// Tile is loaded and ready to render.
    Loaded,
    /// Tile is currently being rendered.
    Rendered,
    /// Tile is refined (children are rendered instead).
    Refined,
}

/// Quadtree traversal configuration.
#[derive(Debug, Clone)]
pub struct QuadtreeConfig {
    /// Maximum screen-space error threshold (pixels).
    pub maximum_screen_space_error: f64,
    /// Maximum tile level.
    pub maximum_level: u32,
    /// Minimum tile level.
    pub minimum_level: u32,
    /// Whether to enable fog culling.
    pub fog_culling: bool,
    /// Fog density for culling.
    pub fog_density: f64,
}

impl Default for QuadtreeConfig {
    fn default() -> Self {
        Self {
            maximum_screen_space_error: 2.0,
            maximum_level: 22,
            minimum_level: 0,
            fog_culling: false,
            fog_density: 0.0002,
        }
    }
}

/// Result of quadtree traversal.
#[derive(Debug, Clone, Default)]
pub struct TraversalResult {
    /// Tiles to render.
    pub tiles_to_render: Vec<QuadtreeTile>,
    /// Tiles to load.
    pub tiles_to_load: Vec<QuadtreeTile>,
    /// Total tiles visited.
    pub tiles_visited: u32,
    /// Maximum depth reached.
    pub max_depth: u32,
}

/// A quadtree primitive for terrain tile management.
///
/// Maps to CesiumJS `Scene/QuadtreePrimitive.js`
#[derive(Debug)]
pub struct QuadtreePrimitive {
    /// Root tiles (typically 2 for WGS84: western and eastern hemispheres).
    pub root_tiles: Vec<QuadtreeTile>,
    /// Traversal configuration.
    pub config: QuadtreeConfig,
}

impl QuadtreePrimitive {
    /// Creates a new quadtree primitive.
    pub fn new(root_tiles: Vec<QuadtreeTile>, config: QuadtreeConfig) -> Self {
        Self { root_tiles, config }
    }

    /// Traverses the quadtree and selects tiles for rendering.
    ///
    /// # Arguments
    /// * `camera_position` - Camera position in world space
    /// * `viewport_height` - Viewport height in pixels
    /// * `fov_y` - Vertical field of view (radians)
    /// * `tile_provider` - Function to get child tiles
    pub fn traverse<F>(
        &self,
        camera_position: DVec3,
        viewport_height: f64,
        fov_y: f64,
        tile_provider: &F,
    ) -> TraversalResult
    where
        F: Fn(u32, u32, u32) -> Option<QuadtreeTile>,
    {
        let mut result = TraversalResult::default();

        for root in &self.root_tiles {
            self.visit_tile(
                root,
                camera_position,
                viewport_height,
                fov_y,
                tile_provider,
                &mut result,
            );
        }

        result
    }

    fn visit_tile<F>(
        &self,
        tile: &QuadtreeTile,
        camera_position: DVec3,
        viewport_height: f64,
        fov_y: f64,
        tile_provider: &F,
        result: &mut TraversalResult,
    ) where
        F: Fn(u32, u32, u32) -> Option<QuadtreeTile>,
    {
        result.tiles_visited += 1;
        result.max_depth = result.max_depth.max(tile.level);

        // Check if tile is visible (frustum culling would go here)
        // For now, assume all tiles are visible

        // Compute screen-space error
        let sse = tile.compute_screen_space_error(camera_position, viewport_height, fov_y);

        // Check if we should refine this tile
        let should_refine = tile.refineable
            && tile.level < self.config.maximum_level
            && sse > self.config.maximum_screen_space_error;

        if should_refine {
            // Try to load and visit children
            let children_coords = tile.children_coords();
            let mut all_children_loaded = true;

            for (cx, cy) in children_coords {
                if let Some(child) = tile_provider(cx, cy, tile.level + 1) {
                    if child.state == TileState::Loaded || child.state == TileState::Rendered {
                        self.visit_tile(
                            &child,
                            camera_position,
                            viewport_height,
                            fov_y,
                            tile_provider,
                            result,
                        );
                    } else {
                        all_children_loaded = false;
                        result.tiles_to_load.push(child);
                    }
                } else {
                    all_children_loaded = false;
                }
            }

            // If not all children are loaded, render this tile as fallback
            if !all_children_loaded && tile.has_content {
                result.tiles_to_render.push(tile.clone());
            }
        } else if tile.has_content {
            // Render this tile
            result.tiles_to_render.push(tile.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tile(x: u32, y: u32, level: u32, geometric_error: f64) -> QuadtreeTile {
        QuadtreeTile::new(
            x,
            y,
            level,
            BoundingSphere::new(DVec3::ZERO, 1000000.0),
            geometric_error,
        )
    }

    #[test]
    fn test_quadtree_tile_creation() {
        let tile = create_test_tile(0, 0, 0, 100000.0);
        assert_eq!(tile.x, 0);
        assert_eq!(tile.y, 0);
        assert_eq!(tile.level, 0);
        assert_eq!(tile.geometric_error, 100000.0);
        assert_eq!(tile.state, TileState::Unloaded);
    }

    #[test]
    fn test_screen_space_error() {
        let tile = create_test_tile(0, 0, 0, 10000.0);
        let camera_position = DVec3::new(0.0, 0.0, 2000000.0);
        let viewport_height = 1080.0;
        let fov_y = std::f64::consts::FRAC_PI_4; // 45 degrees

        let sse = tile.compute_screen_space_error(camera_position, viewport_height, fov_y);

        // SSE should be positive and reasonable
        assert!(sse > 0.0);
        assert!(sse < 10000.0); // Not absurdly large
    }

    #[test]
    fn test_sse_decreases_with_distance() {
        let tile = create_test_tile(0, 0, 0, 10000.0);
        let viewport_height = 1080.0;
        let fov_y = std::f64::consts::FRAC_PI_4;

        let sse_near = tile.compute_screen_space_error(
            DVec3::new(0.0, 0.0, 1500000.0),
            viewport_height,
            fov_y,
        );
        let sse_far = tile.compute_screen_space_error(
            DVec3::new(0.0, 0.0, 5000000.0),
            viewport_height,
            fov_y,
        );

        assert!(sse_near > sse_far);
    }

    #[test]
    fn test_children_coords() {
        let tile = create_test_tile(1, 2, 3, 10000.0);
        let children = tile.children_coords();

        assert_eq!(children[0], (2, 4));
        assert_eq!(children[1], (3, 4));
        assert_eq!(children[2], (2, 5));
        assert_eq!(children[3], (3, 5));
    }

    #[test]
    fn test_quadtree_config_default() {
        let config = QuadtreeConfig::default();
        assert_eq!(config.maximum_screen_space_error, 2.0);
        assert_eq!(config.maximum_level, 22);
        assert_eq!(config.minimum_level, 0);
        assert!(!config.fog_culling);
    }

    #[test]
    fn test_traversal_single_tile() {
        let root = create_test_tile(0, 0, 0, 100.0); // Low geometric error = low SSE
        let primitive = QuadtreePrimitive::new(
            vec![root],
            QuadtreeConfig {
                maximum_screen_space_error: 2.0,
                maximum_level: 10,
                ..Default::default()
            },
        );

        let camera_position = DVec3::new(0.0, 0.0, 10000000.0); // Far away
        let result = primitive.traverse(camera_position, 1080.0, std::f64::consts::FRAC_PI_4, &|_, _, _| None);

        // Should render the root tile (SSE below threshold)
        assert_eq!(result.tiles_to_render.len(), 1);
        assert_eq!(result.tiles_visited, 1);
    }

    #[test]
    fn test_traversal_with_refinement() {
        let root = create_test_tile(0, 0, 0, 1000000.0); // High geometric error = high SSE
        let primitive = QuadtreePrimitive::new(
            vec![root],
            QuadtreeConfig {
                maximum_screen_space_error: 2.0,
                maximum_level: 10,
                ..Default::default()
            },
        );

        let camera_position = DVec3::new(0.0, 0.0, 2000000.0); // Close

        // Provide children (they will be Unloaded, so added to load queue)
        let result = primitive.traverse(camera_position, 1080.0, std::f64::consts::FRAC_PI_4, &|x, y, level| {
            if level <= 2 {
                Some(create_test_tile(x, y, level, 1000000.0 / (level as f64 + 1.0)))
            } else {
                None
            }
        });

        // Root tile should be rendered as fallback (children not loaded)
        assert_eq!(result.tiles_to_render.len(), 1);
        // Children should be queued for loading
        assert_eq!(result.tiles_to_load.len(), 4);
        assert_eq!(result.tiles_visited, 1);
    }

    #[test]
    fn test_tile_state_default() {
        assert_eq!(TileState::default(), TileState::Unloaded);
    }
}
