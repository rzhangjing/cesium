//! Voxel LOD traversal system.
//!
//! Maps to CesiumJS `Scene/VoxelTraversal.js`.
//! Implements screen-space-error based LOD traversal for voxel grids.

use crate::shape::{OrientedBoundingBox, VoxelShape, VoxelShapeType};

/// A spatial node in the voxel octree.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialNode {
    /// Level in the octree (0 = root).
    pub level: u32,
    /// X coordinate at this level.
    pub x: u32,
    /// Y coordinate at this level.
    pub y: u32,
    /// Z coordinate at this level.
    pub z: u32,
    /// Tile dimensions (number of samples per axis, before padding).
    pub dimensions: [u32; 3],
}

impl SpatialNode {
    /// Create a new spatial node.
    pub fn new(level: u32, x: u32, y: u32, z: u32, dimensions: [u32; 3]) -> Self {
        Self { level, x, y, z, dimensions }
    }

    /// Create the root node.
    pub fn root(dimensions: [u32; 3]) -> Self {
        Self::new(0, 0, 0, 0, dimensions)
    }

    /// Get the number of children (always 8 for octree).
    pub fn child_count(&self) -> u32 {
        8
    }

    /// Get a child node by index (0-7).
    pub fn child(&self, index: u32) -> Self {
        let child_level = self.level + 1;
        let child_x = self.x * 2 + (index & 1);
        let child_y = self.y * 2 + ((index >> 1) & 1);
        let child_z = self.z * 2 + ((index >> 2) & 1);
        Self::new(child_level, child_x, child_y, child_z, self.dimensions)
    }

    /// Get the parent node, or None if this is root.
    pub fn parent(&self) -> Option<Self> {
        if self.level == 0 {
            None
        } else {
            Some(Self::new(
                self.level - 1,
                self.x / 2,
                self.y / 2,
                self.z / 2,
                self.dimensions,
            ))
        }
    }

    /// Get the total number of samples in this node (including padding).
    pub fn sample_count(&self, padding: u32) -> u32 {
        let dx = self.dimensions[0] + padding * 2;
        let dy = self.dimensions[1] + padding * 2;
        let dz = self.dimensions[2] + padding * 2;
        dx * dy * dz
    }

    /// Get the Morton index for this node.
    pub fn morton_index(&self) -> u64 {
        morton_encode(self.x as u64, self.y as u64, self.z as u64)
    }
}

/// Result of a traversal operation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TraversalResult {
    /// Nodes selected for rendering (meet SSE threshold).
    pub render_nodes: Vec<SpatialNode>,
    /// Nodes that need more detail (children should be loaded).
    pub refine_nodes: Vec<SpatialNode>,
    /// Total number of nodes visited.
    pub nodes_visited: u32,
    /// Maximum depth reached.
    pub max_depth: u32,
}

/// Voxel traversal configuration.
#[derive(Debug, Clone)]
pub struct VoxelTraversalConfig {
    /// Shape type of the voxel grid.
    pub shape_type: VoxelShapeType,
    /// Screen-space error threshold in pixels.
    pub screen_space_error: f64,
    /// Maximum number of levels to traverse.
    pub max_level: u32,
    /// Tile dimensions (samples per axis).
    pub tile_dimensions: [u32; 3],
    /// Padding around each tile.
    pub padding: u32,
    /// Whether to skip levels that aren't available.
    pub skip_level_of_detail: bool,
    /// Factor for skip LOD (how many levels to skip).
    pub skip_levels: u32,
}

impl Default for VoxelTraversalConfig {
    fn default() -> Self {
        Self {
            shape_type: VoxelShapeType::Box,
            screen_space_error: 16.0,
            max_level: 10,
            tile_dimensions: [8, 8, 8],
            padding: 1,
            skip_level_of_detail: false,
            skip_levels: 1,
        }
    }
}

/// Voxel LOD traversal engine.
///
/// Performs octree traversal with screen-space-error based refinement.
#[derive(Debug, Clone)]
pub struct VoxelTraversal {
    /// Traversal configuration.
    pub config: VoxelTraversalConfig,
    /// Whether data is available at each level (level -> available).
    level_availability: Vec<bool>,
}

impl Default for VoxelTraversal {
    fn default() -> Self {
        Self {
            config: VoxelTraversalConfig::default(),
            level_availability: vec![true; 11],
        }
    }
}

impl VoxelTraversal {
    /// Create a new traversal with the given configuration.
    pub fn new(config: VoxelTraversalConfig) -> Self {
        let max_levels = (config.max_level + 1) as usize;
        Self {
            config,
            level_availability: vec![true; max_levels],
        }
    }

    /// Set availability for a specific level.
    pub fn set_level_available(&mut self, level: u32, available: bool) {
        if (level as usize) < self.level_availability.len() {
            self.level_availability[level as usize] = available;
        }
    }

    /// Check if a level has data available.
    pub fn is_level_available(&self, level: u32) -> bool {
        if (level as usize) < self.level_availability.len() {
            self.level_availability[level as usize]
        } else {
            false
        }
    }

    /// Compute the screen-space error for a node.
    ///
    /// SSE = (geometric_error * viewport_height) / (distance * 2 * tan(fov/2))
    pub fn compute_screen_space_error(
        &self,
        node: &SpatialNode,
        shape: &dyn VoxelShape,
        camera_position: glam::DVec3,
        viewport_height: f64,
        fov_y: f64,
    ) -> f64 {
        let obb = shape.compute_obb_for_tile(node.level, node.x, node.y, node.z);
        let distance = obb.distance_to(camera_position).max(1e-7);

        // Geometric error decreases with level
        let geometric_error = self.compute_geometric_error(node, shape);

        let sse_denominator = 2.0 * (fov_y * 0.5).tan();
        (geometric_error * viewport_height) / (distance * sse_denominator)
    }

    /// Compute the geometric error for a node (size of a voxel cell).
    fn compute_geometric_error(&self, node: &SpatialNode, shape: &dyn VoxelShape) -> f64 {
        let obb = shape.compute_obb_for_tile(node.level, node.x, node.y, node.z);
        let size = obb.bounding_sphere_radius();
        // Geometric error is roughly the size of one sample
        let max_dim = self.config.tile_dimensions.iter().max().copied().unwrap_or(8) as f64;
        size / max_dim
    }

    /// Perform traversal and return selected nodes.
    pub fn traverse(
        &self,
        shape: &dyn VoxelShape,
        camera_position: glam::DVec3,
        viewport_height: f64,
        fov_y: f64,
    ) -> TraversalResult {
        let mut result = TraversalResult::default();
        let root = SpatialNode::root(self.config.tile_dimensions);
        self.traverse_node(
            &root,
            shape,
            camera_position,
            viewport_height,
            fov_y,
            &mut result,
        );
        result
    }

    /// Recursively traverse a node.
    fn traverse_node(
        &self,
        node: &SpatialNode,
        shape: &dyn VoxelShape,
        camera_position: glam::DVec3,
        viewport_height: f64,
        fov_y: f64,
        result: &mut TraversalResult,
    ) {
        result.nodes_visited += 1;
        result.max_depth = result.max_depth.max(node.level);

        // Check if we've reached max level
        if node.level >= self.config.max_level {
            result.render_nodes.push(node.clone());
            return;
        }

        // Check if data is available at this level
        if !self.is_level_available(node.level) {
            // Try children if skip LOD is enabled
            if self.config.skip_level_of_detail && node.level + self.config.skip_levels <= self.config.max_level {
                for i in 0..8 {
                    let child = node.child(i);
                    self.traverse_node(
                        &child,
                        shape,
                        camera_position,
                        viewport_height,
                        fov_y,
                        result,
                    );
                }
            }
            return;
        }

        // Compute SSE
        let sse = self.compute_screen_space_error(
            node,
            shape,
            camera_position,
            viewport_height,
            fov_y,
        );

        if sse <= self.config.screen_space_error {
            // Node meets quality threshold, render it
            result.render_nodes.push(node.clone());
        } else {
            // Need more detail, refine
            result.refine_nodes.push(node.clone());
            for i in 0..8 {
                let child = node.child(i);
                self.traverse_node(
                    &child,
                    shape,
                    camera_position,
                    viewport_height,
                    fov_y,
                    result,
                );
            }
        }
    }

    /// Compute the total number of tiles at a given level.
    pub fn tiles_at_level(level: u32) -> u64 {
        let tiles_per_axis = 2u64.pow(level);
        tiles_per_axis * tiles_per_axis * tiles_per_axis
    }

    /// Get the OBB for a specific tile.
    pub fn tile_obb(
        &self,
        shape: &dyn VoxelShape,
        level: u32,
        x: u32,
        y: u32,
        z: u32,
    ) -> OrientedBoundingBox {
        shape.compute_obb_for_tile(level, x, y, z)
    }
}

/// Encode 3D coordinates into a Morton code (Z-order curve).
fn morton_encode(x: u64, y: u64, z: u64) -> u64 {
    let mut result = 0u64;
    for i in 0..21 {
        result |= ((x >> i) & 1) << (3 * i);
        result |= ((y >> i) & 1) << (3 * i + 1);
        result |= ((z >> i) & 1) << (3 * i + 2);
    }
    result
}

/// Decode a Morton code into 3D coordinates.
pub fn morton_decode(code: u64) -> (u64, u64, u64) {
    let mut x = 0u64;
    let mut y = 0u64;
    let mut z = 0u64;
    for i in 0..21 {
        x |= ((code >> (3 * i)) & 1) << i;
        y |= ((code >> (3 * i + 1)) & 1) << i;
        z |= ((code >> (3 * i + 2)) & 1) << i;
    }
    (x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::box_shape::VoxelBoxShape;

    #[test]
    fn test_spatial_node_root() {
        let root = SpatialNode::root([8, 8, 8]);
        assert_eq!(root.level, 0);
        assert_eq!(root.x, 0);
        assert_eq!(root.y, 0);
        assert_eq!(root.z, 0);
        assert_eq!(root.dimensions, [8, 8, 8]);
    }

    #[test]
    fn test_spatial_node_children() {
        let root = SpatialNode::root([8, 8, 8]);
        assert_eq!(root.child_count(), 8);

        let child0 = root.child(0);
        assert_eq!(child0.level, 1);
        assert_eq!((child0.x, child0.y, child0.z), (0, 0, 0));

        let child7 = root.child(7);
        assert_eq!(child7.level, 1);
        assert_eq!((child7.x, child7.y, child7.z), (1, 1, 1));

        let child5 = root.child(5);
        assert_eq!((child5.x, child5.y, child5.z), (1, 0, 1));
    }

    #[test]
    fn test_spatial_node_parent() {
        let root = SpatialNode::root([8, 8, 8]);
        assert!(root.parent().is_none());

        let child = root.child(3);
        let parent = child.parent().unwrap();
        assert_eq!(parent.level, 0);
        assert_eq!((parent.x, parent.y, parent.z), (0, 0, 0));
    }

    #[test]
    fn test_spatial_node_sample_count() {
        let node = SpatialNode::root([8, 8, 8]);
        // With padding=1: (8+2)^3 = 1000
        assert_eq!(node.sample_count(1), 1000);
        // With padding=0: 8^3 = 512
        assert_eq!(node.sample_count(0), 512);
    }

    #[test]
    fn test_morton_encode_decode() {
        let code = morton_encode(1, 2, 3);
        let (x, y, z) = morton_decode(code);
        assert_eq!((x, y, z), (1, 2, 3));

        let code2 = morton_encode(0, 0, 0);
        assert_eq!(code2, 0);

        let code3 = morton_encode(7, 7, 7);
        let (x3, y3, z3) = morton_decode(code3);
        assert_eq!((x3, y3, z3), (7, 7, 7));
    }

    #[test]
    fn test_traversal_config_default() {
        let config = VoxelTraversalConfig::default();
        assert_eq!(config.shape_type, VoxelShapeType::Box);
        assert_eq!(config.screen_space_error, 16.0);
        assert_eq!(config.max_level, 10);
        assert_eq!(config.tile_dimensions, [8, 8, 8]);
        assert_eq!(config.padding, 1);
    }

    #[test]
    fn test_traversal_basic() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            glam::DMat4::IDENTITY,
            crate::box_shape::BOX_DEFAULT_MIN_BOUNDS,
            crate::box_shape::BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );

        let config = VoxelTraversalConfig {
            max_level: 2,
            screen_space_error: 1000.0, // High threshold = less refinement
            ..Default::default()
        };
        let traversal = VoxelTraversal::new(config);

        let result = traversal.traverse(
            &shape,
            glam::DVec3::new(0.0, 0.0, 10.0),
            1080.0,
            std::f64::consts::FRAC_PI_3,
        );

        assert!(result.nodes_visited > 0);
        assert!(!result.render_nodes.is_empty());
    }

    #[test]
    fn test_traversal_max_level() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            glam::DMat4::IDENTITY,
            crate::box_shape::BOX_DEFAULT_MIN_BOUNDS,
            crate::box_shape::BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );

        let config = VoxelTraversalConfig {
            max_level: 0,
            screen_space_error: 0.001, // Very low threshold = always refine
            ..Default::default()
        };
        let traversal = VoxelTraversal::new(config);

        let result = traversal.traverse(
            &shape,
            glam::DVec3::new(0.0, 0.0, 100.0),
            1080.0,
            std::f64::consts::FRAC_PI_3,
        );

        // At max_level=0, root should be rendered directly
        assert_eq!(result.render_nodes.len(), 1);
        assert_eq!(result.max_depth, 0);
    }

    #[test]
    fn test_traversal_level_availability() {
        let mut traversal = VoxelTraversal::default();
        assert!(traversal.is_level_available(0));
        assert!(traversal.is_level_available(5));

        traversal.set_level_available(3, false);
        assert!(!traversal.is_level_available(3));
        assert!(traversal.is_level_available(2));
    }

    #[test]
    fn test_tiles_at_level() {
        assert_eq!(VoxelTraversal::tiles_at_level(0), 1);
        assert_eq!(VoxelTraversal::tiles_at_level(1), 8);
        assert_eq!(VoxelTraversal::tiles_at_level(2), 64);
        assert_eq!(VoxelTraversal::tiles_at_level(3), 512);
    }

    #[test]
    fn test_screen_space_error_computation() {
        let mut shape = VoxelBoxShape::new();
        shape.update(
            glam::DMat4::IDENTITY,
            crate::box_shape::BOX_DEFAULT_MIN_BOUNDS,
            crate::box_shape::BOX_DEFAULT_MAX_BOUNDS,
            None,
            None,
        );

        let traversal = VoxelTraversal::default();
        let root = SpatialNode::root([8, 8, 8]);

        // Close camera = high SSE
        let sse_close = traversal.compute_screen_space_error(
            &root,
            &shape,
            glam::DVec3::new(0.0, 0.0, 2.0),
            1080.0,
            std::f64::consts::FRAC_PI_3,
        );

        // Far camera = low SSE
        let sse_far = traversal.compute_screen_space_error(
            &root,
            &shape,
            glam::DVec3::new(0.0, 0.0, 1000.0),
            1080.0,
            std::f64::consts::FRAC_PI_3,
        );

        assert!(sse_close > sse_far);
    }
}
