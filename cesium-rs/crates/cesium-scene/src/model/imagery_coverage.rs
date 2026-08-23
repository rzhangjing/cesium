//! Ported from `packages/engine/Source/Scene/Model/ImageryCoverage.js`.

/// Coverage information for model imagery.
pub struct ImageryCoverage {
    _private: (),
}

impl ImageryCoverage {
    /// Creates a new ImageryCoverage.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImageryCoverage {
    fn default() -> Self { Self::new() }
}
