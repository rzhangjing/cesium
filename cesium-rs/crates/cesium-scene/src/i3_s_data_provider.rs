//! Ported from `packages/engine/Source/Scene/I3SDataProvider.js`.

/// I3S data provider.
pub struct I3SDataProvider {
    _private: (),
}

impl I3SDataProvider {
    /// Creates a new I3SDataProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SDataProvider {
    fn default() -> Self { Self::new() }
}
