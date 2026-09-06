//! Ported from `packages/engine/Source/Scene/Implicit3DTileContent.js`.

/// Content for implicit 3D tiles.
///
/// Represents a tile content derived from implicit tiling subdivision.
pub struct Implicit3DTileContent {
    /// Whether the content is ready.
    pub ready: bool,
    /// The implicit tile coordinates.
    pub tile_coordinates: Option<(u32, u32, u32)>,
}

impl Implicit3DTileContent {
    /// Creates a new Implicit3DTileContent.
    pub fn new() -> Self { Self { ready: false, tile_coordinates: None } }

    /// Returns true if the content is ready.
    pub fn is_ready(&self) -> bool { self.ready }
}

impl Default for Implicit3DTileContent {
    fn default() -> Self { Self::new() }
}
