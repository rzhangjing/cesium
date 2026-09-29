//! 3D Tiles 1.1 的隐式切分。
//!
//! 映射到 CesiumJS `Scene/Implicit3DTileContent.js`：
//! - Quadtree/Octree 隐式细分
//! - 可用性位流（availability bitstreams）
//! - Morton 索引计算
//! - 子树文件解析

// 沿自 CesiumJS 移植的 legacy 风格债（deferred.md #18）；在 M13 lint-cleanup 或本文件在其里程碑被重写时重新检视
#![allow(clippy::manual_is_multiple_of)]
/// 隐式切分的细分方案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubdivisionScheme {
    /// Quadtree（2D 细分）。
    #[default]
    Quadtree,
    /// Octree（3D 细分）。
    Octree,
}

impl SubdivisionScheme {
    /// 返回每个节点的子节点数。
    pub fn branching_factor(&self) -> u32 {
        match self {
            Self::Quadtree => 4,
            Self::Octree => 8,
        }
    }

    /// 返回维度数。
    pub fn dimensions(&self) -> u32 {
        match self {
            Self::Quadtree => 2,
            Self::Octree => 3,
        }
    }
}

/// 隐式切分级层中的一个瓦片坐标。
/// 映射到 CesiumJS `Scene/ImplicitTileCoordinates.js`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImplicitTileCoord {
    /// 树中的层级（0 = 根）。
    pub level: u32,
    /// 该层级的 X 坐标。
    pub x: u32,
    /// 该层级的 Y 坐标。
    pub y: u32,
    /// 该层级的 Z 坐标（仅 octree）。
    pub z: u32,
    /// 该坐标所在子树内的不同层级数。
    pub subtree_levels: u32,
}

impl ImplicitTileCoord {
    /// 创建一个新的 quadtree 坐标，默认 subtree_levels=2。
    pub fn quadtree(level: u32, x: u32, y: u32) -> Self {
        Self { level, x, y, z: 0, subtree_levels: 2 }
    }

    /// 创建一个新的 quadtree 坐标，显式指定 subtree_levels。
    pub fn quadtree_with_subtree(level: u32, x: u32, y: u32, subtree_levels: u32) -> Self {
        Self { level, x, y, z: 0, subtree_levels }
    }

    /// 创建一个新的 octree 坐标，默认 subtree_levels=2。
    pub fn octree(level: u32, x: u32, y: u32, z: u32) -> Self {
        Self { level, x, y, z, subtree_levels: 2 }
    }

    /// 创建一个新的 octree 坐标，显式指定 subtree_levels。
    pub fn octree_with_subtree(level: u32, x: u32, y: u32, z: u32, subtree_levels: u32) -> Self {
        Self { level, x, y, z, subtree_levels }
    }

    /// 计算该坐标的 Morton 索引。
    pub fn morton_index(&self, scheme: SubdivisionScheme) -> u64 {
        match scheme {
            SubdivisionScheme::Quadtree => morton_2d(self.x, self.y),
            SubdivisionScheme::Octree => morton_3d(self.x, self.y, self.z),
        }
    }

    /// 返回父坐标。
    pub fn parent(&self) -> Option<Self> {
        if self.level == 0 {
            return None;
        }
        Some(Self {
            level: self.level - 1,
            x: self.x / 2,
            y: self.y / 2,
            z: self.z / 2,
            subtree_levels: self.subtree_levels,
        })
    }

    /// 返回子坐标。
    pub fn children(&self, scheme: SubdivisionScheme) -> Vec<Self> {
        let child_level = self.level + 1;
        let bx = self.x * 2;
        let by = self.y * 2;
        let bz = self.z * 2;

        match scheme {
            SubdivisionScheme::Quadtree => vec![
                Self::quadtree_with_subtree(child_level, bx, by, self.subtree_levels),
                Self::quadtree_with_subtree(child_level, bx + 1, by, self.subtree_levels),
                Self::quadtree_with_subtree(child_level, bx, by + 1, self.subtree_levels),
                Self::quadtree_with_subtree(child_level, bx + 1, by + 1, self.subtree_levels),
            ],
            SubdivisionScheme::Octree => vec![
                Self::octree_with_subtree(child_level, bx, by, bz, self.subtree_levels),
                Self::octree_with_subtree(child_level, bx + 1, by, bz, self.subtree_levels),
                Self::octree_with_subtree(child_level, bx, by + 1, bz, self.subtree_levels),
                Self::octree_with_subtree(child_level, bx + 1, by + 1, bz, self.subtree_levels),
                Self::octree_with_subtree(child_level, bx, by, bz + 1, self.subtree_levels),
                Self::octree_with_subtree(child_level, bx + 1, by, bz + 1, self.subtree_levels),
                Self::octree_with_subtree(child_level, bx, by + 1, bz + 1, self.subtree_levels),
                Self::octree_with_subtree(child_level, bx + 1, by + 1, bz + 1, self.subtree_levels),
            ],
        }
    }

    /// 返回该层级的瓦片数。
    pub fn tiles_at_level(level: u32, scheme: SubdivisionScheme) -> u64 {
        let per_dim = 1u64 << level;
        match scheme {
            SubdivisionScheme::Quadtree => per_dim * per_dim,
            SubdivisionScheme::Octree => per_dim * per_dim * per_dim,
        }
    }

    // ========================================================================
    // ImplicitTileCoordinates 方法（沿自 CesiumJS）
    // ========================================================================

    /// 计算子索引（本瓦片是父节点的哪个孩子）。
    /// 映射到 `ImplicitTileCoordinates.childIndex`
    pub fn child_index(&self, scheme: SubdivisionScheme) -> u32 {
        let mut idx = 0u32;
        idx |= self.x & 1;
        idx |= (self.y & 1) << 1;
        if scheme == SubdivisionScheme::Octree {
            idx |= (self.z & 1) << 2;
        }
        idx
    }

    /// 计算瓦片索引（层级偏移 + morton 索引）。
    /// 映射到 `ImplicitTileCoordinates.tileIndex`
    pub fn tile_index(&self, scheme: SubdivisionScheme) -> u64 {
        let level_offset = match scheme {
            SubdivisionScheme::Octree => ((1u64 << (3 * self.level)) - 1) / 7,
            SubdivisionScheme::Quadtree => ((1u64 << (2 * self.level)) - 1) / 3,
        };
        level_offset + self.morton_index(scheme)
    }

    /// 给定一个相对偏移，计算后代坐标。
    /// 映射到 `ImplicitTileCoordinates.getDescendantCoordinates`
    pub fn get_descendant_coordinates(&self, offset: &ImplicitTileCoord) -> Self {
        let descendant_level = self.level + offset.level;
        let descendant_x = (self.x << offset.level) + offset.x;
        let descendant_y = (self.y << offset.level) + offset.y;
        let descendant_z = (self.z << offset.level) + offset.z;
        Self {
            level: descendant_level,
            x: descendant_x,
            y: descendant_y,
            z: descendant_z,
            subtree_levels: self.subtree_levels,
        }
    }

    /// 向上跨越若干层级来计算祖先坐标。
    /// 映射到 `ImplicitTileCoordinates.getAncestorCoordinates`
    pub fn get_ancestor_coordinates(&self, offset_levels: u32) -> Self {
        let divisor = 1u32 << offset_levels;
        Self {
            level: self.level - offset_levels,
            x: self.x / divisor,
            y: self.y / divisor,
            z: self.z / divisor,
            subtree_levels: self.subtree_levels,
        }
    }

    /// 计算从本祖先到某个后代的偏移。
    /// 映射到 `ImplicitTileCoordinates.getOffsetCoordinates`
    pub fn get_offset_coordinates(&self, descendant: &ImplicitTileCoord) -> Self {
        let offset_level = descendant.level - self.level;
        let dimension_at_offset = 1u32 << offset_level;
        Self {
            level: offset_level,
            x: descendant.x % dimension_at_offset,
            y: descendant.y % dimension_at_offset,
            z: descendant.z % dimension_at_offset,
            subtree_levels: self.subtree_levels,
        }
    }

    /// 由子索引（父节点内的 morton 索引）获取子坐标。
    /// 映射到 `ImplicitTileCoordinates.getChildCoordinates`
    pub fn get_child_coordinates(&self, child_index: u32) -> Self {
        let level = self.level + 1;
        let x = 2 * self.x + (child_index % 2);
        let y = 2 * self.y + ((child_index / 2) % 2);
        let z = 2 * self.z + ((child_index / 4) % 2);
        Self {
            level,
            x,
            y,
            z,
            subtree_levels: self.subtree_levels,
        }
    }

    /// 获取包含本瓦片的子树根的坐标。
    /// 映射到 `ImplicitTileCoordinates.getSubtreeCoordinates`
    pub fn get_subtree_coordinates(&self) -> Self {
        self.get_ancestor_coordinates(self.level % self.subtree_levels)
    }

    /// 获取包含本瓦片的父子树的坐标。
    /// 映射到 `ImplicitTileCoordinates.getParentSubtreeCoordinates`
    pub fn get_parent_subtree_coordinates(&self) -> Self {
        self.get_ancestor_coordinates((self.level % self.subtree_levels) + self.subtree_levels)
    }

    /// 返回本瓦片是否为另一瓦片的祖先。
    /// 映射到 `ImplicitTileCoordinates.isAncestor`
    pub fn is_ancestor(&self, descendant: &ImplicitTileCoord, scheme: SubdivisionScheme) -> bool {
        let level_diff = descendant.level as i32 - self.level as i32;
        if level_diff <= 0 {
            return false;
        }
        let shift = level_diff as u32;
        let ancestor_x = descendant.x >> shift;
        let ancestor_y = descendant.y >> shift;
        let is_ancestor_xy = self.x == ancestor_x && self.y == ancestor_y;
        if scheme == SubdivisionScheme::Octree {
            let ancestor_z = descendant.z >> shift;
            is_ancestor_xy && self.z == ancestor_z
        } else {
            is_ancestor_xy
        }
    }

    /// 返回本瓦片是否为隐式瓦片集的根（层级 0）。
    /// 映射到 `ImplicitTileCoordinates.isImplicitTilesetRoot`
    pub fn is_implicit_tileset_root(&self) -> bool {
        self.level == 0
    }

    /// 返回本瓦片是否为某个子树的根。
    /// 映射到 `ImplicitTileCoordinates.isSubtreeRoot`
    pub fn is_subtree_root(&self) -> bool {
        self.level % self.subtree_levels == 0
    }

    /// 返回本瓦片是否位于其子树的最后一层。
    /// 映射到 `ImplicitTileCoordinates.isBottomOfSubtree`
    pub fn is_bottom_of_subtree(&self) -> bool {
        self.level % self.subtree_levels == self.subtree_levels - 1
    }

    /// 由给定层级的 Morton 索引创建坐标。
    /// 映射到 `ImplicitTileCoordinates.fromMortonIndex`
    pub fn from_morton_index(
        scheme: SubdivisionScheme,
        subtree_levels: u32,
        level: u32,
        morton_index: u64,
    ) -> Self {
        match scheme {
            SubdivisionScheme::Octree => {
                let (x, y, z) = decode_morton_3d(morton_index);
                Self::octree_with_subtree(level, x, y, z, subtree_levels)
            }
            SubdivisionScheme::Quadtree => {
                let (x, y) = decode_morton_2d(morton_index);
                Self::quadtree_with_subtree(level, x, y, subtree_levels)
            }
        }
    }

    /// 将 URI 中的模板占位符替换为坐标值。
    /// 将 `{level}`、`{x}`、`{y}`、`{z}` 替换为实际的坐标值。
    /// 映射到 `ImplicitTileCoordinates.getTemplateValues`
    pub fn get_template_values(&self, template_uri: &str) -> String {
        template_uri
            .replace("{level}", &self.level.to_string())
            .replace("{x}", &self.x.to_string())
            .replace("{y}", &self.y.to_string())
            .replace("{z}", &self.z.to_string())
    }

    /// 由瓦片索引创建坐标。
    /// 映射到 `ImplicitTileCoordinates.fromTileIndex`
    pub fn from_tile_index(
        scheme: SubdivisionScheme,
        subtree_levels: u32,
        tile_index: u64,
    ) -> Self {
        let level;
        let level_offset;
        let morton_index;

        match scheme {
            SubdivisionScheme::Octree => {
                // L = floor(log2(7*tileIndex + 1) / 3)
                level = ((7 * tile_index + 1) as f64).log2().floor() as u32 / 3;
                level_offset = ((1u64 << (3 * level)) - 1) / 7;
                morton_index = tile_index - level_offset;
            }
            SubdivisionScheme::Quadtree => {
                // L = floor(log2(3*tileIndex + 1) / 2)
                level = ((3 * tile_index + 1) as f64).log2().floor() as u32 / 2;
                level_offset = ((1u64 << (2 * level)) - 1) / 3;
                morton_index = tile_index - level_offset;
            }
        }

        Self::from_morton_index(scheme, subtree_levels, level, morton_index)
    }
}

/// 计算 2D Morton 码（Z 阶曲线）。
/// CesiumJS 约定：x 占偶数位（0,2,4...），y 占奇数位（1,3,5...）。
pub fn morton_2d(x: u32, y: u32) -> u64 {
    (part1by1(y as u64) << 1) | part1by1(x as u64)
}

/// 计算 3D Morton 码。
/// CesiumJS 约定：x 占位置 0,3,6...，y 占 1,4,7...，z 占 2,5,8...
pub fn morton_3d(x: u32, y: u32, z: u32) -> u64 {
    (part1by2(z as u64) << 2) | (part1by2(y as u64) << 1) | part1by2(x as u64)
}

/// 为 2D Morton 码展开位。
fn part1by1(mut n: u64) -> u64 {
    n &= 0x0000_0000_ffff_ffff;
    n = (n | (n << 16)) & 0x0000_ffff_0000_ffff;
    n = (n | (n << 8)) & 0x00ff_00ff_00ff_00ff;
    n = (n | (n << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    n = (n | (n << 2)) & 0x3333_3333_3333_3333;
    n = (n | (n << 1)) & 0x5555_5555_5555_5555;
    n
}

/// 为 3D Morton 码展开位。
fn part1by2(mut n: u64) -> u64 {
    n &= 0x0000_0000_001f_ffff;
    n = (n | (n << 32)) & 0x001f_0000_0000_ffff;
    n = (n | (n << 16)) & 0x001f_0000_ff00_00ff;
    n = (n | (n << 8)) & 0x100f_00f0_0f00_f00f;
    n = (n | (n << 4)) & 0x10c3_0c30_c30c_30c3;
    n = (n | (n << 2)) & 0x1249_2492_4924_9249;
    n
}

/// 将一个 2D Morton 索引解码为 (x, y) 坐标。
/// x 取自偶数位，y 取自奇数位（CesiumJS 约定）。
pub fn decode_morton_2d(morton: u64) -> (u32, u32) {
    let x = compact1by1(morton) as u32;
    let y = compact1by1(morton >> 1) as u32;
    (x, y)
}

/// 将一个 3D Morton 索引解码为 (x, y, z) 坐标。
/// x 取自位置 0,3,6...，y 取自 1,4,7...，z 取自 2,5,8...（CesiumJS 约定）。
pub fn decode_morton_3d(morton: u64) -> (u32, u32, u32) {
    let x = compact1by2(morton) as u32;
    let y = compact1by2(morton >> 1) as u32;
    let z = compact1by2(morton >> 2) as u32;
    (x, y, z)
}

/// 为 2D Morton 解码压缩位（part1by1 的逆运算）。
fn compact1by1(mut n: u64) -> u64 {
    n &= 0x5555_5555_5555_5555;
    n = (n ^ (n >> 1)) & 0x3333_3333_3333_3333;
    n = (n ^ (n >> 2)) & 0x0f0f_0f0f_0f0f_0f0f;
    n = (n ^ (n >> 4)) & 0x00ff_00ff_00ff_00ff;
    n = (n ^ (n >> 8)) & 0x0000_ffff_0000_ffff;
    n = (n ^ (n >> 16)) & 0x0000_0000_ffff_ffff;
    n
}

/// 为 3D Morton 解码压缩位（part1by2 的逆运算）。
fn compact1by2(mut n: u64) -> u64 {
    n &= 0x1249_2492_4924_9249;
    n = (n ^ (n >> 2)) & 0x10c3_0c30_c30c_30c3;
    n = (n ^ (n >> 4)) & 0x100f_00f0_0f00_f00f;
    n = (n ^ (n >> 8)) & 0x001f_0000_ff00_00ff;
    n = (n ^ (n >> 16)) & 0x001f_0000_0000_ffff;
    n = (n ^ (n >> 32)) & 0x0000_0000_001f_ffff;
    n
}

/// 隐式瓦片的可用性位流。
#[derive(Debug, Clone)]
pub struct AvailabilityBitstream {
    /// 位数据（每字节内 LSB 优先）。
    pub bits: Vec<u8>,
    /// 有效位数。
    pub length: u64,
}

impl AvailabilityBitstream {
    /// 创建一个所有位均未置 1 的新位流。
    pub fn new(length: u64) -> Self {
        let byte_count = length.div_ceil(8) as usize;
        Self {
            bits: vec![0u8; byte_count],
            length,
        }
    }

    /// 由原始字节创建。
    pub fn from_bytes(bits: Vec<u8>, length: u64) -> Self {
        Self { bits, length }
    }

    /// 若 index 处的位已置 1 则返回 true。
    pub fn is_available(&self, index: u64) -> bool {
        if index >= self.length {
            return false;
        }
        let byte_index = (index / 8) as usize;
        let bit_index = (index % 8) as u8;
        (self.bits[byte_index] >> bit_index) & 1 == 1
    }

    /// 设置 index 处的位。
    pub fn set(&mut self, index: u64, available: bool) {
        if index >= self.length {
            return;
        }
        let byte_index = (index / 8) as usize;
        let bit_index = (index % 8) as u8;
        if available {
            self.bits[byte_index] |= 1 << bit_index;
        } else {
            self.bits[byte_index] &= !(1 << bit_index);
        }
    }

    /// 返回可用瓦片的数量。
    pub fn count_available(&self) -> u64 {
        let mut count = 0u64;
        for i in 0..self.length {
            if self.is_available(i) {
                count += 1;
            }
        }
        count
    }
}

/// 来自 tileset.json 的隐式切分配置。
#[derive(Debug, Clone)]
pub struct ImplicitTilingConfig {
    /// 细分方案。
    pub subdivision_scheme: SubdivisionScheme,
    /// 每个子树的层数。
    pub subtree_levels: u32,
    /// 树的最大层数。
    pub maximum_level: u32,
    /// 子树文件的 URL 模板。
    pub subtree_uri_template: String,
    /// 内容文件的 URL 模板。
    pub content_uri_template: String,
}

impl ImplicitTilingConfig {
    /// 为给定坐标生成一个子树 URI。
    pub fn get_subtree_uri(&self, coord: &ImplicitTileCoord) -> String {
        self.subtree_uri_template
            .replace("{level}", &coord.level.to_string())
            .replace("{x}", &coord.x.to_string())
            .replace("{y}", &coord.y.to_string())
            .replace("{z}", &coord.z.to_string())
    }

    /// 为给定坐标生成一个内容 URI。
    pub fn get_content_uri(&self, coord: &ImplicitTileCoord) -> String {
        self.content_uri_template
            .replace("{level}", &coord.level.to_string())
            .replace("{x}", &coord.x.to_string())
            .replace("{y}", &coord.y.to_string())
            .replace("{z}", &coord.z.to_string())
    }

    /// 计算某个瓦片的子树根坐标。
    pub fn get_subtree_root(&self, coord: &ImplicitTileCoord) -> ImplicitTileCoord {
        let subtree_level = (coord.level / self.subtree_levels) * self.subtree_levels;
        let level_diff = coord.level - subtree_level;
        ImplicitTileCoord {
            level: subtree_level,
            x: coord.x >> level_diff,
            y: coord.y >> level_diff,
            z: coord.z >> level_diff,
            subtree_levels: self.subtree_levels,
        }
    }
}

/// 一个已解析的子树文件。
#[derive(Debug, Clone)]
pub struct Subtree {
    /// 本子树的根坐标。
    pub root: ImplicitTileCoord,
    /// 子树内的瓦片可用性。
    pub tile_availability: AvailabilityBitstream,
    /// 子树内的内容可用性。
    pub content_availability: AvailabilityBitstream,
    /// 子树的孩子子树可用性。
    pub child_subtree_availability: AvailabilityBitstream,
}

impl Subtree {
    /// 返回子树中的节点总数。
    pub fn total_nodes(subtree_levels: u32, scheme: SubdivisionScheme) -> u64 {
        let branching = scheme.branching_factor() as u64;
        // branching^0 + branching^1 + ... + branching^(levels-1) 之和
        (branching.pow(subtree_levels) - 1) / (branching - 1)
    }

    /// 计算某个瓦片在子树内的线性索引。
    pub fn local_index(
        coord: &ImplicitTileCoord,
        subtree_root: &ImplicitTileCoord,
        scheme: SubdivisionScheme,
    ) -> u64 {
        let relative_level = coord.level - subtree_root.level;
        let branching = scheme.branching_factor() as u64;

        // 到本层级起始处的偏移
        let level_offset = if relative_level == 0 {
            0
        } else {
            (branching.pow(relative_level) - 1) / (branching - 1)
        };

        // 层级内的 Morton 索引
        let local_coord = ImplicitTileCoord {
            level: relative_level,
            x: coord.x - (subtree_root.x << relative_level),
            y: coord.y - (subtree_root.y << relative_level),
            z: coord.z - (subtree_root.z << relative_level),
            subtree_levels: coord.subtree_levels,
        };
        let morton = local_coord.morton_index(scheme);

        level_offset + morton
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subdivision_scheme() {
        assert_eq!(SubdivisionScheme::Quadtree.branching_factor(), 4);
        assert_eq!(SubdivisionScheme::Octree.branching_factor(), 8);
        assert_eq!(SubdivisionScheme::Quadtree.dimensions(), 2);
        assert_eq!(SubdivisionScheme::Octree.dimensions(), 3);
    }

    #[test]
    fn test_morton_2d() {
        assert_eq!(morton_2d(0, 0), 0);
        assert_eq!(morton_2d(1, 0), 1); // x 占偶数位
        assert_eq!(morton_2d(0, 1), 2); // y 占奇数位
        assert_eq!(morton_2d(1, 1), 3);
        assert_eq!(morton_2d(2, 0), 4);
    }

    #[test]
    fn test_morton_3d() {
        assert_eq!(morton_3d(0, 0, 0), 0);
        assert_eq!(morton_3d(1, 0, 0), 1); // x 占位置 0,3,6...
        assert_eq!(morton_3d(0, 1, 0), 2); // y 占位置 1,4,7...
        assert_eq!(morton_3d(0, 0, 1), 4); // z 占位置 2,5,8...
        assert_eq!(morton_3d(1, 1, 1), 7);
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
        assert_eq!(children[1], ImplicitTileCoord::quadtree(1, 1, 0));
        assert_eq!(children[2], ImplicitTileCoord::quadtree(1, 0, 1));
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
    fn test_availability_bitstream() {
        let mut bs = AvailabilityBitstream::new(16);
        assert!(!bs.is_available(0));
        assert!(!bs.is_available(5));

        bs.set(0, true);
        bs.set(5, true);
        bs.set(15, true);

        assert!(bs.is_available(0));
        assert!(!bs.is_available(1));
        assert!(bs.is_available(5));
        assert!(bs.is_available(15));
        assert!(!bs.is_available(16)); // 越界
    }

    #[test]
    fn test_availability_count() {
        let mut bs = AvailabilityBitstream::new(8);
        bs.set(0, true);
        bs.set(3, true);
        bs.set(7, true);
        assert_eq!(bs.count_available(), 3);
    }

    #[test]
    fn test_implicit_tiling_config_uri() {
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
    fn test_subtree_root() {
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

    #[test]
    fn test_subtree_total_nodes() {
        // 4 层的 Quadtree：1 + 4 + 16 + 64 = 85
        assert_eq!(Subtree::total_nodes(4, SubdivisionScheme::Quadtree), 85);
        // 2 层的 Octree：1 + 8 = 9
        assert_eq!(Subtree::total_nodes(2, SubdivisionScheme::Octree), 9);
    }

    #[test]
    fn test_subtree_local_index() {
        let root = ImplicitTileCoord::quadtree(0, 0, 0);
        let coord = ImplicitTileCoord::quadtree(1, 1, 0);
        let index = Subtree::local_index(&coord, &root, SubdivisionScheme::Quadtree);
        // 层级 1 从偏移 1 开始，morton(1,0) = 1（CesiumJS：x 占偶数位）
        assert_eq!(index, 1 + 1);
    }

    #[test]
    fn test_get_template_values_quadtree() {
        let coord = ImplicitTileCoord::quadtree(4, 3, 7);
        let result = coord.get_template_values("tiles/{level}/{x}/{y}.glb");
        assert_eq!(result, "tiles/4/3/7.glb");
    }

    #[test]
    fn test_get_template_values_octree() {
        let coord = ImplicitTileCoord::octree(3, 1, 2, 4);
        let result = coord.get_template_values("{level}/{x}/{y}/{z}.b3dm");
        assert_eq!(result, "3/1/2/4.b3dm");
    }

    #[test]
    fn test_get_template_values_repeated_placeholders() {
        let coord = ImplicitTileCoord::quadtree(2, 0, 1);
        let result = coord.get_template_values("{level}-{x}-{level}-{y}");
        assert_eq!(result, "2-0-2-1");
    }

    #[test]
    fn test_get_template_values_quadtree_z_stays_zero() {
        let coord = ImplicitTileCoord::quadtree(1, 0, 0);
        let result = coord.get_template_values("{level}/{x}/{y}/{z}");
        assert_eq!(result, "1/0/0/0");
    }
}
