//! QuadtreeTile adjacency specs - tile neighbor finding
//! Ported from Scene/QuadtreeTileSpec.js (13 A-class tests)

use cesium_quadtree::quadtree_tile_adjacency::{
    create_level_zero_tiles, find_level_zero_tile, find_tile_to_east, find_tile_to_north,
    find_tile_to_south, find_tile_to_west, TileCoord, TilingSchemeDescriptor,
};

// ─── createLevelZeroTiles ───────────────────────────────────────────────────

#[test]
fn creates_expected_number_of_tiles() {
    let scheme = TilingSchemeDescriptor::new(3, 2);
    let tiles = create_level_zero_tiles(&scheme);
    assert_eq!(tiles.len(), 6);
}

#[test]
fn created_tiles_are_ordered_northwest_east_south() {
    let scheme = TilingSchemeDescriptor::new(3, 3);
    let tiles = create_level_zero_tiles(&scheme);
    // Row 0 (north): (0,0), (1,0), (2,0)
    assert_eq!(tiles[0], TileCoord::new(0, 0, 0));
    assert_eq!(tiles[1], TileCoord::new(1, 0, 0));
    assert_eq!(tiles[2], TileCoord::new(2, 0, 0));
    // Row 1: (0,1), (1,1), (2,1)
    assert_eq!(tiles[3], TileCoord::new(0, 1, 0));
    assert_eq!(tiles[4], TileCoord::new(1, 1, 0));
    assert_eq!(tiles[5], TileCoord::new(2, 1, 0));
    // Row 2 (south): (0,2), (1,2), (2,2)
    assert_eq!(tiles[6], TileCoord::new(0, 2, 0));
    assert_eq!(tiles[7], TileCoord::new(1, 2, 0));
    assert_eq!(tiles[8], TileCoord::new(2, 2, 0));
}

// ─── findLevelZeroTile ──────────────────────────────────────────────────────

#[test]
fn wraps_x_around_antimeridian() {
    let scheme = TilingSchemeDescriptor::new(3, 3);
    let tiles = create_level_zero_tiles(&scheme);

    // x=-1 wraps to x=2
    assert_eq!(
        find_level_zero_tile(&scheme, &tiles, -1, 0),
        Some(TileCoord::new(2, 0, 0))
    );
    // x=3 wraps to x=0
    assert_eq!(
        find_level_zero_tile(&scheme, &tiles, 3, 0),
        Some(TileCoord::new(0, 0, 0))
    );
}

#[test]
fn returns_none_for_y_out_of_bounds() {
    let scheme = TilingSchemeDescriptor::new(3, 3);
    let tiles = create_level_zero_tiles(&scheme);

    // North of north pole
    assert_eq!(find_level_zero_tile(&scheme, &tiles, 0, -1), None);
    // South of south pole
    assert_eq!(find_level_zero_tile(&scheme, &tiles, 0, 3), None);
}

// ─── Adjacency at level zero (root tiles) ───────────────────────────────────

#[test]
fn can_get_tiles_around_a_root_tile() {
    let scheme = TilingSchemeDescriptor::new(3, 3);
    let tiles = create_level_zero_tiles(&scheme);

    // L0X0Y0
    let t = TileCoord::new(0, 0, 0);
    assert_eq!(find_tile_to_west(&scheme, &tiles, &t), Some(TileCoord::new(2, 0, 0))); // wraps
    assert_eq!(find_tile_to_east(&scheme, &tiles, &t), Some(TileCoord::new(1, 0, 0)));
    assert_eq!(find_tile_to_north(&scheme, &tiles, &t), None); // north pole
    assert_eq!(find_tile_to_south(&scheme, &tiles, &t), Some(TileCoord::new(0, 1, 0)));

    // L0X1Y0
    let t = TileCoord::new(1, 0, 0);
    assert_eq!(find_tile_to_west(&scheme, &tiles, &t), Some(TileCoord::new(0, 0, 0)));
    assert_eq!(find_tile_to_east(&scheme, &tiles, &t), Some(TileCoord::new(2, 0, 0)));
    assert_eq!(find_tile_to_north(&scheme, &tiles, &t), None);
    assert_eq!(find_tile_to_south(&scheme, &tiles, &t), Some(TileCoord::new(1, 1, 0)));

    // L0X2Y0
    let t = TileCoord::new(2, 0, 0);
    assert_eq!(find_tile_to_west(&scheme, &tiles, &t), Some(TileCoord::new(1, 0, 0)));
    assert_eq!(find_tile_to_east(&scheme, &tiles, &t), Some(TileCoord::new(0, 0, 0))); // wraps
    assert_eq!(find_tile_to_north(&scheme, &tiles, &t), None);
    assert_eq!(find_tile_to_south(&scheme, &tiles, &t), Some(TileCoord::new(2, 1, 0)));

    // L0X0Y1
    let t = TileCoord::new(0, 1, 0);
    assert_eq!(find_tile_to_west(&scheme, &tiles, &t), Some(TileCoord::new(2, 1, 0))); // wraps
    assert_eq!(find_tile_to_east(&scheme, &tiles, &t), Some(TileCoord::new(1, 1, 0)));
    assert_eq!(find_tile_to_north(&scheme, &tiles, &t), Some(TileCoord::new(0, 0, 0)));
    assert_eq!(find_tile_to_south(&scheme, &tiles, &t), Some(TileCoord::new(0, 2, 0)));
}

#[test]
fn can_get_adjacent_tiles_wrapping_around_antimeridian() {
    let scheme = TilingSchemeDescriptor::new(2, 1);
    let tiles = create_level_zero_tiles(&scheme);

    // With 2 tiles in X: tile(0,0) west → tile(1,0), east → tile(1,0)
    let t = TileCoord::new(0, 0, 0);
    assert_eq!(find_tile_to_west(&scheme, &tiles, &t), Some(TileCoord::new(1, 0, 0)));
    assert_eq!(find_tile_to_east(&scheme, &tiles, &t), Some(TileCoord::new(1, 0, 0)));

    let t = TileCoord::new(1, 0, 0);
    assert_eq!(find_tile_to_west(&scheme, &tiles, &t), Some(TileCoord::new(0, 0, 0)));
    assert_eq!(find_tile_to_east(&scheme, &tiles, &t), Some(TileCoord::new(0, 0, 0)));
}

#[test]
fn returns_none_north_of_north_pole_south_of_south_pole() {
    let scheme = TilingSchemeDescriptor::new(2, 1);
    let tiles = create_level_zero_tiles(&scheme);

    let t = TileCoord::new(0, 0, 0);
    assert_eq!(find_tile_to_north(&scheme, &tiles, &t), None);
    assert_eq!(find_tile_to_south(&scheme, &tiles, &t), None);
}

// ─── Adjacency for child tiles (shared parent) ──────────────────────────────

#[test]
fn can_get_tiles_around_a_tile_sharing_common_parent() {
    let scheme = TilingSchemeDescriptor::new(2, 1);
    let tiles = create_level_zero_tiles(&scheme);

    // Children of tile(0,0,0): NW=(0,0,1), NE=(1,0,1), SW=(0,1,1), SE=(1,1,1)
    let nw = TileCoord::new(0, 0, 1);
    let ne = TileCoord::new(1, 0, 1);
    let sw = TileCoord::new(0, 1, 1);
    let se = TileCoord::new(1, 1, 1);

    // NW's east is NE (same parent)
    assert_eq!(find_tile_to_east(&scheme, &tiles, &nw), Some(ne));
    // NW's south is SW (same parent)
    assert_eq!(find_tile_to_south(&scheme, &tiles, &nw), Some(sw));
    // NE's west is NW (same parent)
    assert_eq!(find_tile_to_west(&scheme, &tiles, &ne), Some(nw));
    // NE's south is SE (same parent)
    assert_eq!(find_tile_to_south(&scheme, &tiles, &ne), Some(se));
    // SW's north is NW (same parent)
    assert_eq!(find_tile_to_north(&scheme, &tiles, &sw), Some(nw));
    // SW's east is SE (same parent)
    assert_eq!(find_tile_to_east(&scheme, &tiles, &sw), Some(se));
    // SE's north is NE (same parent)
    assert_eq!(find_tile_to_north(&scheme, &tiles, &se), Some(ne));
    // SE's west is SW (same parent)
    assert_eq!(find_tile_to_west(&scheme, &tiles, &se), Some(sw));
}

// ─── Adjacency for child tiles (different parents) ──────────────────────────

#[test]
fn can_get_tiles_around_a_tile_not_sharing_common_parent() {
    let scheme = TilingSchemeDescriptor::new(2, 1);
    let tiles = create_level_zero_tiles(&scheme);

    // Tile(0,0,0) children: NW=(0,0,1), NE=(1,0,1), SW=(0,1,1), SE=(1,1,1)
    // Tile(1,0,0) children: NW=(2,0,1), NE=(3,0,1), SW=(2,1,1), SE=(3,1,1)

    // NE child of tile(0,0,0) = (1,0,1). East should be NW child of tile(1,0,0) = (2,0,1)
    let ne_of_first = TileCoord::new(1, 0, 1);
    assert_eq!(
        find_tile_to_east(&scheme, &tiles, &ne_of_first),
        Some(TileCoord::new(2, 0, 1))
    );

    // SE child of tile(0,0,0) = (1,1,1). East should be SW child of tile(1,0,0) = (2,1,1)
    let se_of_first = TileCoord::new(1, 1, 1);
    assert_eq!(
        find_tile_to_east(&scheme, &tiles, &se_of_first),
        Some(TileCoord::new(2, 1, 1))
    );

    // NW child of tile(1,0,0) = (2,0,1). West should be NE child of tile(0,0,0) = (1,0,1)
    let nw_of_second = TileCoord::new(2, 0, 1);
    assert_eq!(
        find_tile_to_west(&scheme, &tiles, &nw_of_second),
        Some(TileCoord::new(1, 0, 1))
    );

    // SW child of tile(1,0,0) = (2,1,1). West should be SE child of tile(0,0,0) = (1,1,1)
    let sw_of_second = TileCoord::new(2, 1, 1);
    assert_eq!(
        find_tile_to_west(&scheme, &tiles, &sw_of_second),
        Some(TileCoord::new(1, 1, 1))
    );
}

// ─── Deep nesting (level 2) ─────────────────────────────────────────────────

#[test]
fn adjacency_works_at_deeper_levels() {
    let scheme = TilingSchemeDescriptor::new(2, 1);
    let tiles = create_level_zero_tiles(&scheme);

    // Level 2: children of (0,0,1) are (0,0,2), (1,0,2), (0,1,2), (1,1,2)
    let nw_l2 = TileCoord::new(0, 0, 2);
    let ne_l2 = TileCoord::new(1, 0, 2);

    // Same parent adjacency
    assert_eq!(find_tile_to_east(&scheme, &tiles, &nw_l2), Some(ne_l2));
    assert_eq!(find_tile_to_west(&scheme, &tiles, &ne_l2), Some(nw_l2));

    // Cross-parent: NE of (0,0,1) is (1,0,1). Its NW child is (2,0,2).
    // So east of (1,0,2) [NE child of (0,0,1)] should go up to parent (0,0,1),
    // find east of parent = (1,0,1), then take NW child = (2,0,2)
    assert_eq!(
        find_tile_to_east(&scheme, &tiles, &ne_l2),
        Some(TileCoord::new(2, 0, 2))
    );
}

// ─── Geographic tiling scheme (2x1) ─────────────────────────────────────────

#[test]
fn geographic_scheme_level_zero() {
    let scheme = TilingSchemeDescriptor::geographic();
    let tiles = create_level_zero_tiles(&scheme);
    assert_eq!(tiles.len(), 2);
    assert_eq!(tiles[0], TileCoord::new(0, 0, 0));
    assert_eq!(tiles[1], TileCoord::new(1, 0, 0));
}

#[test]
fn web_mercator_scheme_level_zero() {
    let scheme = TilingSchemeDescriptor::web_mercator();
    let tiles = create_level_zero_tiles(&scheme);
    assert_eq!(tiles.len(), 1);
    assert_eq!(tiles[0], TileCoord::new(0, 0, 0));
}
