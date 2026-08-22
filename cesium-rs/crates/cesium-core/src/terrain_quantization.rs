//! Ported from `packages/engine/Source/Core/TerrainQuantization.js`.

/// This enumerated type is used to determine how the vertices of the
/// terrain mesh are compressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TerrainQuantization {
    /// The vertices are not compressed.
    None = 0,
    /// The vertices are compressed to 12 bits.
    Bits12 = 1,
}
