//! Ported from `packages/engine/Source/Scene/Cesium3DTileRefine.js`.

/// 3D tile refinement strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTileRefine {
    /// Add children.
    Add = 0,
    /// Replace with children.
    Replace = 1,
}
