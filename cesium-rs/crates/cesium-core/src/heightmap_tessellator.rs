//! Ported from `packages/engine/Source/Core/HeightmapTessellator.js`.
//!
//! ## Method-level alignment table
//!
//! | JS | Rust | Notes |
//! |---|---|---|
//! | `HeightmapTessellator.DEFAULT_STRUCTURE` | [`HeightmapTessellator::DEFAULT_STRUCTURE`] | identical default values |
//! | `HeightmapTessellator.computeVertices` | — | DEVIATION: vertex tessellation runs inside the heightmap terrain-data mesh builder (see `heightmap_terrain_data.rs`); full worker materialization belongs to the Globe terrain batch |

/// Describes the layout of height samples in a heightmap buffer.
///
/// Mirrors the `structure` option of `HeightmapTerrainData` /
/// `HeightmapTessellator.computeVertices`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeightmapStructure {
    /// The factor by which to multiply height samples to obtain the height
    /// above `height_offset`, in meters.
    pub height_scale: f64,
    /// The offset to add to the scaled height to obtain the final height.
    pub height_offset: f64,
    /// The number of buffer elements that make up a single height sample.
    pub elements_per_height: usize,
    /// The number of elements to skip between the first element of
    /// consecutive heights.
    pub stride: usize,
    /// The multiplier used to combine the elements of a multi-element height.
    pub element_multiplier: f64,
    /// Indicates endianness of multi-element heights.
    pub is_big_endian: bool,
    /// The lowest value that can be stored, or `None` for no lower bound.
    pub lowest_encoded_height: Option<f64>,
    /// The highest value that can be stored, or `None` for no upper bound.
    pub highest_encoded_height: Option<f64>,
}

impl Default for HeightmapStructure {
    fn default() -> Self {
        *HeightmapTessellator::DEFAULT_STRUCTURE
    }
}

/// Tessellates heightmap data into triangles.
pub struct HeightmapTessellator {
    _private: (),
}

impl HeightmapTessellator {
    /// The default heightmap structure:
    /// heightScale 1.0, heightOffset 0.0, elementsPerHeight 1, stride 1,
    /// elementMultiplier 256.0, isBigEndian false, no clamping bounds.
    ///
    /// Mirrors `HeightmapTessellator.DEFAULT_STRUCTURE`.
    pub const DEFAULT_STRUCTURE: &HeightmapStructure = &HeightmapStructure {
        height_scale: 1.0,
        height_offset: 0.0,
        elements_per_height: 1,
        stride: 1,
        element_multiplier: 256.0,
        is_big_endian: false,
        lowest_encoded_height: None,
        highest_encoded_height: None,
    };

    /// Creates a new HeightmapTessellator.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for HeightmapTessellator {
    fn default() -> Self { Self::new() }
}
