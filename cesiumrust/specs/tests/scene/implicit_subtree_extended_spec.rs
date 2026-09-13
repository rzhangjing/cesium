//! Implicit tiling extended specs - ported from ImplicitSubtreeSpec.js,
//! ImplicitTileCoordinatesSpec.js (advanced operations)
//!
//! Tests subtree local_index, total_nodes, ancestor/descendant/offset coordinates,
//! Morton encode/decode roundtrips, ImplicitTilingConfig URI generation, and
//! AvailabilityBitstream advanced operations.

use cesium_implicit_tiling::{
    decode_morton_2d, decode_morton_3d, morton_2d, morton_3d, AvailabilityBitstream,
    ImplicitTileCoord, ImplicitTilingConfig, SubdivisionScheme, Subtree,
};

// ─── Subtree::total_nodes ──────────────────────────────────────────────────

#[test]
fn subtree_total_nodes_quadtree() {
    // Quadtree: 1 + 4 + 16 = 21 for 3 levels
    assert_eq!(Subtree::total_nodes(1, SubdivisionScheme::Quadtree), 1);
    assert_eq!(Subtree::total_nodes(2, SubdivisionScheme::Quadtree), 5);
    assert_eq!(Subtree::total_nodes(3, SubdivisionScheme::Quadtree), 21);
    assert_eq!(Subtree::total_nodes(4, SubdivisionScheme::Quadtree), 85);
}

#[test]
fn subtree_total_nodes_octree() {
    // Octree: 1 + 8 + 64 = 73 for 3 levels
    assert_eq!(Subtree::total_nodes(1, SubdivisionScheme::Octree), 1);
    assert_eq!(Subtree::total_nodes(2, SubdivisionScheme::Octree), 9);
    assert_eq!(Subtree::total_nodes(3, SubdivisionScheme::Octree), 73);
}

// ─── Subtree::local_index ──────────────────────────────────────────────────

#[test]
fn subtree_local_index_root() {
    let root = ImplicitTileCoord::quadtree_with_subtree(0, 0, 0, 4);
    let idx = Subtree::local_index(&root, &root, SubdivisionScheme::Quadtree);
    assert_eq!(idx, 0);
}

#[test]
fn subtree_local_index_level1_quadtree() {
    let root = ImplicitTileCoord::quadtree_with_subtree(0, 0, 0, 4);
    // Level 1 starts at offset 1 (after root)
    let c00 = ImplicitTileCoord::quadtree_with_subtree(1, 0, 0, 4);
    let c10 = ImplicitTileCoord::quadtree_with_subtree(1, 1, 0, 4);
    let c01 = ImplicitTileCoord::quadtree_with_subtree(1, 0, 1, 4);
    let c11 = ImplicitTileCoord::quadtree_with_subtree(1, 1, 1, 4);
    assert_eq!(Subtree::local_index(&c00, &root, SubdivisionScheme::Quadtree), 1);
    assert_eq!(Subtree::local_index(&c10, &root, SubdivisionScheme::Quadtree), 2);
    assert_eq!(Subtree::local_index(&c01, &root, SubdivisionScheme::Quadtree), 3);
    assert_eq!(Subtree::local_index(&c11, &root, SubdivisionScheme::Quadtree), 4);
}

#[test]
fn subtree_local_index_level2_quadtree() {
    let root = ImplicitTileCoord::quadtree_with_subtree(0, 0, 0, 4);
    // Level 2 starts at offset 5 (1 + 4)
    let coord = ImplicitTileCoord::quadtree_with_subtree(2, 0, 0, 4);
    assert_eq!(Subtree::local_index(&coord, &root, SubdivisionScheme::Quadtree), 5);
    // morton(1,1) = 3, so (2, 1, 1) → 5 + 3 = 8
    let coord2 = ImplicitTileCoord::quadtree_with_subtree(2, 1, 1, 4);
    assert_eq!(Subtree::local_index(&coord2, &root, SubdivisionScheme::Quadtree), 8);
}

#[test]
fn subtree_local_index_non_root_subtree() {
    // Subtree rooted at level 2, coord (2, 1, 1)
    let root = ImplicitTileCoord::quadtree_with_subtree(2, 1, 1, 4);
    let idx = Subtree::local_index(&root, &root, SubdivisionScheme::Quadtree);
    assert_eq!(idx, 0);
    // Child at level 3, (2, 0) relative → (3, 2, 2) absolute
    let child = ImplicitTileCoord::quadtree_with_subtree(3, 2, 2, 4);
    let child_idx = Subtree::local_index(&child, &root, SubdivisionScheme::Quadtree);
    assert_eq!(child_idx, 1); // level_offset=1, morton(0,0)=0
}

// ─── Ancestor/Descendant/Offset coordinates ────────────────────────────────

#[test]
fn get_descendant_coordinates_basic() {
    let ancestor = ImplicitTileCoord::quadtree_with_subtree(1, 1, 0, 4);
    let offset = ImplicitTileCoord::quadtree_with_subtree(2, 3, 1, 4);
    let desc = ancestor.get_descendant_coordinates(&offset);
    assert_eq!(desc.level, 3);
    assert_eq!(desc.x, (1 << 2) + 3); // 7
    assert_eq!(desc.y, (0 << 2) + 1); // 1
}

#[test]
fn get_ancestor_coordinates_basic() {
    let coord = ImplicitTileCoord::quadtree_with_subtree(5, 13, 7, 4);
    let ancestor = coord.get_ancestor_coordinates(2);
    assert_eq!(ancestor.level, 3);
    assert_eq!(ancestor.x, 13 / 4); // 3
    assert_eq!(ancestor.y, 7 / 4); // 1
}

#[test]
fn get_offset_coordinates_basic() {
    let ancestor = ImplicitTileCoord::quadtree_with_subtree(1, 1, 0, 4);
    let descendant = ImplicitTileCoord::quadtree_with_subtree(3, 5, 2, 4);
    let offset = ancestor.get_offset_coordinates(&descendant);
    assert_eq!(offset.level, 2);
    assert_eq!(offset.x, 5 % 4); // 1
    assert_eq!(offset.y, 2 % 4); // 2
}

#[test]
fn descendant_ancestor_roundtrip() {
    let ancestor = ImplicitTileCoord::quadtree_with_subtree(2, 3, 1, 4);
    let offset = ImplicitTileCoord::quadtree_with_subtree(3, 5, 2, 4);
    let desc = ancestor.get_descendant_coordinates(&offset);
    let recovered = desc.get_ancestor_coordinates(3);
    assert_eq!(recovered.level, ancestor.level);
    assert_eq!(recovered.x, ancestor.x);
    assert_eq!(recovered.y, ancestor.y);
}

// ─── get_child_coordinates ─────────────────────────────────────────────────

#[test]
fn get_child_coordinates_quadtree() {
    let parent = ImplicitTileCoord::quadtree_with_subtree(1, 1, 1, 4);
    let c0 = parent.get_child_coordinates(0);
    let c1 = parent.get_child_coordinates(1);
    let c2 = parent.get_child_coordinates(2);
    let c3 = parent.get_child_coordinates(3);
    assert_eq!(c0, ImplicitTileCoord::quadtree_with_subtree(2, 2, 2, 4));
    assert_eq!(c1, ImplicitTileCoord::quadtree_with_subtree(2, 3, 2, 4));
    assert_eq!(c2, ImplicitTileCoord::quadtree_with_subtree(2, 2, 3, 4));
    assert_eq!(c3, ImplicitTileCoord::quadtree_with_subtree(2, 3, 3, 4));
}

#[test]
fn get_child_coordinates_octree() {
    let parent = ImplicitTileCoord::octree_with_subtree(0, 0, 0, 0, 4);
    let c7 = parent.get_child_coordinates(7);
    assert_eq!(c7.level, 1);
    assert_eq!(c7.x, 1);
    assert_eq!(c7.y, 1);
    assert_eq!(c7.z, 1);
}

// ─── Subtree coordinates ───────────────────────────────────────────────────

#[test]
fn get_subtree_coordinates_at_root() {
    let coord = ImplicitTileCoord::quadtree_with_subtree(0, 0, 0, 4);
    let subtree_coord = coord.get_subtree_coordinates();
    assert_eq!(subtree_coord.level, 0);
}

#[test]
fn get_subtree_coordinates_mid_subtree() {
    // subtree_levels=4, level 6 → 6%4=2, ancestor by 2 → level 4
    let coord = ImplicitTileCoord::quadtree_with_subtree(6, 15, 8, 4);
    let subtree_coord = coord.get_subtree_coordinates();
    assert_eq!(subtree_coord.level, 4);
    assert_eq!(subtree_coord.x, 15 / 4); // 3
    assert_eq!(subtree_coord.y, 8 / 4); // 2
}

#[test]
fn is_subtree_root_check() {
    let root = ImplicitTileCoord::quadtree_with_subtree(0, 0, 0, 4);
    assert!(root.is_subtree_root());
    let level4 = ImplicitTileCoord::quadtree_with_subtree(4, 3, 2, 4);
    assert!(level4.is_subtree_root());
    let level2 = ImplicitTileCoord::quadtree_with_subtree(2, 1, 1, 4);
    assert!(!level2.is_subtree_root());
}

#[test]
fn is_bottom_of_subtree_check() {
    // subtree_levels=4: bottom levels are 3, 7, 11, ...
    let level3 = ImplicitTileCoord::quadtree_with_subtree(3, 0, 0, 4);
    assert!(level3.is_bottom_of_subtree());
    let level2 = ImplicitTileCoord::quadtree_with_subtree(2, 0, 0, 4);
    assert!(!level2.is_bottom_of_subtree());
}

// ─── is_ancestor ───────────────────────────────────────────────────────────

#[test]
fn is_ancestor_true() {
    let ancestor = ImplicitTileCoord::quadtree_with_subtree(1, 1, 0, 4);
    let descendant = ImplicitTileCoord::quadtree_with_subtree(3, 5, 1, 4);
    assert!(ancestor.is_ancestor(&descendant, SubdivisionScheme::Quadtree));
}

#[test]
fn is_ancestor_false_same_level() {
    let a = ImplicitTileCoord::quadtree_with_subtree(2, 1, 1, 4);
    let b = ImplicitTileCoord::quadtree_with_subtree(2, 2, 2, 4);
    assert!(!a.is_ancestor(&b, SubdivisionScheme::Quadtree));
}

#[test]
fn is_ancestor_false_wrong_branch() {
    let ancestor = ImplicitTileCoord::quadtree_with_subtree(1, 0, 0, 4);
    let descendant = ImplicitTileCoord::quadtree_with_subtree(3, 7, 7, 4);
    assert!(!ancestor.is_ancestor(&descendant, SubdivisionScheme::Quadtree));
}

// ─── from_morton_index / from_tile_index ───────────────────────────────────

#[test]
fn from_morton_index_quadtree() {
    let coord = ImplicitTileCoord::from_morton_index(SubdivisionScheme::Quadtree, 4, 2, 5);
    assert_eq!(coord.level, 2);
    // morton_2d decode of 5: x=1, y=2 (binary 0101 → x bits: 01=1, y bits: 10=2)
    let (ex, ey) = decode_morton_2d(5);
    assert_eq!(coord.x, ex);
    assert_eq!(coord.y, ey);
}

#[test]
fn from_tile_index_quadtree_root() {
    let coord = ImplicitTileCoord::from_tile_index(SubdivisionScheme::Quadtree, 4, 0);
    assert_eq!(coord.level, 0);
    assert_eq!(coord.x, 0);
    assert_eq!(coord.y, 0);
}

#[test]
fn from_tile_index_quadtree_level1() {
    // tile_index 1..4 are level 1 (offset 1)
    let coord = ImplicitTileCoord::from_tile_index(SubdivisionScheme::Quadtree, 4, 1);
    assert_eq!(coord.level, 1);
    assert_eq!(coord.x, 0);
    assert_eq!(coord.y, 0);
    let coord4 = ImplicitTileCoord::from_tile_index(SubdivisionScheme::Quadtree, 4, 4);
    assert_eq!(coord4.level, 1);
    assert_eq!(coord4.x, 1);
    assert_eq!(coord4.y, 1);
}

#[test]
fn tile_index_roundtrip() {
    let original = ImplicitTileCoord::quadtree_with_subtree(3, 5, 2, 4);
    let idx = original.tile_index(SubdivisionScheme::Quadtree);
    let recovered = ImplicitTileCoord::from_tile_index(SubdivisionScheme::Quadtree, 4, idx);
    assert_eq!(recovered.level, original.level);
    assert_eq!(recovered.x, original.x);
    assert_eq!(recovered.y, original.y);
}

// ─── Morton encode/decode roundtrip ────────────────────────────────────────

#[test]
fn morton_2d_roundtrip() {
    for x in 0..8u32 {
        for y in 0..8u32 {
            let code = morton_2d(x, y);
            let (dx, dy) = decode_morton_2d(code);
            assert_eq!((dx, dy), (x, y), "Failed for ({x},{y})");
        }
    }
}

#[test]
fn morton_3d_roundtrip() {
    for x in 0..4u32 {
        for y in 0..4u32 {
            for z in 0..4u32 {
                let code = morton_3d(x, y, z);
                let (dx, dy, dz) = decode_morton_3d(code);
                assert_eq!((dx, dy, dz), (x, y, z), "Failed for ({x},{y},{z})");
            }
        }
    }
}

// ─── ImplicitTilingConfig ──────────────────────────────────────────────────

#[test]
fn config_subtree_uri() {
    let config = ImplicitTilingConfig {
        subdivision_scheme: SubdivisionScheme::Quadtree,
        subtree_levels: 4,
        maximum_level: 12,
        subtree_uri_template: "tiles/{level}/{x}/{y}.subtree".to_string(),
        content_uri_template: "tiles/{level}/{x}/{y}.b3dm".to_string(),
    };
    let coord = ImplicitTileCoord::quadtree_with_subtree(4, 3, 7, 4);
    assert_eq!(config.get_subtree_uri(&coord), "tiles/4/3/7.subtree");
    assert_eq!(config.get_content_uri(&coord), "tiles/4/3/7.b3dm");
}

#[test]
fn config_subtree_root() {
    let config = ImplicitTilingConfig {
        subdivision_scheme: SubdivisionScheme::Quadtree,
        subtree_levels: 4,
        maximum_level: 16,
        subtree_uri_template: String::new(),
        content_uri_template: String::new(),
    };
    // Level 6, subtree_levels=4 → subtree root at level 4
    let coord = ImplicitTileCoord::quadtree_with_subtree(6, 13, 9, 4);
    let root = config.get_subtree_root(&coord);
    assert_eq!(root.level, 4);
    assert_eq!(root.x, 13 >> 2); // 3
    assert_eq!(root.y, 9 >> 2); // 2
}

// ─── AvailabilityBitstream extended ────────────────────────────────────────

#[test]
fn bitstream_count_available() {
    let mut bs = AvailabilityBitstream::new(16);
    assert_eq!(bs.count_available(), 0);
    bs.set(0, true);
    bs.set(5, true);
    bs.set(15, true);
    assert_eq!(bs.count_available(), 3);
}

#[test]
fn bitstream_from_bytes() {
    // 0b10100101 = 0xA5 → bits 0,2,5,7 set
    let bs = AvailabilityBitstream::from_bytes(vec![0xA5], 8);
    assert!(bs.is_available(0));
    assert!(!bs.is_available(1));
    assert!(bs.is_available(2));
    assert!(!bs.is_available(3));
    assert!(!bs.is_available(4));
    assert!(bs.is_available(5));
    assert!(!bs.is_available(6));
    assert!(bs.is_available(7));
}
