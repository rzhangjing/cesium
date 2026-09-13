//! Level-of-Detail (LOD) selection for 3D Tiles.
//!
//! Implements screen-space error (SSE) computation and tile traversal
//! for selecting which tiles to render.
//!
//! Maps to CesiumJS `Scene/Cesium3DTilesetTraversal.js`

use crate::tile::{Tile, TileRefine};
use cesium_geospatial::ellipsoid::Ellipsoid;
use glam::DVec3;

/// Camera state for LOD computation.
#[derive(Debug, Clone)]
pub struct CameraState {
    /// Camera position in ECEF coordinates.
    pub position: DVec3,

    /// Camera view direction (normalized).
    pub direction: DVec3,

    /// Camera up direction (normalized).
    pub up: DVec3,

    /// Vertical field of view in radians.
    pub fov_y: f64,

    /// Viewport height in pixels.
    pub viewport_height: f64,
}

impl CameraState {
    /// Creates a new camera state.
    pub fn new(position: DVec3, direction: DVec3, up: DVec3, fov_y: f64, viewport_height: f64) -> Self {
        Self {
            position,
            direction: direction.normalize(),
            up: up.normalize(),
            fov_y,
            viewport_height,
        }
    }

    /// Computes the screen space error for a given geometric error and distance.
    ///
    /// SSE = (geometricError * viewportHeight) / (distance * 2 * tan(fovY / 2))
    ///
    /// Maps to CesiumJS `Cesium3DTileset._computeScreenSpaceError`
    pub fn compute_screen_space_error(&self, geometric_error: f64, distance: f64) -> f64 {
        if distance <= 0.0 {
            return f64::MAX;
        }

        let sse_denominator = 2.0 * (self.fov_y / 2.0).tan();
        (geometric_error * self.viewport_height) / (distance * sse_denominator)
    }
}

/// Result of tile selection for a single tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileSelectionResult {
    /// Tile should be rendered (content loaded).
    Render,
    /// Tile should be refined (children should be considered).
    Refine,
    /// Tile is culled (not visible).
    Culled,
}

/// A selected tile with its selection result.
#[derive(Debug, Clone)]
pub struct SelectedTile {
    /// Path to the tile in the tree (indices from root).
    pub path: Vec<usize>,

    /// The selection result.
    pub result: TileSelectionResult,

    /// Screen space error for this tile.
    pub screen_space_error: f64,

    /// Distance from camera to tile.
    pub distance_to_camera: f64,
}

/// LOD selection context.
#[derive(Debug, Clone)]
pub struct LodSelectionContext {
    /// Maximum screen space error threshold.
    pub maximum_screen_space_error: f64,

    /// Whether to cull tiles outside the view frustum.
    pub cull_with_frustum: bool,

    /// Whether to skip tiles that have already been refined.
    pub skip_level_of_detail: bool,
}

impl Default for LodSelectionContext {
    fn default() -> Self {
        Self {
            maximum_screen_space_error: 16.0,
            cull_with_frustum: true,
            skip_level_of_detail: false,
        }
    }
}

/// Computes the distance from the camera to a tile's bounding volume.
pub fn compute_distance_to_tile(
    camera: &CameraState,
    tile: &Tile,
    ellipsoid: &Ellipsoid,
) -> f64 {
    tile.bounding_volume.distance_to(camera.position, ellipsoid)
}

/// Computes the screen space error for a tile.
pub fn compute_tile_sse(
    camera: &CameraState,
    tile: &Tile,
    ellipsoid: &Ellipsoid,
) -> f64 {
    let distance = compute_distance_to_tile(camera, tile, ellipsoid);
    camera.compute_screen_space_error(tile.geometric_error, distance)
}

/// Determines if a tile should be refined based on its screen space error.
///
/// A tile should be refined if its SSE exceeds the maximum threshold
/// and it has children.
pub fn should_refine_tile(
    sse: f64,
    max_sse: f64,
    has_children: bool,
) -> bool {
    has_children && sse > max_sse
}

/// Selects tiles for rendering using a simple traversal algorithm.
///
/// This implements a basic top-down traversal that:
/// 1. Computes SSE for each tile
/// 2. If SSE <= threshold, selects the tile for rendering
/// 3. If SSE > threshold and tile has children, refines to children
///
/// # Arguments
/// * `root` - The root tile of the tileset
/// * `camera` - The camera state for SSE computation
/// * `context` - The LOD selection context
/// * `ellipsoid` - The ellipsoid for coordinate conversions
///
/// # Returns
/// A list of selected tiles with their selection results
pub fn select_tiles(
    root: &Tile,
    camera: &CameraState,
    context: &LodSelectionContext,
    ellipsoid: &Ellipsoid,
) -> Vec<SelectedTile> {
    let mut selected = Vec::new();
    select_tiles_recursive(
        root,
        camera,
        context,
        ellipsoid,
        TileRefine::Replace,
        &[],
        &mut selected,
    );
    selected
}

/// Recursive tile selection helper.
#[allow(clippy::too_many_arguments)]
fn select_tiles_recursive(
    tile: &Tile,
    camera: &CameraState,
    context: &LodSelectionContext,
    ellipsoid: &Ellipsoid,
    parent_refine: TileRefine,
    path: &[usize],
    selected: &mut Vec<SelectedTile>,
) {
    let distance = compute_distance_to_tile(camera, tile, ellipsoid);
    let sse = camera.compute_screen_space_error(tile.geometric_error, distance);

    let refine_mode = tile.effective_refine(parent_refine);
    let has_children = !tile.children.is_empty();
    let should_refine = should_refine_tile(sse, context.maximum_screen_space_error, has_children);

    if should_refine {
        // Refine: traverse children
        for (i, child) in tile.children.iter().enumerate() {
            let mut child_path = path.to_vec();
            child_path.push(i);
            select_tiles_recursive(
                child,
                camera,
                context,
                ellipsoid,
                refine_mode,
                &child_path,
                selected,
            );
        }

        // For ADD refinement, also render the parent
        if refine_mode == TileRefine::Add && tile.has_content() {
            selected.push(SelectedTile {
                path: path.to_vec(),
                result: TileSelectionResult::Render,
                screen_space_error: sse,
                distance_to_camera: distance,
            });
        }
    } else {
        // Render this tile
        if tile.has_content() {
            selected.push(SelectedTile {
                path: path.to_vec(),
                result: TileSelectionResult::Render,
                screen_space_error: sse,
                distance_to_camera: distance,
            });
        } else if has_children {
            // Empty tile with children: refine anyway
            for (i, child) in tile.children.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(i);
                select_tiles_recursive(
                    child,
                    camera,
                    context,
                    ellipsoid,
                    refine_mode,
                    &child_path,
                    selected,
                );
            }
        }
    }
}

/// Gets a tile by its path in the tree.
pub fn get_tile_by_path<'a>(root: &'a Tile, path: &[usize]) -> Option<&'a Tile> {
    let mut current = root;
    for &index in path {
        current = current.children.get(index)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounding_volume::BoundingVolume;
    use crate::tile::TileContent;

    fn create_test_camera() -> CameraState {
        CameraState::new(
            DVec3::new(0.0, 0.0, 1000.0),
            DVec3::new(0.0, 0.0, -1.0),
            DVec3::new(0.0, 1.0, 0.0),
            std::f64::consts::FRAC_PI_4, // 45 degrees
            1080.0,
        )
    }

    fn create_test_tile(geometric_error: f64, has_children: bool) -> Tile {
        let children = if has_children {
            vec![
                Tile {
                    bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 50.0),
                    geometric_error: geometric_error / 2.0,
                    refine: None,
                    transform: None,
                    content: Some(TileContent {
                        uri: "child.b3dm".to_string(),
                        bounding_volume: None,
                        group: None,
                    }),
                    contents: None,
                    children: vec![],
                    viewer_request_volume: None,
                    extras: None,
                },
            ]
        } else {
            vec![]
        };

        Tile {
            bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 100.0),
            geometric_error,
            refine: Some(TileRefine::Replace),
            transform: None,
            content: Some(TileContent {
                uri: "parent.b3dm".to_string(),
                bounding_volume: None,
                group: None,
            }),
            contents: None,
            children,
            viewer_request_volume: None,
            extras: None,
        }
    }

    #[test]
    fn test_screen_space_error_computation() {
        let camera = create_test_camera();

        // SSE = (geometricError * viewportHeight) / (distance * 2 * tan(fovY / 2))
        // SSE = (100 * 1080) / (1000 * 2 * tan(22.5°))
        let sse = camera.compute_screen_space_error(100.0, 1000.0);
        let expected = (100.0 * 1080.0) / (1000.0 * 2.0 * (std::f64::consts::FRAC_PI_4 / 2.0).tan());
        assert!((sse - expected).abs() < 1e-10);
    }

    #[test]
    fn test_sse_increases_with_geometric_error() {
        let camera = create_test_camera();

        let sse_small = camera.compute_screen_space_error(10.0, 1000.0);
        let sse_large = camera.compute_screen_space_error(100.0, 1000.0);

        assert!(sse_large > sse_small);
    }

    #[test]
    fn test_sse_increases_with_proximity() {
        let camera = create_test_camera();

        let sse_far = camera.compute_screen_space_error(100.0, 10000.0);
        let sse_near = camera.compute_screen_space_error(100.0, 1000.0);

        assert!(sse_near > sse_far);
    }

    #[test]
    fn test_should_refine_tile() {
        assert!(should_refine_tile(20.0, 16.0, true)); // SSE > threshold, has children
        assert!(!should_refine_tile(10.0, 16.0, true)); // SSE < threshold
        assert!(!should_refine_tile(20.0, 16.0, false)); // No children
    }

    #[test]
    fn test_select_tiles_no_refinement() {
        let root = create_test_tile(10.0, false); // Low error, no children
        let camera = create_test_camera();
        let context = LodSelectionContext::default();

        let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].result, TileSelectionResult::Render);
    }

    #[test]
    fn test_select_tiles_with_refinement() {
        let root = create_test_tile(1000.0, true); // High error, has children
        let camera = create_test_camera();
        let context = LodSelectionContext::default();

        let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);

        // Should refine to children
        assert!(selected.iter().any(|t| t.path == vec![0]));
    }

    #[test]
    fn test_get_tile_by_path() {
        let root = create_test_tile(100.0, true);

        // Root has empty path
        let root_tile = get_tile_by_path(&root, &[]);
        assert!(root_tile.is_some());
        assert_eq!(root_tile.unwrap().geometric_error, 100.0);

        // Child has path [0]
        let child_tile = get_tile_by_path(&root, &[0]);
        assert!(child_tile.is_some());
        assert_eq!(child_tile.unwrap().geometric_error, 50.0);

        // Invalid path
        let invalid = get_tile_by_path(&root, &[1]);
        assert!(invalid.is_none());
    }

    #[test]
    fn test_distance_to_tile() {
        let camera = create_test_camera();
        let tile = Tile {
            bounding_volume: BoundingVolume::from_sphere(DVec3::ZERO, 100.0),
            geometric_error: 10.0,
            refine: None,
            transform: None,
            content: None,
            contents: None,
            children: vec![],
            viewer_request_volume: None,
            extras: None,
        };

        let distance = compute_distance_to_tile(&camera, &tile, &Ellipsoid::WGS84);
        // Camera at (0, 0, 1000), sphere at origin with radius 100
        // Distance = 1000 - 100 = 900
        assert!((distance - 900.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_refinement_mode() {
        let mut root = create_test_tile(1000.0, true);
        root.refine = Some(TileRefine::Add);

        let camera = create_test_camera();
        let context = LodSelectionContext::default();

        let selected = select_tiles(&root, &camera, &context, &Ellipsoid::WGS84);

        // With ADD refinement, both parent and children should be rendered
        assert!(selected.iter().any(|t| t.path.is_empty())); // Parent
        assert!(selected.iter().any(|t| t.path == vec![0])); // Child
    }
}
