//! QuadtreeTile adjacency: find neighboring tiles in a quadtree.
//!
//! Maps to CesiumJS `Scene/QuadtreeTile.js` adjacency methods:
//! - `createLevelZeroTiles`
//! - `findTileToWest/East/North/South`
//! - `findLevelZeroTile`

/// A tile coordinate in the quadtree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    /// Tile X coordinate.
    pub x: u32,
    /// Tile Y coordinate.
    pub y: u32,
    /// Level of detail (0 = coarsest).
    pub level: u32,
}

impl TileCoord {
    /// Creates a new tile coordinate.
    pub fn new(x: u32, y: u32, level: u32) -> Self {
        Self { x, y, level }
    }

    /// Returns the parent coordinate (None if level 0).
    pub fn parent(&self) -> Option<TileCoord> {
        if self.level == 0 {
            None
        } else {
            Some(TileCoord {
                x: self.x / 2,
                y: self.y / 2,
                level: self.level - 1,
            })
        }
    }

    /// Returns the northwest child (x*2, y*2, level+1).
    pub fn northwest_child(&self) -> TileCoord {
        TileCoord::new(self.x * 2, self.y * 2, self.level + 1)
    }

    /// Returns the northeast child (x*2+1, y*2, level+1).
    pub fn northeast_child(&self) -> TileCoord {
        TileCoord::new(self.x * 2 + 1, self.y * 2, self.level + 1)
    }

    /// Returns the southwest child (x*2, y*2+1, level+1).
    pub fn southwest_child(&self) -> TileCoord {
        TileCoord::new(self.x * 2, self.y * 2 + 1, self.level + 1)
    }

    /// Returns the southeast child (x*2+1, y*2+1, level+1).
    pub fn southeast_child(&self) -> TileCoord {
        TileCoord::new(self.x * 2 + 1, self.y * 2 + 1, self.level + 1)
    }

    /// Determines which child position this tile is relative to its parent.
    /// Returns None if level == 0.
    fn child_position(&self) -> Option<ChildPosition> {
        if self.level == 0 {
            return None;
        }
        let is_east = self.x % 2 == 1;
        let is_south = self.y % 2 == 1;
        Some(match (is_east, is_south) {
            (false, false) => ChildPosition::Northwest,
            (true, false) => ChildPosition::Northeast,
            (false, true) => ChildPosition::Southwest,
            (true, true) => ChildPosition::Southeast,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildPosition {
    Northwest,
    Northeast,
    Southwest,
    Southeast,
}

/// A tiling scheme descriptor for adjacency calculations.
#[derive(Debug, Clone)]
pub struct TilingSchemeDescriptor {
    /// Number of tiles in X direction at level 0.
    pub x_tiles_at_level_zero: u32,
    /// Number of tiles in Y direction at level 0.
    pub y_tiles_at_level_zero: u32,
}

impl TilingSchemeDescriptor {
    /// Creates a new tiling scheme descriptor.
    pub fn new(x_tiles: u32, y_tiles: u32) -> Self {
        Self {
            x_tiles_at_level_zero: x_tiles,
            y_tiles_at_level_zero: y_tiles,
        }
    }

    /// Geographic tiling scheme (2x1 at level 0).
    pub fn geographic() -> Self {
        Self::new(2, 1)
    }

    /// Web Mercator tiling scheme (1x1 at level 0).
    pub fn web_mercator() -> Self {
        Self::new(1, 1)
    }
}

/// Creates level zero tiles for a given tiling scheme.
///
/// Returns tiles ordered from northwest, proceeding east then south.
///
/// Maps to `QuadtreeTile.createLevelZeroTiles`.
pub fn create_level_zero_tiles(scheme: &TilingSchemeDescriptor) -> Vec<TileCoord> {
    let mut result = Vec::with_capacity(
        (scheme.x_tiles_at_level_zero * scheme.y_tiles_at_level_zero) as usize,
    );
    for y in 0..scheme.y_tiles_at_level_zero {
        for x in 0..scheme.x_tiles_at_level_zero {
            result.push(TileCoord::new(x, y, 0));
        }
    }
    result
}

/// Finds the level-zero tile at the given coordinates, wrapping X around the anti-meridian.
///
/// Returns None if Y is out of bounds (north of north pole or south of south pole).
///
/// Maps to `QuadtreeTile.findLevelZeroTile`.
pub fn find_level_zero_tile(
    scheme: &TilingSchemeDescriptor,
    level_zero_tiles: &[TileCoord],
    x: i32,
    y: i32,
) -> Option<TileCoord> {
    let x_tiles = scheme.x_tiles_at_level_zero as i32;
    let y_tiles = scheme.y_tiles_at_level_zero as i32;

    let mut wrapped_x = x;
    if wrapped_x < 0 {
        wrapped_x += x_tiles;
    } else if wrapped_x >= x_tiles {
        wrapped_x -= x_tiles;
    }

    if y < 0 || y >= y_tiles {
        return None;
    }

    level_zero_tiles
        .iter()
        .find(|t| t.x == wrapped_x as u32 && t.y == y as u32)
        .copied()
}

/// Finds the tile to the west of the given tile.
///
/// Maps to `QuadtreeTile.findTileToWest`.
pub fn find_tile_to_west(
    scheme: &TilingSchemeDescriptor,
    level_zero_tiles: &[TileCoord],
    tile: &TileCoord,
) -> Option<TileCoord> {
    let parent = match tile.parent() {
        None => {
            return find_level_zero_tile(scheme, level_zero_tiles, tile.x as i32 - 1, tile.y as i32)
        }
        Some(p) => p,
    };

    match tile.child_position() {
        Some(ChildPosition::Southeast) => Some(parent.southwest_child()),
        Some(ChildPosition::Northeast) => Some(parent.northwest_child()),
        Some(ChildPosition::Southwest) | Some(ChildPosition::Northwest) => {
            let west_of_parent = find_tile_to_west(scheme, level_zero_tiles, &parent)?;
            match tile.child_position() {
                Some(ChildPosition::Southwest) => Some(west_of_parent.southeast_child()),
                _ => Some(west_of_parent.northeast_child()),
            }
        }
        None => unreachable!(),
    }
}

/// Finds the tile to the east of the given tile.
///
/// Maps to `QuadtreeTile.findTileToEast`.
pub fn find_tile_to_east(
    scheme: &TilingSchemeDescriptor,
    level_zero_tiles: &[TileCoord],
    tile: &TileCoord,
) -> Option<TileCoord> {
    let parent = match tile.parent() {
        None => {
            return find_level_zero_tile(scheme, level_zero_tiles, tile.x as i32 + 1, tile.y as i32)
        }
        Some(p) => p,
    };

    match tile.child_position() {
        Some(ChildPosition::Southwest) => Some(parent.southeast_child()),
        Some(ChildPosition::Northwest) => Some(parent.northeast_child()),
        Some(ChildPosition::Southeast) | Some(ChildPosition::Northeast) => {
            let east_of_parent = find_tile_to_east(scheme, level_zero_tiles, &parent)?;
            match tile.child_position() {
                Some(ChildPosition::Southeast) => Some(east_of_parent.southwest_child()),
                _ => Some(east_of_parent.northwest_child()),
            }
        }
        None => unreachable!(),
    }
}

/// Finds the tile to the south of the given tile.
///
/// Maps to `QuadtreeTile.findTileToSouth`.
pub fn find_tile_to_south(
    scheme: &TilingSchemeDescriptor,
    level_zero_tiles: &[TileCoord],
    tile: &TileCoord,
) -> Option<TileCoord> {
    let parent = match tile.parent() {
        None => {
            return find_level_zero_tile(scheme, level_zero_tiles, tile.x as i32, tile.y as i32 + 1)
        }
        Some(p) => p,
    };

    match tile.child_position() {
        Some(ChildPosition::Northwest) => Some(parent.southwest_child()),
        Some(ChildPosition::Northeast) => Some(parent.southeast_child()),
        Some(ChildPosition::Southwest) | Some(ChildPosition::Southeast) => {
            let south_of_parent = find_tile_to_south(scheme, level_zero_tiles, &parent)?;
            match tile.child_position() {
                Some(ChildPosition::Southwest) => Some(south_of_parent.northwest_child()),
                _ => Some(south_of_parent.northeast_child()),
            }
        }
        None => unreachable!(),
    }
}

/// Finds the tile to the north of the given tile.
///
/// Maps to `QuadtreeTile.findTileToNorth`.
pub fn find_tile_to_north(
    scheme: &TilingSchemeDescriptor,
    level_zero_tiles: &[TileCoord],
    tile: &TileCoord,
) -> Option<TileCoord> {
    let parent = match tile.parent() {
        None => {
            return find_level_zero_tile(scheme, level_zero_tiles, tile.x as i32, tile.y as i32 - 1)
        }
        Some(p) => p,
    };

    match tile.child_position() {
        Some(ChildPosition::Southwest) => Some(parent.northwest_child()),
        Some(ChildPosition::Southeast) => Some(parent.northeast_child()),
        Some(ChildPosition::Northwest) | Some(ChildPosition::Northeast) => {
            let north_of_parent = find_tile_to_north(scheme, level_zero_tiles, &parent)?;
            match tile.child_position() {
                Some(ChildPosition::Northwest) => Some(north_of_parent.southwest_child()),
                _ => Some(north_of_parent.southeast_child()),
            }
        }
        None => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_level_zero_tiles_geographic() {
        let scheme = TilingSchemeDescriptor::new(2, 1);
        let tiles = create_level_zero_tiles(&scheme);
        assert_eq!(tiles.len(), 2);
        assert_eq!(tiles[0], TileCoord::new(0, 0, 0));
        assert_eq!(tiles[1], TileCoord::new(1, 0, 0));
    }

    #[test]
    fn test_create_level_zero_tiles_3x3() {
        let scheme = TilingSchemeDescriptor::new(3, 3);
        let tiles = create_level_zero_tiles(&scheme);
        assert_eq!(tiles.len(), 9);
        // Ordered NW→E→S
        assert_eq!(tiles[0], TileCoord::new(0, 0, 0));
        assert_eq!(tiles[1], TileCoord::new(1, 0, 0));
        assert_eq!(tiles[2], TileCoord::new(2, 0, 0));
        assert_eq!(tiles[3], TileCoord::new(0, 1, 0));
    }

    #[test]
    fn test_find_level_zero_tile_wraps_x() {
        let scheme = TilingSchemeDescriptor::new(3, 3);
        let tiles = create_level_zero_tiles(&scheme);
        // x=-1 wraps to x=2
        let found = find_level_zero_tile(&scheme, &tiles, -1, 0);
        assert_eq!(found, Some(TileCoord::new(2, 0, 0)));
        // x=3 wraps to x=0
        let found = find_level_zero_tile(&scheme, &tiles, 3, 0);
        assert_eq!(found, Some(TileCoord::new(0, 0, 0)));
    }

    #[test]
    fn test_find_level_zero_tile_y_out_of_bounds() {
        let scheme = TilingSchemeDescriptor::new(3, 3);
        let tiles = create_level_zero_tiles(&scheme);
        assert_eq!(find_level_zero_tile(&scheme, &tiles, 0, -1), None);
        assert_eq!(find_level_zero_tile(&scheme, &tiles, 0, 3), None);
    }

    #[test]
    fn test_adjacency_level_zero() {
        let scheme = TilingSchemeDescriptor::new(3, 3);
        let tiles = create_level_zero_tiles(&scheme);
        let tile = TileCoord::new(0, 0, 0);

        // West wraps around
        assert_eq!(
            find_tile_to_west(&scheme, &tiles, &tile),
            Some(TileCoord::new(2, 0, 0))
        );
        // East
        assert_eq!(
            find_tile_to_east(&scheme, &tiles, &tile),
            Some(TileCoord::new(1, 0, 0))
        );
        // North of row 0 → None
        assert_eq!(find_tile_to_north(&scheme, &tiles, &tile), None);
        // South
        assert_eq!(
            find_tile_to_south(&scheme, &tiles, &tile),
            Some(TileCoord::new(0, 1, 0))
        );
    }
}
