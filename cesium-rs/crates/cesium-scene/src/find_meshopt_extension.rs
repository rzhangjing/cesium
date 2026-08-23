//! Ported from `packages/engine/Source/Scene/findMeshoptExtension.js`.

/// Finds meshopt extension.
pub struct FindMeshoptExtension {
    _private: (),
}

impl FindMeshoptExtension {
    /// Creates a new FindMeshoptExtension.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for FindMeshoptExtension {
    fn default() -> Self { Self::new() }
}
