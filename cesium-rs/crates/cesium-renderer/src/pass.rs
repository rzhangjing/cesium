//! Ported from `packages/engine/Source/Renderer/Pass.js`.
//!
//! Render pass identifiers for command sorting.

/// The render pass for a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Pass {
    /// Environment pass.
    Environment = 0,
    /// Compute pass.
    Compute = 1,
    /// Globe pass.
    Globe = 2,
    /// Terrain classification pass.
    TerrainClassification = 3,
    /// 3D tile edges pass.
    Cesium3dTileEdges = 4,
    /// 3D tile pass.
    Cesium3dTile = 5,
    /// 3D tile classification pass.
    Cesium3dTileClassification = 6,
    /// 3D tile classification ignore show pass.
    Cesium3dTileClassificationIgnoreShow = 7,
    /// Opaque pass.
    Opaque = 8,
    /// Translucent pass.
    Translucent = 9,
    /// Voxels pass.
    Voxels = 10,
    /// Gaussian splats pass.
    GaussianSplats = 11,
    /// 3D tile edges direct pass.
    Cesium3dTileEdgesDirect = 12,
    /// Overlay pass.
    Overlay = 13,
    /// Total number of passes.
    NumberOfPasses = 14,
}
