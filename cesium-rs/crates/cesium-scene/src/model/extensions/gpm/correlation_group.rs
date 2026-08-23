//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// A correlation group for GPM.
pub struct CorrelationGroup {
    _private: (),
}

impl CorrelationGroup {
    /// Creates a new CorrelationGroup.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CorrelationGroup {
    fn default() -> Self { Self::new() }
}
