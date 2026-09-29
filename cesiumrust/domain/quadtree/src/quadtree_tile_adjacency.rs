//! QuadtreeTile 邻接：在四叉树中查找相邻瓦片。
//!
//! 映射到 CesiumJS `Scene/QuadtreeTile.js` 的邻接方法：
//! - `createLevelZeroTiles`
//! - `findTileToWest/East/North/South`
//! - `findLevelZeroTile`

/// 四叉树中的一个瓦片坐标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileCoord {
    /// 瓦片 X 坐标。
    pub x: u32,
    /// 瓦片 Y 坐标。
    pub y: u32,
    /// 细节层级（0 = 最粗）。
    pub level: u32,
}

impl TileCoord {
    /// 创建一个新的瓦片坐标。
    pub fn new(x: u32, y: u32, level: u32) -> Self {
        Self { x, y, level }
    }

    /// 返回父坐标（若为层级 0 则返回 None）。
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

    /// 返回西北子节点（x*2, y*2, level+1）。
    pub fn northwest_child(&self) -> TileCoord {
        TileCoord::new(self.x * 2, self.y * 2, self.level + 1)
    }

    /// 返回东北子节点（x*2+1, y*2, level+1）。
    pub fn northeast_child(&self) -> TileCoord {
        TileCoord::new(self.x * 2 + 1, self.y * 2, self.level + 1)
    }

    /// 返回西南子节点（x*2, y*2+1, level+1）。
    pub fn southwest_child(&self) -> TileCoord {
        TileCoord::new(self.x * 2, self.y * 2 + 1, self.level + 1)
    }

    /// 返回东南子节点（x*2+1, y*2+1, level+1）。
    pub fn southeast_child(&self) -> TileCoord {
        TileCoord::new(self.x * 2 + 1, self.y * 2 + 1, self.level + 1)
    }

    /// 判断本瓦片相对于其父节点处于哪个子位置。
    /// 若 level == 0 则返回 None。
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

/// 用于邻接计算的切分方案描述符。
#[derive(Debug, Clone)]
pub struct TilingSchemeDescriptor {
    /// 层级 0 时 X 方向的瓦片数。
    pub x_tiles_at_level_zero: u32,
    /// 层级 0 时 Y 方向的瓦片数。
    pub y_tiles_at_level_zero: u32,
}

impl TilingSchemeDescriptor {
    /// 创建一个新的切分方案描述符。
    pub fn new(x_tiles: u32, y_tiles: u32) -> Self {
        Self {
            x_tiles_at_level_zero: x_tiles,
            y_tiles_at_level_zero: y_tiles,
        }
    }

    /// 地理（Geographic）切分方案（层级 0 为 2x1）。
    pub fn geographic() -> Self {
        Self::new(2, 1)
    }

    /// Web Mercator 切分方案（层级 0 为 1x1）。
    pub fn web_mercator() -> Self {
        Self::new(1, 1)
    }
}

/// 为给定切分方案创建层级零的瓦片。
///
/// 返回的瓦片从西北开始排序，先向东再向南。
///
/// 映射到 `QuadtreeTile.createLevelZeroTiles`。
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

/// 在给定坐标处查找层级零瓦片，X 沿反日子线环绕。
///
/// 若 Y 越界（北极以北或南极以南）则返回 None。
///
/// 映射到 `QuadtreeTile.findLevelZeroTile`。
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

/// 查找给定瓦片西侧的瓦片。
///
/// 映射到 `QuadtreeTile.findTileToWest`。
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

/// 查找给定瓦片东侧的瓦片。
///
/// 映射到 `QuadtreeTile.findTileToEast`。
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

/// 查找给定瓦片南侧的瓦片。
///
/// 映射到 `QuadtreeTile.findTileToSouth`。
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

/// 查找给定瓦片北侧的瓦片。
///
/// 映射到 `QuadtreeTile.findTileToNorth`。
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
        // 按 NW→E→S 排序
        assert_eq!(tiles[0], TileCoord::new(0, 0, 0));
        assert_eq!(tiles[1], TileCoord::new(1, 0, 0));
        assert_eq!(tiles[2], TileCoord::new(2, 0, 0));
        assert_eq!(tiles[3], TileCoord::new(0, 1, 0));
    }

    #[test]
    fn test_find_level_zero_tile_wraps_x() {
        let scheme = TilingSchemeDescriptor::new(3, 3);
        let tiles = create_level_zero_tiles(&scheme);
        // x=-1 环绕为 x=2
        let found = find_level_zero_tile(&scheme, &tiles, -1, 0);
        assert_eq!(found, Some(TileCoord::new(2, 0, 0)));
        // x=3 环绕为 x=0
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

        // 西侧环绕
        assert_eq!(
            find_tile_to_west(&scheme, &tiles, &tile),
            Some(TileCoord::new(2, 0, 0))
        );
        // 东
        assert_eq!(
            find_tile_to_east(&scheme, &tiles, &tile),
            Some(TileCoord::new(1, 0, 0))
        );
        // 第 0 行的北侧 → None
        assert_eq!(find_tile_to_north(&scheme, &tiles, &tile), None);
        // 南
        assert_eq!(
            find_tile_to_south(&scheme, &tiles, &tile),
            Some(TileCoord::new(0, 1, 0))
        );
    }
}
