//! Ported from `packages/engine/Source/Scene/ImplicitTileset.js`.

/// Implicit tileset.
pub struct ImplicitTileset {
    _private: (),
}

impl ImplicitTileset {
    /// Creates a new ImplicitTileset.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImplicitTileset {
    fn default() -> Self { Self::new() }
}
