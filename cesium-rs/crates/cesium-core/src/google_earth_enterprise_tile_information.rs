//! Ported from `packages/engine/Source/Core/GoogleEarthEnterpriseTileInformation.js`.
//!
//! Tile information for Google Earth Enterprise terrain.

/// Tile information for Google Earth Enterprise terrain.
/// Skeleton: requires binary parsing.
pub struct GoogleEarthEnterpriseTileInformation {
    /// Whether the tile has terrain data.
    pub has_terrain: bool,
    /// Whether the tile has imagery.
    pub has_imagery: bool,
}

impl GoogleEarthEnterpriseTileInformation {
    /// Creates new tile information.
    pub fn new(has_terrain: bool, has_imagery: bool) -> Self {
        Self { has_terrain, has_imagery }
    }
}
