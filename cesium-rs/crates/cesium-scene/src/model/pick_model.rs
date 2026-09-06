//! Ported from `packages/engine/Source/Scene/Model/PickModel.js`.

/// Pick model.
///
/// Handles picking of model primitives.
pub struct PickModel {
    /// Whether picking is active.
    pub active: bool,
}

impl PickModel {
    /// Creates a new PickModel.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for PickModel {
    fn default() -> Self { Self::new() }
}
