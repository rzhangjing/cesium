//! Ported from `packages/engine/Source/Scene/Cesium3DTileColorBlendMode.js`.

/// The color blend mode for 3D Tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTileColorBlendMode {
    /// Replace the color with the highlight color.
    Highlight = 0,
    /// Mix between the source color and highlight.
    Replace = 1,
    /// Mix between the source color and highlight based on distance.
    Mix = 2,
}
