//! Ported from `packages/engine/Source/Scene/Cesium3DTileOptimizedHint.js`.
//!
//! Optimization hint constants for 3D Tiles rendering.
//! This is a companion to `Cesium3DTileOptimizationHint` providing
//! additional hint categories.

/// Optimization hint constants for 3D Tiles.
///
/// Provides named constants for common optimization decisions
/// during tile selection and rendering.
pub struct Cesium3DTileOptimizedHint;

impl Cesium3DTileOptimizedHint {
    /// No optimization information is available.
    pub const NOT_COMPUTED: i8 = -1;
    /// The optimization should be skipped.
    pub const SKIP_OPTIMIZATION: i8 = 0;
    /// The optimization should be applied.
    pub const USE_OPTIMIZATION: i8 = 1;

    /// Returns whether the given hint value indicates an optimization should be used.
    pub fn should_optimize(value: i8) -> bool {
        value == Self::USE_OPTIMIZATION
    }

    /// Returns whether the hint value is valid (not NOT_COMPUTED).
    pub fn is_computed(value: i8) -> bool {
        value != Self::NOT_COMPUTED
    }
}
