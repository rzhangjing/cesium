//! Ported from `packages/engine/Source/Scene/DynamicEnvironmentMapManager.js`.

/// Dynamic environment map manager.
///
/// Manages dynamic environment map generation for reflections.
pub struct DynamicEnvironmentMapManager {
    /// Whether the manager is active.
    pub active: bool,
}

impl DynamicEnvironmentMapManager {
    /// Creates a new DynamicEnvironmentMapManager.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for DynamicEnvironmentMapManager {
    fn default() -> Self { Self::new() }
}
