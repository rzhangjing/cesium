//! Ported from `packages/engine/Source/Scene/Cesium3DTileOptimizationHint.js`.

/// Whether an optimization should be applied (JS
/// `Cesium3DTileOptimizationHint`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i8)]
pub enum Cesium3DTileOptimizationHint {
    /// The optimization has not been computed yet (JS `NOT_COMPUTED = -1`).
    NotComputed = -1,
    /// Do not apply the optimization (JS `SKIP_OPTIMIZATION = 0`).
    SkipOptimization = 0,
    /// Apply the optimization (JS `USE_OPTIMIZATION = 1`).
    UseOptimization = 1,
}
