//! Ported from `packages/engine/Source/Scene/ImplicitSubdivisionScheme.js`.

/// Implicit subdivision scheme.
pub struct ImplicitSubdivisionScheme {
    _private: (),
}

impl ImplicitSubdivisionScheme {
    /// Creates a new ImplicitSubdivisionScheme.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImplicitSubdivisionScheme {
    fn default() -> Self { Self::new() }
}
