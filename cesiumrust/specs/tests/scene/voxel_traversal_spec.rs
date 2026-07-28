//! Voxel traversal tests ported from CesiumJS SpatialNodeSpec.js + VoxelShapeTypeSpec.js
//! Tests: SpatialNode children coordinates, VoxelShapeType bounds, traversal basics

use cesium_voxel::{
    SpatialNode, VoxelBoxShape, VoxelCylinderShape, VoxelEllipsoidShape, VoxelShape,
    VoxelShapeType, VoxelTraversal,
};
use glam::{DMat4, DVec3};

// ============================================================================
// SpatialNode: constructs (from SpatialNodeSpec.js)
// ============================================================================

#[test]
fn test_spatial_node_constructs() {
    // Ported from: SpatialNodeSpec "constructs"
    let node = SpatialNode::new(2, 1, 2, 3, [2, 3, 4]);
    assert_eq!(node.level, 2);
    assert_eq!(node.x, 1);
    assert_eq!(node.y, 2);
    assert_eq!(node.z, 3);
}

// ============================================================================
// SpatialNode: returns coordinates of child (from SpatialNodeSpec.js)
// ============================================================================

#[test]
fn test_spatial_node_children_coordinates() {
    // Ported from: SpatialNodeSpec "returns coordinates of child"
    let node = SpatialNode::new(2, 1, 2, 3, [2, 3, 4]);

    // CesiumJS expected child coordinates: [level, x, y, z]
    let expected: [(u32, u32, u32, u32); 8] = [
        (3, 2, 4, 6),
        (3, 3, 4, 6),
        (3, 2, 5, 6),
        (3, 3, 5, 6),
        (3, 2, 4, 7),
        (3, 3, 4, 7),
        (3, 2, 5, 7),
        (3, 3, 5, 7),
    ];

    for i in 0..8u32 {
        let child = node.child(i);
        let (exp_level, exp_x, exp_y, exp_z) = expected[i as usize];
        assert_eq!(child.level, exp_level, "child {} level", i);
        assert_eq!(child.x, exp_x, "child {} x", i);
        assert_eq!(child.y, exp_y, "child {} y", i);
        assert_eq!(child.z, exp_z, "child {} z", i);
    }
}

// ============================================================================
// SpatialNode: root and parent
// ============================================================================

#[test]
fn test_spatial_node_root_and_parent() {
    let root = SpatialNode::root([8, 8, 8]);
    assert_eq!(root.level, 0);
    assert_eq!(root.x, 0);
    assert_eq!(root.y, 0);
    assert_eq!(root.z, 0);
    assert!(root.parent().is_none());

    let child = root.child(5); // x=1, y=0, z=1
    let parent = child.parent().unwrap();
    assert_eq!(parent.level, 0);
    assert_eq!(parent.x, 0);
    assert_eq!(parent.y, 0);
    assert_eq!(parent.z, 0);
}

// ============================================================================
// VoxelShapeType: getMinBounds works (from VoxelShapeTypeSpec.js)
// ============================================================================

#[test]
fn test_voxel_shape_type_min_bounds() {
    // Ported from: VoxelShapeTypeSpec "getMinBounds works"
    let box_min = VoxelShapeType::Box.default_min_bounds();
    assert_eq!(box_min, DVec3::new(-1.0, -1.0, -1.0));

    let ellipsoid_min = VoxelShapeType::Ellipsoid.default_min_bounds();
    assert!((ellipsoid_min.x + std::f64::consts::PI).abs() < 1e-10);
    assert!((ellipsoid_min.y + std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    assert_eq!(ellipsoid_min.z, -1.0);

    let cylinder_min = VoxelShapeType::Cylinder.default_min_bounds();
    assert_eq!(cylinder_min.x, 0.0);
    assert!((cylinder_min.y + std::f64::consts::PI).abs() < 1e-10);
    assert_eq!(cylinder_min.z, -1.0);
}

// ============================================================================
// VoxelShapeType: getMaxBounds works (from VoxelShapeTypeSpec.js)
// ============================================================================

#[test]
fn test_voxel_shape_type_max_bounds() {
    // Ported from: VoxelShapeTypeSpec "getMaxBounds works"
    let box_max = VoxelShapeType::Box.default_max_bounds();
    assert_eq!(box_max, DVec3::new(1.0, 1.0, 1.0));

    let ellipsoid_max = VoxelShapeType::Ellipsoid.default_max_bounds();
    assert!((ellipsoid_max.x - std::f64::consts::PI).abs() < 1e-10);
    assert!((ellipsoid_max.y - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
    assert_eq!(ellipsoid_max.z, 1.0);

    let cylinder_max = VoxelShapeType::Cylinder.default_max_bounds();
    assert_eq!(cylinder_max.x, 1.0);
    assert!((cylinder_max.y - std::f64::consts::PI).abs() < 1e-10);
    assert_eq!(cylinder_max.z, 1.0);
}

// ============================================================================
// VoxelShapeType: bounds consistency with shapes
// ============================================================================

#[test]
fn test_voxel_shape_type_bounds_match_shapes() {
    // Verify VoxelShapeType bounds match the actual shape defaults
    let box_shape = VoxelBoxShape::new();
    assert_eq!(
        VoxelShapeType::Box.default_min_bounds(),
        box_shape.min_bounds()
    );
    assert_eq!(
        VoxelShapeType::Box.default_max_bounds(),
        box_shape.max_bounds()
    );

    let cyl_shape = VoxelCylinderShape::new();
    assert_eq!(
        VoxelShapeType::Cylinder.default_min_bounds(),
        cyl_shape.min_bounds()
    );
    assert_eq!(
        VoxelShapeType::Cylinder.default_max_bounds(),
        cyl_shape.max_bounds()
    );

    let ell_shape = VoxelEllipsoidShape::new();
    assert_eq!(
        VoxelShapeType::Ellipsoid.default_min_bounds(),
        ell_shape.min_bounds()
    );
    assert_eq!(
        VoxelShapeType::Ellipsoid.default_max_bounds(),
        ell_shape.max_bounds()
    );
}

// ============================================================================
// VoxelTraversal: basic traversal
// ============================================================================

#[test]
fn test_voxel_traversal_basic() {
    // Basic traversal with box shape
    let traversal = VoxelTraversal::default();
    assert!(traversal.is_level_available(0));
    assert!(traversal.is_level_available(1));

    let mut shape = VoxelBoxShape::new();
    shape.update(DMat4::IDENTITY, DVec3::splat(-1.0), DVec3::ONE, None, None);

    let result = traversal.traverse(&shape, DVec3::new(0.0, 0.0, 5.0), 1024.0, std::f64::consts::FRAC_PI_2);

    // Should visit at least the root
    assert!(result.nodes_visited > 0);
    // Should have some render or refine nodes
    assert!(!result.render_nodes.is_empty() || !result.refine_nodes.is_empty());
}

// ============================================================================
// VoxelTraversal: level availability
// ============================================================================

#[test]
fn test_voxel_traversal_level_availability() {
    let mut traversal = VoxelTraversal::default();

    // Disable level 2
    traversal.set_level_available(2, false);
    assert!(traversal.is_level_available(0));
    assert!(traversal.is_level_available(1));
    assert!(!traversal.is_level_available(2));
    assert!(traversal.is_level_available(3)); // Still available

    // Non-existent level
    assert!(!traversal.is_level_available(100));
}
