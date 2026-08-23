//! Ported from `packages/engine/Source/Scene/createGooglePhotorealistic3DTileset.js`.

/// Creates a Google Photorealistic 3D tileset.
pub struct CreateGooglePhotorealistic3DTileset {
    _private: (),
}

impl CreateGooglePhotorealistic3DTileset {
    /// Creates a new CreateGooglePhotorealistic3DTileset.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateGooglePhotorealistic3DTileset {
    fn default() -> Self { Self::new() }
}
