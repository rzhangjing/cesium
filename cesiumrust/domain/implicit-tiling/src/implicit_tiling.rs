//! Implicit tiling for 3D Tiles 1.1.
//!
//! Maps to CesiumJS `Scene/Implicit3DTileContent.js`:
//! - Quadtree/Octree implicit subdivision
//! - Availability bitstreams
//! - Morton index computation
//! - Subtree file parsing

/// Subdivision scheme for implicit tiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SubdivisionScheme {
    /// Quadtree (2D subdivision).
    #[default]
    Quadtree,
    /// Octree (3D subdivision).
    Octree,
}

impl SubdivisionScheme {
    /// Returns the number of children per node.
    pub fn branching_factor(&self) -> u32 {
        match self {
            Self::Quadtree => 4,
            Self::Octree => 8,
        }
    }

    /// Returns the number of dimensions.
    pub fn dimensions(&self) -> u32 {
        match self {
            Self::Quadtree => 2,
            Self::Octree => 3,
        }
    }
}

/// A tile coordinate in the implicit tiling hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImplicitTileCoord {
    /// Level in the tree (0 = root).
    pub level: u32,
    /// X coordinate at this level.
    pub x: u32,
    /// Y coordinate at this level.
    pub y: u32,
    /// Z coordinate at this level (octree only).
    pub z: u32,
}

impl ImplicitTileCoord {
    /// Creates a new quadtree coordinate.
    pub fn quadtree(level: u32, x: u32, y: u32) -> Self {
        Self { level, x, y, z: 0 }
    }

    /// Creates a new octree coordinate.
    pub fn octree(level: u32, x: u32, y: u32, z: u32) -> Self {
        Self { level, x, y, z }
    }

    /// Computes the Morton index for this coordinate.
    pub fn morton_index(&self, scheme: SubdivisionScheme) -> u64 {
        match scheme {
            SubdivisionScheme::Quadtree => morton_2d(self.x, self.y),
            SubdivisionScheme::Octree => morton_3d(self.x, self.y, self.z),
        }
    }

    /// Returns the parent coordinate.
    pub fn parent(&self) -> Option<Self> {
        if self.level == 0 {
            return None;
        }
        Some(Self {
            level: self.level - 1,
            x: self.x / 2,
            y: self.y / 2,
            z: self.z / 2,
        })
    }

    /// Returns child coordinates.
    pub fn children(&self, scheme: SubdivisionScheme) -> Vec<Self> {
        let child_level = self.level + 1;
        let bx = self.x * 2;
        let by = self.y * 2;
        let bz = self.z * 2;

        match scheme {
            SubdivisionScheme::Quadtree => vec![
                Self::quadtree(child_level, bx, by),
                Self::quadtree(child_level, bx + 1, by),
                Self::quadtree(child_level, bx, by + 1),
                Self::quadtree(child_level, bx + 1, by + 1),
            ],
            SubdivisionScheme::Octree => vec![
                Self::octree(child_level, bx, by, bz),
                Self::octree(child_level, bx + 1, by, bz),
                Self::octree(child_level, bx, by + 1, bz),
                Self::octree(child_level, bx + 1, by + 1, bz),
                Self::octree(child_level, bx, by, bz + 1),
                Self::octree(child_level, bx + 1, by, bz + 1),
                Self::octree(child_level, bx, by + 1, bz + 1),
                Self::octree(child_level, bx + 1, by + 1, bz + 1),
            ],
        }
    }

    /// Returns the number of tiles at this level.
    pub fn tiles_at_level(level: u32, scheme: SubdivisionScheme) -> u64 {
        let per_dim = 1u64 << level;
        match scheme {
            SubdivisionScheme::Quadtree => per_dim * per_dim,
            SubdivisionScheme::Octree => per_dim * per_dim * per_dim,
        }
    }
}

/// Computes 2D Morton code (Z-order curve).
pub fn morton_2d(x: u32, y: u32) -> u64 {
    (part1by1(x as u64) << 1) | part1by1(y as u64)
}

/// Computes 3D Morton code.
pub fn morton_3d(x: u32, y: u32, z: u32) -> u64 {
    (part1by2(x as u64) << 2) | (part1by2(y as u64) << 1) | part1by2(z as u64)
}

/// Spreads bits for 2D Morton code.
fn part1by1(mut n: u64) -> u64 {
    n &= 0x0000_0000_ffff_ffff;
    n = (n | (n << 16)) & 0x0000_ffff_0000_ffff;
    n = (n | (n << 8)) & 0x00ff_00ff_00ff_00ff;
    n = (n | (n << 4)) & 0x0f0f_0f0f_0f0f_0f0f;
    n = (n | (n << 2)) & 0x3333_3333_3333_3333;
    n = (n | (n << 1)) & 0x5555_5555_5555_5555;
    n
}

/// Spreads bits for 3D Morton code.
fn part1by2(mut n: u64) -> u64 {
    n &= 0x0000_0000_001f_ffff;
    n = (n | (n << 32)) & 0x001f_0000_0000_ffff;
    n = (n | (n << 16)) & 0x001f_0000_ff00_00ff;
    n = (n | (n << 8)) & 0x100f_00f0_0f00_f00f;
    n = (n | (n << 4)) & 0x10c3_0c30_c30c_30c3;
    n = (n | (n << 2)) & 0x1249_2492_4924_9249;
    n
}

/// Availability bitstream for implicit tiles.
#[derive(Debug, Clone)]
pub struct AvailabilityBitstream {
    /// Bit data (LSB first within each byte).
    pub bits: Vec<u8>,
    /// Number of valid bits.
    pub length: u64,
}

impl AvailabilityBitstream {
    /// Creates a new bitstream with all bits unset.
    pub fn new(length: u64) -> Self {
        let byte_count = length.div_ceil(8) as usize;
        Self {
            bits: vec![0u8; byte_count],
            length,
        }
    }

    /// Creates from raw bytes.
    pub fn from_bytes(bits: Vec<u8>, length: u64) -> Self {
        Self { bits, length }
    }

    /// Returns true if the bit at index is set.
    pub fn is_available(&self, index: u64) -> bool {
        if index >= self.length {
            return false;
        }
        let byte_index = (index / 8) as usize;
        let bit_index = (index % 8) as u8;
        (self.bits[byte_index] >> bit_index) & 1 == 1
    }

    /// Sets the bit at index.
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

    /// Returns the number of available tiles.
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

/// Implicit tiling configuration from tileset.json.
#[derive(Debug, Clone)]
pub struct ImplicitTilingConfig {
    /// Subdivision scheme.
    pub subdivision_scheme: SubdivisionScheme,
    /// Number of levels in each subtree.
    pub subtree_levels: u32,
    /// Maximum number of levels in the tree.
    pub maximum_level: u32,
    /// URL template for subtree files.
    pub subtree_uri_template: String,
    /// URL template for content files.
    pub content_uri_template: String,
}

impl ImplicitTilingConfig {
    /// Generates a subtree URI for a given coordinate.
    pub fn get_subtree_uri(&self, coord: &ImplicitTileCoord) -> String {
        self.subtree_uri_template
            .replace("{level}", &coord.level.to_string())
            .replace("{x}", &coord.x.to_string())
            .replace("{y}", &coord.y.to_string())
            .replace("{z}", &coord.z.to_string())
    }

    /// Generates a content URI for a given coordinate.
    pub fn get_content_uri(&self, coord: &ImplicitTileCoord) -> String {
        self.content_uri_template
            .replace("{level}", &coord.level.to_string())
            .replace("{x}", &coord.x.to_string())
            .replace("{y}", &coord.y.to_string())
            .replace("{z}", &coord.z.to_string())
    }

    /// Computes the subtree root coordinate for a tile.
    pub fn get_subtree_root(&self, coord: &ImplicitTileCoord) -> ImplicitTileCoord {
        let subtree_level = (coord.level / self.subtree_levels) * self.subtree_levels;
        let level_diff = coord.level - subtree_level;
        ImplicitTileCoord {
            level: subtree_level,
            x: coord.x >> level_diff,
            y: coord.y >> level_diff,
            z: coord.z >> level_diff,
        }
    }
}

/// A parsed subtree file.
#[derive(Debug, Clone)]
pub struct Subtree {
    /// Root coordinate of this subtree.
    pub root: ImplicitTileCoord,
    /// Tile availability within the subtree.
    pub tile_availability: AvailabilityBitstream,
    /// Content availability within the subtree.
    pub content_availability: AvailabilityBitstream,
    /// Child subtree availability.
    pub child_subtree_availability: AvailabilityBitstream,
}

impl Subtree {
    /// Returns the total number of nodes in a subtree.
    pub fn total_nodes(subtree_levels: u32, scheme: SubdivisionScheme) -> u64 {
        let branching = scheme.branching_factor() as u64;
        // Sum of branching^0 + branching^1 + ... + branching^(levels-1)
        (branching.pow(subtree_levels) - 1) / (branching - 1)
    }

    /// Computes the linear index of a tile within the subtree.
    pub fn local_index(
        coord: &ImplicitTileCoord,
        subtree_root: &ImplicitTileCoord,
        scheme: SubdivisionScheme,
    ) -> u64 {
        let relative_level = coord.level - subtree_root.level;
        let branching = scheme.branching_factor() as u64;

        // Offset to the start of this level
        let level_offset = if relative_level == 0 {
            0
        } else {
            (branching.pow(relative_level) - 1) / (branching - 1)
        };

        // Morton index within the level
        let local_coord = ImplicitTileCoord {
            level: relative_level,
            x: coord.x - (subtree_root.x << relative_level),
            y: coord.y - (subtree_root.y << relative_level),
            z: coord.z - (subtree_root.z << relative_level),
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
        assert_eq!(morton_2d(1, 0), 2);
        assert_eq!(morton_2d(0, 1), 1);
        assert_eq!(morton_2d(1, 1), 3);
        assert_eq!(morton_2d(2, 0), 8);
    }

    #[test]
    fn test_morton_3d() {
        assert_eq!(morton_3d(0, 0, 0), 0);
        assert_eq!(morton_3d(1, 0, 0), 4);
        assert_eq!(morton_3d(0, 1, 0), 2);
        assert_eq!(morton_3d(0, 0, 1), 1);
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
        assert!(!bs.is_available(16)); // Out of bounds
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
        // Quadtree with 4 levels: 1 + 4 + 16 + 64 = 85
        assert_eq!(Subtree::total_nodes(4, SubdivisionScheme::Quadtree), 85);
        // Octree with 2 levels: 1 + 8 = 9
        assert_eq!(Subtree::total_nodes(2, SubdivisionScheme::Octree), 9);
    }

    #[test]
    fn test_subtree_local_index() {
        let root = ImplicitTileCoord::quadtree(0, 0, 0);
        let coord = ImplicitTileCoord::quadtree(1, 1, 0);
        let index = Subtree::local_index(&coord, &root, SubdivisionScheme::Quadtree);
        // Level 1 starts at offset 1, morton(1,0) = 2
        assert_eq!(index, 1 + 2);
    }
}
