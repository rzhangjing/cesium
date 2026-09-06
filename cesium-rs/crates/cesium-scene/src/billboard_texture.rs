//! Ported from `packages/engine/Source/Scene/BillboardTexture.js`.

/// Billboard texture.
///
/// Manages a texture used for billboard rendering.
pub struct BillboardTexture {
    /// The texture URL.
    pub url: String,
    /// Whether the texture is loaded.
    pub loaded: bool,
}

impl BillboardTexture {
    /// Creates a new BillboardTexture.
    pub fn new() -> Self { Self { url: String::new(), loaded: false } }
}

impl Default for BillboardTexture {
    fn default() -> Self { Self::new() }
}
