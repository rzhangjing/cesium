//! Scene/Implicit3DTileContentSpec.js → Rust integration tests

use cesium_implicit_tiling::{
    morton_2d, morton_3d, AvailabilityBitstream, ImplicitTileCoord, ImplicitTilingConfig,
    SubdivisionScheme, Subtree,
};

// === SubdivisionScheme ===

#[test]
fn test_subdivision_scheme_quadtree() {
    assert_eq!(SubdivisionScheme::Quadtree.branching_factor(), 4);
    assert_eq!(SubdivisionScheme::Quadtree.dimensions(), 2);
}

#[test]
fn test_subdivision_scheme_octree() {
    assert_eq!(SubdivisionScheme::Octree.branching_factor(), 8);
    assert_eq!(SubdivisionScheme::Octree.dimensions(), 3);
}

#[test]
fn test_subdivision_scheme_default() {
    assert_eq!(SubdivisionScheme::default(), SubdivisionScheme::Quadtree);
}

// === Morton codes ===

#[test]
fn test_morton_2d_basic() {
    assert_eq!(morton_2d(0, 0), 0);
    assert_eq!(morton_2d(1, 0), 2);
    assert_eq!(morton_2d(0, 1), 1);
    assert_eq!(morton_2d(1, 1), 3);
}

#[test]
fn test_morton_2d_larger() {
    assert_eq!(morton_2d(2, 0), 8);
    assert_eq!(morton_2d(3, 3), 15);
}

#[test]
fn test_morton_3d_basic() {
    assert_eq!(morton_3d(0, 0, 0), 0);
    assert_eq!(morton_3d(1, 0, 0), 4);
    assert_eq!(morton_3d(0, 1, 0), 2);
    assert_eq!(morton_3d(0, 0, 1), 1);
    assert_eq!(morton_3d(1, 1, 1), 7);
}

// === ImplicitTileCoord ===

#[test]
fn test_tile_coord_quadtree() {
    let coord = ImplicitTileCoord::quadtree(2, 3, 1);
    assert_eq!(coord.level, 2);
    assert_eq!(coord.x, 3);
    assert_eq!(coord.y, 1);
    assert_eq!(coord.z, 0);
}

#[test]
fn test_tile_coord_octree() {
    let coord = ImplicitTileCoord::octree(1, 1, 0, 1);
    assert_eq!(coord.level, 1);
    assert_eq!(coord.z, 1);
}

#[test]
fn test_tile_coord_parent() {
    let coord = ImplicitTileCoord::quadtree(2, 3, 2);
    let parent = coord.parent().unwrap();
    assert_eq!(parent.level, 1);
    assert_eq!(parent.x, 1);
    assert_eq!(parent.y, 1);
}

#[test]
fn test_tile_coord_root_no_parent() {
    let coord = ImplicitTileCoord::quadtree(0, 0, 0);
    assert!(coord.parent().is_none());
}

#[test]
fn test_tile_coord_children_quadtree() {
    let coord = ImplicitTileCoord::quadtree(0, 0, 0);
    let children = coord.children(SubdivisionScheme::Quadtree);
    assert_eq!(children.len(), 4);
    assert_eq!(children[0], ImplicitTileCoord::quadtree(1, 0, 0));
    assert_eq!(children[3], ImplicitTileCoord::quadtree(1, 1, 1));
}

#[test]
fn test_tile_coord_children_octree() {
    let coord = ImplicitTileCoord::octree(0, 0, 0, 0);
    let children = coord.children(SubdivisionScheme::Octree);
    assert_eq!(children.len(), 8);
}

#[test]
fn test_tiles_at_level() {
    assert_eq!(ImplicitTileCoord::tiles_at_level(0, SubdivisionScheme::Quadtree), 1);
    assert_eq!(ImplicitTileCoord::tiles_at_level(1, SubdivisionScheme::Quadtree), 4);
    assert_eq!(ImplicitTileCoord::tiles_at_level(2, SubdivisionScheme::Quadtree), 16);
    assert_eq!(ImplicitTileCoord::tiles_at_level(1, SubdivisionScheme::Octree), 8);
    assert_eq!(ImplicitTileCoord::tiles_at_level(2, SubdivisionScheme::Octree), 64);
}

#[test]
fn test_tile_coord_morton_index() {
    let coord = ImplicitTileCoord::quadtree(1, 1, 0);
    assert_eq!(coord.morton_index(SubdivisionScheme::Quadtree), morton_2d(1, 0));
}

// === AvailabilityBitstream ===

#[test]
fn test_availability_bitstream_new() {
    let bs = AvailabilityBitstream::new(16);
    assert_eq!(bs.length, 16);
    assert!(!bs.is_available(0));
    assert!(!bs.is_available(15));
}

#[test]
fn test_availability_bitstream_set_get() {
    let mut bs = AvailabilityBitstream::new(16);
    bs.set(0, true);
    bs.set(5, true);
    bs.set(15, true);

    assert!(bs.is_available(0));
    assert!(!bs.is_available(1));
    assert!(bs.is_available(5));
    assert!(bs.is_available(15));
    assert!(!bs.is_available(16)); // out of bounds
}

#[test]
fn test_availability_bitstream_unset() {
    let mut bs = AvailabilityBitstream::new(8);
    bs.set(3, true);
    assert!(bs.is_available(3));
    bs.set(3, false);
    assert!(!bs.is_available(3));
}

#[test]
fn test_availability_bitstream_count() {
    let mut bs = AvailabilityBitstream::new(8);
    bs.set(0, true);
    bs.set(3, true);
    bs.set(7, true);
    assert_eq!(bs.count_available(), 3);
}

#[test]
fn test_availability_bitstream_from_bytes() {
    // 0b00000101 = bits 0 and 2 set
    let bs = AvailabilityBitstream::from_bytes(vec![0x05], 8);
    assert!(bs.is_available(0));
    assert!(!bs.is_available(1));
    assert!(bs.is_available(2));
}

// === ImplicitTilingConfig ===

#[test]
fn test_implicit_tiling_config_subtree_uri() {
    let config = ImplicitTilingConfig {
        subdivision_scheme: SubdivisionScheme::Quadtree,
        subtree_levels: 4,
        maximum_level: 16,
        subtree_uri_template: "subtrees/{level}/{x}/{y}.subtree".to_string(),
        content_uri_template: "tiles/{level}/{x}/{y}.glb".to_string(),
    };

    let coord = ImplicitTileCoord::quadtree(4, 3, 7);
    assert_eq!(config.get_subtree_uri(&coord), "subtrees/4/3/7.subtree");
    assert_eq!(config.get_content_uri(&coord), "tiles/4/3/7.glb");
}

#[test]
fn test_implicit_tiling_config_subtree_root() {
    let config = ImplicitTilingConfig {
        subdivision_scheme: SubdivisionScheme::Quadtree,
        subtree_levels: 4,
        maximum_level: 16,
        subtree_uri_template: String::new(),
        content_uri_template: String::new(),
    };

    let coord = ImplicitTileCoord::quadtree(6, 15, 23);
    let root = config.get_subtree_root(&coord);
    assert_eq!(root.level, 4);
    assert_eq!(root.x, 3);
    assert_eq!(root.y, 5);
}

// === Subtree ===

#[test]
fn test_subtree_total_nodes_quadtree() {
    // 1 + 4 + 16 + 64 = 85
    assert_eq!(Subtree::total_nodes(4, SubdivisionScheme::Quadtree), 85);
}

#[test]
fn test_subtree_total_nodes_octree() {
    // 1 + 8 = 9
    assert_eq!(Subtree::total_nodes(2, SubdivisionScheme::Octree), 9);
}

#[test]
fn test_subtree_local_index() {
    let root = ImplicitTileCoord::quadtree(0, 0, 0);
    let coord = ImplicitTileCoord::quadtree(1, 1, 0);
    let index = Subtree::local_index(&coord, &root, SubdivisionScheme::Quadtree);
    // Level 1 offset = 1, morton(1,0) = 2
    assert_eq!(index, 3);
}
