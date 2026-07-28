//! ImplicitTileCoordinates tests ported from CesiumJS ImplicitTileCoordinatesSpec.js
//! Tests: getDescendantCoordinates, getAncestorCoordinates, getOffsetCoordinates,
//! getChildCoordinates, getSubtreeCoordinates, getParentSubtreeCoordinates,
//! isAncestor, isImplicitTilesetRoot, isSubtreeRoot, isBottomOfSubtree,
//! childIndex, mortonIndex, tileIndex, fromMortonIndex, fromTileIndex

use cesium_implicit_tiling::{
    decode_morton_2d, decode_morton_3d, morton_2d, morton_3d, ImplicitTileCoord,
    SubdivisionScheme,
};

fn qt(level: u32, x: u32, y: u32) -> ImplicitTileCoord {
    ImplicitTileCoord::quadtree(level, x, y)
}

fn qt_s(level: u32, x: u32, y: u32, st: u32) -> ImplicitTileCoord {
    ImplicitTileCoord::quadtree_with_subtree(level, x, y, st)
}

fn ot(level: u32, x: u32, y: u32, z: u32) -> ImplicitTileCoord {
    ImplicitTileCoord::octree(level, x, y, z)
}

fn ot_s(level: u32, x: u32, y: u32, z: u32, st: u32) -> ImplicitTileCoord {
    ImplicitTileCoord::octree_with_subtree(level, x, y, z, st)
}

// ============================================================================
// getDescendantCoordinates
// ============================================================================

#[test]
fn test_get_descendant_coordinates_quadtree() {
    // Ported from: "getDescendantCoordinates works as expected for quadtree"
    assert_eq!(
        qt(0, 0, 0).get_descendant_coordinates(&qt(0, 0, 0)),
        qt(0, 0, 0)
    );
    assert_eq!(
        qt(0, 0, 0).get_descendant_coordinates(&qt(1, 1, 1)),
        qt(1, 1, 1)
    );
    assert_eq!(
        qt(1, 1, 1).get_descendant_coordinates(&qt(2, 3, 3)),
        qt(3, 7, 7)
    );
}

#[test]
fn test_get_descendant_coordinates_octree() {
    // Ported from: "getDescendantCoordinates works as expected for octree"
    assert_eq!(
        ot(0, 0, 0, 0).get_descendant_coordinates(&ot(0, 0, 0, 0)),
        ot(0, 0, 0, 0)
    );
    assert_eq!(
        ot(0, 0, 0, 0).get_descendant_coordinates(&ot(1, 1, 1, 1)),
        ot(1, 1, 1, 1)
    );
    assert_eq!(
        ot(1, 1, 1, 1).get_descendant_coordinates(&ot(2, 3, 3, 3)),
        ot(3, 7, 7, 7)
    );
}

// ============================================================================
// getAncestorCoordinates
// ============================================================================

#[test]
fn test_get_ancestor_coordinates_quadtree() {
    // Ported from: "getAncestorCoordinates works as expected for quadtree"
    assert_eq!(qt(0, 0, 0).get_ancestor_coordinates(0), qt(0, 0, 0));
    assert_eq!(qt(1, 0, 0).get_ancestor_coordinates(1), qt(0, 0, 0));
    assert_eq!(qt(1, 1, 1).get_ancestor_coordinates(1), qt(0, 0, 0));
    assert_eq!(qt(2, 3, 3).get_ancestor_coordinates(1), qt(1, 1, 1));
    assert_eq!(qt(2, 3, 3).get_ancestor_coordinates(2), qt(0, 0, 0));
}

#[test]
fn test_get_ancestor_coordinates_octree() {
    // Ported from: "getAncestorCoordinates works as expected for octree"
    assert_eq!(ot(0, 0, 0, 0).get_ancestor_coordinates(0), ot(0, 0, 0, 0));
    assert_eq!(ot(1, 0, 0, 0).get_ancestor_coordinates(1), ot(0, 0, 0, 0));
    assert_eq!(ot(1, 1, 1, 1).get_ancestor_coordinates(1), ot(0, 0, 0, 0));
    assert_eq!(ot(2, 3, 3, 3).get_ancestor_coordinates(1), ot(1, 1, 1, 1));
    assert_eq!(ot(2, 3, 3, 3).get_ancestor_coordinates(2), ot(0, 0, 0, 0));
}

// ============================================================================
// getOffsetCoordinates
// ============================================================================

#[test]
fn test_get_offset_coordinates_quadtree() {
    // Ported from: "getOffsetCoordinates works as expected for quadtree"
    assert_eq!(qt(0, 0, 0).get_offset_coordinates(&qt(0, 0, 0)), qt(0, 0, 0));
    assert_eq!(qt(0, 0, 0).get_offset_coordinates(&qt(1, 1, 1)), qt(1, 1, 1));
    assert_eq!(qt(1, 1, 1).get_offset_coordinates(&qt(3, 7, 7)), qt(2, 3, 3));
}

#[test]
fn test_get_offset_coordinates_octree() {
    // Ported from: "getOffsetCoordinates works as expected for octree"
    assert_eq!(
        ot(0, 0, 0, 0).get_offset_coordinates(&ot(0, 0, 0, 0)),
        ot(0, 0, 0, 0)
    );
    assert_eq!(
        ot(0, 0, 0, 0).get_offset_coordinates(&ot(1, 1, 1, 1)),
        ot(1, 1, 1, 1)
    );
    assert_eq!(
        ot(1, 1, 1, 1).get_offset_coordinates(&ot(3, 7, 7, 7)),
        ot(2, 3, 3, 3)
    );
}

// ============================================================================
// getChildCoordinates
// ============================================================================

#[test]
fn test_get_child_coordinates_quadtree() {
    // Ported from: "getChildCoordinates works as expected for quadtree"
    let coord = qt(0, 0, 0);
    assert_eq!(coord.get_child_coordinates(0), qt(1, 0, 0));
    assert_eq!(coord.get_child_coordinates(1), qt(1, 1, 0));
    assert_eq!(coord.get_child_coordinates(2), qt(1, 0, 1));
    assert_eq!(coord.get_child_coordinates(3), qt(1, 1, 1));
}

#[test]
fn test_get_child_coordinates_octree() {
    // Ported from: "getChildCoordinates works as expected for octree"
    let coord = ot(0, 0, 0, 0);
    assert_eq!(coord.get_child_coordinates(0), ot(1, 0, 0, 0));
    assert_eq!(coord.get_child_coordinates(1), ot(1, 1, 0, 0));
    assert_eq!(coord.get_child_coordinates(2), ot(1, 0, 1, 0));
    assert_eq!(coord.get_child_coordinates(3), ot(1, 1, 1, 0));
    assert_eq!(coord.get_child_coordinates(4), ot(1, 0, 0, 1));
    assert_eq!(coord.get_child_coordinates(5), ot(1, 1, 0, 1));
    assert_eq!(coord.get_child_coordinates(6), ot(1, 0, 1, 1));
    assert_eq!(coord.get_child_coordinates(7), ot(1, 1, 1, 1));
}

// ============================================================================
// getSubtreeCoordinates / getParentSubtreeCoordinates
// ============================================================================

#[test]
fn test_get_subtree_coordinates_quadtree() {
    // Ported from: "getSubtreeCoordinates works as expected for quadtree"
    // subtreeLevels=2 (default)
    assert_eq!(qt(0, 0, 0).get_subtree_coordinates(), qt(0, 0, 0));
    assert_eq!(qt(1, 1, 1).get_subtree_coordinates(), qt(0, 0, 0));
    assert_eq!(qt(2, 3, 3).get_subtree_coordinates(), qt(2, 3, 3));
    assert_eq!(qt(3, 7, 7).get_subtree_coordinates(), qt(2, 3, 3));
}

#[test]
fn test_get_subtree_coordinates_octree() {
    // Ported from: "getSubtreeCoordinates works as expected for octree"
    assert_eq!(ot(0, 0, 0, 0).get_subtree_coordinates(), ot(0, 0, 0, 0));
    assert_eq!(ot(1, 1, 1, 1).get_subtree_coordinates(), ot(0, 0, 0, 0));
    assert_eq!(ot(2, 3, 3, 3).get_subtree_coordinates(), ot(2, 3, 3, 3));
    assert_eq!(ot(3, 7, 7, 7).get_subtree_coordinates(), ot(2, 3, 3, 3));
}

#[test]
fn test_get_parent_subtree_coordinates_quadtree() {
    // Ported from: "getParentSubtreeCoordinates works as expected for quadtree"
    assert_eq!(qt(2, 3, 3).get_parent_subtree_coordinates(), qt(0, 0, 0));
    assert_eq!(qt(3, 7, 7).get_parent_subtree_coordinates(), qt(0, 0, 0));
    assert_eq!(qt(4, 15, 15).get_parent_subtree_coordinates(), qt(2, 3, 3));
}

#[test]
fn test_get_parent_subtree_coordinates_octree() {
    // Ported from: "getParentSubtreeCoordinates works as expected for octree"
    assert_eq!(
        ot(2, 3, 3, 3).get_parent_subtree_coordinates(),
        ot(0, 0, 0, 0)
    );
    assert_eq!(
        ot(3, 7, 7, 7).get_parent_subtree_coordinates(),
        ot(0, 0, 0, 0)
    );
    assert_eq!(
        ot(4, 15, 15, 15).get_parent_subtree_coordinates(),
        ot(2, 3, 3, 3)
    );
}

// ============================================================================
// isAncestor
// ============================================================================

#[test]
fn test_is_ancestor_quadtree() {
    // Ported from: "isAncestor works as expected for quadtree"
    let scheme = SubdivisionScheme::Quadtree;
    assert!(qt(0, 0, 0).is_ancestor(&qt(1, 0, 0), scheme));
    assert!(qt(0, 0, 0).is_ancestor(&qt(1, 1, 1), scheme));
    assert!(qt(1, 1, 1).is_ancestor(&qt(3, 7, 7), scheme));
    assert!(!qt(1, 1, 1).is_ancestor(&qt(0, 0, 0), scheme));
    assert!(!qt(1, 1, 1).is_ancestor(&qt(1, 1, 1), scheme));
    assert!(!qt(1, 0, 0).is_ancestor(&qt(2, 2, 2), scheme));
}

#[test]
fn test_is_ancestor_octree() {
    // Ported from: "isAncestor works as expected for octree"
    let scheme = SubdivisionScheme::Octree;
    assert!(ot(0, 0, 0, 0).is_ancestor(&ot(1, 0, 0, 0), scheme));
    assert!(ot(0, 0, 0, 0).is_ancestor(&ot(1, 1, 1, 1), scheme));
    assert!(ot(1, 1, 1, 1).is_ancestor(&ot(3, 7, 7, 7), scheme));
    assert!(!ot(1, 1, 1, 1).is_ancestor(&ot(0, 0, 0, 0), scheme));
    assert!(!ot(1, 1, 1, 1).is_ancestor(&ot(1, 1, 1, 1), scheme));
}

// ============================================================================
// isImplicitTilesetRoot / isSubtreeRoot / isBottomOfSubtree
// ============================================================================

#[test]
fn test_is_implicit_tileset_root() {
    // Ported from: "isImplicitTilesetRoot works as expected"
    assert!(qt(0, 0, 0).is_implicit_tileset_root());
    assert!(!qt(1, 0, 0).is_implicit_tileset_root());
    assert!(!qt(2, 0, 0).is_implicit_tileset_root());
}

#[test]
fn test_is_subtree_root() {
    // Ported from: "isSubtreeRoot works as expected" (subtreeLevels=2)
    assert!(qt(0, 0, 0).is_subtree_root());
    assert!(!qt(1, 0, 0).is_subtree_root());
    assert!(qt(2, 0, 0).is_subtree_root());
    assert!(!qt(3, 0, 0).is_subtree_root());
}

#[test]
fn test_is_bottom_of_subtree() {
    // Ported from: "isBottomOfSubtree works as expected" (subtreeLevels=2)
    assert!(!qt(0, 0, 0).is_bottom_of_subtree());
    assert!(qt(1, 0, 0).is_bottom_of_subtree());
    assert!(!qt(2, 0, 0).is_bottom_of_subtree());
    assert!(qt(3, 0, 0).is_bottom_of_subtree());
}

// ============================================================================
// childIndex
// ============================================================================

#[test]
fn test_child_index_quadtree() {
    // Ported from: "childIndex works as expected for quadtree"
    // x=3=0b11, y=2=0b10, interleave last bits: y0=0, x0=1 → 0b01 = 1
    assert_eq!(qt(4, 3, 2).child_index(SubdivisionScheme::Quadtree), 1);
}

#[test]
fn test_child_index_octree() {
    // Ported from: "childIndex works as expected for octree"
    // x=3=0b11, y=2=0b10, z=1=0b01
    // interleave: z0=1, y0=0, x0=1 → 0b101 = 5
    assert_eq!(ot(4, 3, 2, 1).child_index(SubdivisionScheme::Octree), 5);
}

// ============================================================================
// mortonIndex
// ============================================================================

#[test]
fn test_morton_index_quadtree() {
    // Ported from: "mortonIndex works as expected for quadtree"
    // x=5=0b0101, y=11=0b1011, interleave(y,x) = 0b10011011 = 155
    assert_eq!(qt(4, 5, 11).morton_index(SubdivisionScheme::Quadtree), 155);
}

#[test]
fn test_morton_index_octree() {
    // Ported from: "mortonIndex works as expected for octree"
    // x=7, y=15, z=32, interleave(z,y,x) = 132315
    assert_eq!(ot(6, 7, 15, 32).morton_index(SubdivisionScheme::Octree), 132315);
}

// ============================================================================
// tileIndex
// ============================================================================

#[test]
fn test_tile_index_quadtree() {
    // Ported from: "tileIndex works as expected for quadtree"
    // level=4, morton=155, levelOffset=(4^4-1)/3=85, tileIndex=85+155=240
    assert_eq!(qt(4, 5, 11).tile_index(SubdivisionScheme::Quadtree), 240);
}

#[test]
fn test_tile_index_octree() {
    // Ported from: "tileIndex works as expected for octree"
    // level=6, morton=132315, levelOffset=(8^6-1)/7=37449, tileIndex=37449+132315=169764
    assert_eq!(ot(6, 7, 15, 32).tile_index(SubdivisionScheme::Octree), 169764);
}

// ============================================================================
// fromMortonIndex
// ============================================================================

#[test]
fn test_from_morton_index_quadtree() {
    // Ported from: "fromMortonIndex works as expected for quadtree"
    // 42 = 0b101010, deinterleave2D(42) = x=0b000=0, y=0b111=7
    let coord = ImplicitTileCoord::from_morton_index(SubdivisionScheme::Quadtree, 6, 3, 42);
    assert_eq!(coord, qt_s(3, 0, 7, 6));
}

#[test]
fn test_from_morton_index_octree() {
    // Ported from: "fromMortonIndex works as expected for octree"
    // 43 = 0b101011, deinterleave3D(43) = x=0b11=3, y=0b01=1, z=0b10=2
    let coord = ImplicitTileCoord::from_morton_index(SubdivisionScheme::Octree, 6, 2, 43);
    assert_eq!(coord, ot_s(2, 3, 1, 2, 6));
}

// ============================================================================
// fromTileIndex
// ============================================================================

#[test]
fn test_from_tile_index_quadtree() {
    // Ported from: "fromTileIndex works as expected for quadtree"
    // tileIndex=63, level=floor(log2(3*63+1)/2)=floor(log2(190)/2)=floor(7.57/2)=3
    // levelOffset=(4^3-1)/3=21, morton=63-21=42
    // deinterleave2D(42) = x=0, y=7
    let coord = ImplicitTileCoord::from_tile_index(SubdivisionScheme::Quadtree, 6, 63);
    assert_eq!(coord, qt_s(3, 0, 7, 6));
}

#[test]
fn test_from_tile_index_octree() {
    // Ported from: "fromTileIndex works as expected for octree"
    // tileIndex=52, level=floor(log2(7*52+1)/3)=floor(log2(365)/3)=floor(8.51/3)=2
    // levelOffset=(8^2-1)/7=9, morton=52-9=43
    // deinterleave3D(43) = x=3, y=1, z=2
    let coord = ImplicitTileCoord::from_tile_index(SubdivisionScheme::Octree, 6, 52);
    assert_eq!(coord, ot_s(2, 3, 1, 2, 6));
}

// ============================================================================
// Morton encode/decode round-trip
// ============================================================================

#[test]
fn test_morton_2d_round_trip() {
    for x in 0..16u32 {
        for y in 0..16u32 {
            let encoded = morton_2d(x, y);
            let (dx, dy) = decode_morton_2d(encoded);
            assert_eq!((x, y), (dx, dy), "morton_2d roundtrip failed for ({}, {})", x, y);
        }
    }
}

#[test]
fn test_morton_3d_round_trip() {
    for x in 0..8u32 {
        for y in 0..8u32 {
            for z in 0..8u32 {
                let encoded = morton_3d(x, y, z);
                let (dx, dy, dz) = decode_morton_3d(encoded);
                assert_eq!(
                    (x, y, z),
                    (dx, dy, dz),
                    "morton_3d roundtrip failed for ({}, {}, {})",
                    x, y, z
                );
            }
        }
    }
}

// ============================================================================
// tileIndex ↔ fromTileIndex round-trip
// ============================================================================

#[test]
fn test_tile_index_round_trip_quadtree() {
    for level in 0..5u32 {
        let dim = 1u32 << level;
        for x in 0..dim.min(4) {
            for y in 0..dim.min(4) {
                let coord = qt(level, x, y);
                let idx = coord.tile_index(SubdivisionScheme::Quadtree);
                let decoded = ImplicitTileCoord::from_tile_index(SubdivisionScheme::Quadtree, 2, idx);
                assert_eq!(
                    (coord.level, coord.x, coord.y),
                    (decoded.level, decoded.x, decoded.y),
                    "tileIndex roundtrip failed for level={} x={} y={}",
                    level, x, y
                );
            }
        }
    }
}

#[test]
fn test_tile_index_round_trip_octree() {
    for level in 0..4u32 {
        let dim = 1u32 << level;
        for x in 0..dim.min(3) {
            for y in 0..dim.min(3) {
                for z in 0..dim.min(3) {
                    let coord = ot(level, x, y, z);
                    let idx = coord.tile_index(SubdivisionScheme::Octree);
                    let decoded =
                        ImplicitTileCoord::from_tile_index(SubdivisionScheme::Octree, 2, idx);
                    assert_eq!(
                        (coord.level, coord.x, coord.y, coord.z),
                        (decoded.level, decoded.x, decoded.y, decoded.z),
                        "tileIndex roundtrip failed for level={} x={} y={} z={}",
                        level, x, y, z
                    );
                }
            }
        }
    }
}
