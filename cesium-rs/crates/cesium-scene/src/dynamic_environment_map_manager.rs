//! Ported from `packages/engine/Source/Scene/DynamicEnvironmentMapManager.js`.

/// Manages dynamic environment maps.
///
/// DEVIATION: stub implementation.
pub struct DynamicEnvironmentMapManager {
    _private: (),
}

impl DynamicEnvironmentMapManager {
    /// Creates a new dynamic environment map manager.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for DynamicEnvironmentMapManager {
    fn default() -> Self { Self::new() }
}
