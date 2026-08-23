//! Ported from `packages/engine/Source/Scene/CumulusCloud.js`.

/// A cumulus cloud.
pub struct CumulusCloud {
    _private: (),
}

impl CumulusCloud {
    /// Creates a new CumulusCloud.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CumulusCloud {
    fn default() -> Self { Self::new() }
}
