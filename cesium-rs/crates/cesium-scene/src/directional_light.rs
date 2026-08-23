//! Ported from `packages/engine/Source/Scene/DirectionalLight.js`.

/// A directional light.
///
/// DEVIATION: stub implementation.
pub struct DirectionalLight {
    _private: (),
}

impl DirectionalLight {
    /// Creates a new directional light.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DirectionalLight {
    fn default() -> Self { Self::new() }
}
