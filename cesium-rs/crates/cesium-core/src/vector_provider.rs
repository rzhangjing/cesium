//! Ported from `packages/engine/Source/Core/VectorProvider.js`.

/// A provider for vector data.
pub struct VectorProvider {
    _private: (),
}

impl VectorProvider {
    /// Creates a new VectorProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VectorProvider {
    fn default() -> Self { Self::new() }
}
