//! Ported from `packages/engine/Source/Scene/ClassificationType.js`.

/// Type of classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ClassificationType {
    /// 3D Tiles.
    Cesium3DTiles = 0,
    /// Terrain.
    Terrain = 1,
    /// Both.
    Both = 2,
}
