//! Ported from `packages/engine/Source/Scene/ImplicitTileCoordinates.js`.

/// Implicit tile coordinates.
pub struct ImplicitTileCoordinates {
    _private: (),
}

impl ImplicitTileCoordinates {
    /// Creates a new ImplicitTileCoordinates.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImplicitTileCoordinates {
    fn default() -> Self { Self::new() }
}
