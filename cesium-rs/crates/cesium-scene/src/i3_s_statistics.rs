//! Ported from `packages/engine/Source/Scene/I3SStatistics.js`.

/// I3S statistics.
///
/// Contains statistical information about I3S feature attributes.
pub struct I3SStatistics {
    /// The minimum value.
    pub min: Option<f64>,
    /// The maximum value.
    pub max: Option<f64>,
}

impl I3SStatistics {
    /// Creates a new I3SStatistics.
    pub fn new() -> Self { Self { min: None, max: None } }
}

impl Default for I3SStatistics {
    fn default() -> Self { Self::new() }
}
