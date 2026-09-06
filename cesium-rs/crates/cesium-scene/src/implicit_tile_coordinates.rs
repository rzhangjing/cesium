//! Ported from `packages/engine/Source/Scene/ImplicitTileCoordinates.js`.

/// Implicit tile coordinates.
///
/// Tracks the (level, x, y, z) coordinates of an implicit tile.
pub struct ImplicitTileCoordinates {
    /// The level in the tile tree.
    pub level: u32,
    /// The x coordinate.
    pub x: u64,
    /// The y coordinate.
    pub y: u64,
    /// The z coordinate.
    pub z: u64,
}

impl ImplicitTileCoordinates {
    /// Creates a new ImplicitTileCoordinates.
    pub fn new() -> Self { Self { level: 0, x: 0, y: 0, z: 0 } }
}

impl Default for ImplicitTileCoordinates {
    fn default() -> Self { Self::new() }
}
