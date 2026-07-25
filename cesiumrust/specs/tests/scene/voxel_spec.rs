//! Scene/VoxelBoxShapeSpec.js, VoxelCylinderShapeSpec.js, VoxelEllipsoidShapeSpec.js
//! → Rust integration tests

use cesium_voxel::{VoxelShapeType, VoxelBoxShape, VoxelCylinderShape, VoxelEllipsoidShape};
use glam::DVec3;

// === VoxelShapeType ===

#[test]
fn test_voxel_shape_type_variants() {
    assert_ne!(VoxelShapeType::Box, VoxelShapeType::Cylinder);
    assert_ne!(VoxelShapeType::Cylinder, VoxelShapeType::Ellipsoid);
    assert_ne!(VoxelShapeType::Box, VoxelShapeType::Ellipsoid);
}

#[test]
fn test_voxel_shape_type_default_min_bounds() {
    let box_bounds = VoxelShapeType::Box.default_min_bounds();
    assert_eq!(box_bounds, DVec3::new(-1.0, -1.0, -1.0));
}

// === VoxelBoxShape ===

#[test]
fn test_voxel_box_shape_new() {
    let shape = VoxelBoxShape::new();
    let min_b = shape.min_bounds();
    let max_b = shape.max_bounds();
    assert!(min_b.x <= max_b.x);
    assert!(min_b.y <= max_b.y);
    assert!(min_b.z <= max_b.z);
}

#[test]
fn test_voxel_box_shape_contains_local() {
    let shape = VoxelBoxShape::new();
    // Origin should be inside default box
    assert!(shape.contains_local(DVec3::ZERO));
}

// === VoxelCylinderShape ===

#[test]
fn test_voxel_cylinder_shape_new() {
    let shape = VoxelCylinderShape::new();
    let min_b = shape.min_bounds();
    let max_b = shape.max_bounds();
    assert!(min_b.x <= max_b.x);
}

// === VoxelEllipsoidShape ===

#[test]
fn test_voxel_ellipsoid_shape_new() {
    let shape = VoxelEllipsoidShape::new();
    let min_b = shape.min_bounds();
    let max_b = shape.max_bounds();
    assert!(min_b.x <= max_b.x);
}
