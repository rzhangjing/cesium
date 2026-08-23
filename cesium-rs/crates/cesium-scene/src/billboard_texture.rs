//! Ported from `packages/engine/Source/Scene/BillboardTexture.js`.

/// A texture associated with a billboard.
pub struct BillboardTexture {
    _private: (),
}

impl BillboardTexture {
    /// Creates a new BillboardTexture.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BillboardTexture {
    fn default() -> Self { Self::new() }
}
