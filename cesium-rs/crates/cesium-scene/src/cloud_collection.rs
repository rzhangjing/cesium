//! Ported from `packages/engine/Source/Scene/CloudCollection.js`.

/// A collection of clouds.
pub struct CloudCollection {
    _private: (),
}

impl CloudCollection {
    /// Creates a new CloudCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CloudCollection {
    fn default() -> Self { Self::new() }
}
