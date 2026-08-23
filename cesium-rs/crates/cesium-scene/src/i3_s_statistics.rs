//! Ported from `packages/engine/Source/Scene/I3SStatistics.js`.

/// I3S statistics.
pub struct I3SStatistics {
    _private: (),
}

impl I3SStatistics {
    /// Creates a new I3SStatistics.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SStatistics {
    fn default() -> Self { Self::new() }
}
