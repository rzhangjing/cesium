//! Ported from `packages/engine/Source/Core/ApproximateTerrainHeights.js`.
//!
//! Approximate terrain heights for bounding sphere computations.

/// Approximate terrain heights data.
/// Skeleton: actual data is loaded from a JSON file at runtime.
pub struct ApproximateTerrainHeights;

impl ApproximateTerrainHeights {
    /// Initializes the approximate terrain heights data.
    pub fn initialize() -> Result<(), String> {
        // Skeleton: loads from Assets/approximateTerrainHeights.json
        Ok(())
    }

    /// Returns the approximate minimum and maximum terrain heights for a rectangle.
    pub fn get_minimum_maximum_heights(
        _rectangle: &crate::rectangle::Rectangle,
    ) -> (f64, f64) {
        (-1.0, 9000.0)
    }

    /// Destroys the cached data.
    pub fn destroy() {}
}
