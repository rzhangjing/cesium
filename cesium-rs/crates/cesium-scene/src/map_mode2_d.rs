//! Ported from `packages/engine/Source/Scene/MapMode2D.js`.

/// Defines how 2D mode handles wrapping around the international date line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MapMode2D {
    /// 2D mode does not wrap.
    Rotate = 0,
    /// 2D mode wraps around the date line.
    InfiniteScroll = 1,
}
