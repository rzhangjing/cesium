//! Ported from `packages/engine/Source/Scene/Cesium3DTileContentType.js`.

/// Type of 3D tile content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTileContentType {
    /// 3D model.
    Model = 0,
    /// External tileset.
    ExternalTileset = 1,
    /// Geometric error.
    GeometricError = 2,
    /// Empty.
    Empty = 3,
}
