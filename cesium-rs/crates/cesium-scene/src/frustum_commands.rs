//! Ported from `packages/engine/Source/Scene/FrustumCommands.js`.

/// Commands for a frustum.
pub struct FrustumCommands {
    _private: (),
}

impl FrustumCommands {
    /// Creates a new FrustumCommands.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for FrustumCommands {
    fn default() -> Self { Self::new() }
}
