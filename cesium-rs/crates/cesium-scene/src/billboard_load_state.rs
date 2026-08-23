//! Ported from `packages/engine/Source/Scene/BillboardLoadState.js`.

/// The loading state of a billboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BillboardLoadState {
    /// Not yet loaded.
    Unloaded = 0,
    /// Currently loading.
    Loading = 1,
    /// Loaded and ready to render.
    Ready = 2,
    /// Loading failed.
    Failed = 3,
}
