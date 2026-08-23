//! Ported from `packages/engine/Source/Scene/Cesium3DTileContentState.js`.

/// State of 3D tile content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Cesium3DTileContentState {
    /// Not loaded.
    Unloaded = 0,
    /// Loading.
    Loading = 1,
    /// Processing.
    Processing = 2,
    /// Ready.
    Ready = 3,
    /// Failed.
    Failed = 4,
    /// Unloaded.
    Expired = 5,
}
