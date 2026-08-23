//! Ported from `packages/engine/Source/DataSources/EntityView.js`.

use cesium_core::cartesian3::Cartesian3;

/// Defines the view to use when tracking an entity.
pub struct EntityView {
    /// The offset position in the local coordinate frame.
    pub offset: Cartesian3,
}

impl EntityView {
    /// Creates a new entity view.
    pub fn new() -> Self {
        Self {
            offset: Cartesian3::new(0.0, 0.0, 100.0),
        }
    }
}

impl Default for EntityView {
    fn default() -> Self { Self::new() }
}
