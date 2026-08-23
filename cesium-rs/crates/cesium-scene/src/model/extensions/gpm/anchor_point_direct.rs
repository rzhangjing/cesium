//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Direct anchor point for GPM.
pub struct AnchorPointDirect {
    _private: (),
}

impl AnchorPointDirect {
    /// Creates a new AnchorPointDirect.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for AnchorPointDirect {
    fn default() -> Self { Self::new() }
}
