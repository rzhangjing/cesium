//! Ported from `packages/engine/Source/Scene/CreateGooglePhotorealistic3DTileset.js`.

/// Creates a Google Photorealistic 3D Tileset.
///
/// Factory for Google's photorealistic 3D Tiles asset.
pub struct CreateGooglePhotorealistic3DTileset {
    /// Whether creation is complete.
    pub complete: bool,
}

impl CreateGooglePhotorealistic3DTileset {
    /// Creates a new CreateGooglePhotorealistic3DTileset.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for CreateGooglePhotorealistic3DTileset {
    fn default() -> Self { Self::new() }
}
