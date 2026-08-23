//! Ported from `packages/engine/Source/Scene/Cesium3DTilePass.js`.

/// 3D tile rendering pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTilePass {
    /// Render pass.
    Render = 0,
    /// Pick pass.
    Pick = 1,
    /// Shadow pass.
    Shadow = 2,
    /// Classification pass.
    Classification = 3,
}
