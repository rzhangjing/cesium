//! Ported from `packages/engine/Source/Scene/Megatexture.js`.

/// Megatexture.
///
/// Manages a large virtual texture composed of many smaller tiles.
pub struct Megatexture {
    /// The megatexture size in texels.
    pub size: u32,
    /// Whether the megatexture is ready.
    pub ready: bool,
}

impl Megatexture {
    /// Creates a new Megatexture.
    pub fn new() -> Self { Self { size: 0, ready: false } }
}

impl Default for Megatexture {
    fn default() -> Self { Self::new() }
}
