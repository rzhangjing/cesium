//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Indirect anchor point for GPM.
pub struct AnchorPointIndirect {
    _private: (),
}

impl AnchorPointIndirect {
    /// Creates a new AnchorPointIndirect.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for AnchorPointIndirect {
    fn default() -> Self { Self::new() }
}
