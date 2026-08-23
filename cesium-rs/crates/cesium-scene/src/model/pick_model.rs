//! Ported from `packages/engine/Source/Scene/Model/pickModel.js`.

/// Picking utilities for models.
pub struct PickModel {
    _private: (),
}

impl PickModel {
    /// Creates a new PickModel.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PickModel {
    fn default() -> Self { Self::new() }
}
