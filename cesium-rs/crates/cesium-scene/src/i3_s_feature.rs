//! Ported from `packages/engine/Source/Scene/I3SFeature.js`.

/// An I3S feature.
///
/// Represents a single feature within an I3S node.
pub struct I3SFeature {
    /// The feature ID.
    pub id: u64,
    /// Whether the feature is visible.
    pub show: bool,
}

impl I3SFeature {
    /// Creates a new I3SFeature.
    pub fn new() -> Self { Self { id: 0, show: true } }
}

impl Default for I3SFeature {
    fn default() -> Self { Self::new() }
}
