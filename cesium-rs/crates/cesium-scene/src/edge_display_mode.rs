//! Ported from `packages/engine/Source/Scene/EdgeDisplayMode.js`.

/// Edge display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EdgeDisplayMode {
    /// No edges.
    None = 0,
    /// Flat edges.
    Flat = 1,
    /// Phong edges.
    Phong = 2,
}
