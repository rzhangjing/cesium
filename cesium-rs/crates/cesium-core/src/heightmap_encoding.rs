//! Ported from `packages/engine/Source/Core/HeightmapEncoding.js`.

/// The encoding that is used for a heightmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum HeightmapEncoding {
    /// No encoding.
    None = 0,
    /// LERC encoding.
    Lerc = 1,
}
