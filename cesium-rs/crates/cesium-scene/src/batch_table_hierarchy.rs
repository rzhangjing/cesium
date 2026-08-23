//! Ported from `packages/engine/Source/Scene/BatchTableHierarchy.js`.

/// A batch table hierarchy for structured metadata.
pub struct BatchTableHierarchy {
    _private: (),
}

impl BatchTableHierarchy {
    /// Creates a new BatchTableHierarchy.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BatchTableHierarchy {
    fn default() -> Self { Self::new() }
}
