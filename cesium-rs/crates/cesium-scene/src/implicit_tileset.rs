//! Ported from `packages/engine/Source/Scene/ImplicitTileset.js`.

/// Implicit tileset.
///
/// Represents an implicit tiling scheme within a 3D tileset.
pub struct ImplicitTileset {
    /// The subtree levels.
    pub subtree_levels: u32,
    /// Whether the tileset is loaded.
    pub loaded: bool,
}

impl ImplicitTileset {
    /// Creates a new ImplicitTileset.
    pub fn new() -> Self { Self { subtree_levels: 0, loaded: false } }
}

impl Default for ImplicitTileset {
    fn default() -> Self { Self::new() }
}
