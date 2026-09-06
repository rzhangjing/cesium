//! Ported from `packages/engine/Source/Scene/CloudCollection.js`.

/// A collection of clouds.
///
/// Manages cumulus cloud rendering in the scene.
pub struct CloudCollection {
    /// Whether the collection is visible.
    pub show: bool,
    /// The number of clouds.
    pub length: u32,
}

impl CloudCollection {
    /// Creates a new CloudCollection.
    pub fn new() -> Self { Self { show: true, length: 0 } }
}

impl Default for CloudCollection {
    fn default() -> Self { Self::new() }
}
