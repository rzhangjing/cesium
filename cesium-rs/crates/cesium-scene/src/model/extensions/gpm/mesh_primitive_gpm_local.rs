//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/MeshPrimitiveGpmLocal.js`.

use crate::model::extensions::gpm::ppe_texture::PpeTexture;

/// Local Generic Point-cloud Model information about a glTF primitive.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshPrimitiveGpmLocal {
    /// The Per-Point Error textures.
    ppe_textures: Vec<PpeTexture>,
}

impl MeshPrimitiveGpmLocal {
    /// Creates a new `MeshPrimitiveGpmLocal`.
    ///
    /// Port of the `MeshPrimitiveGpmLocal(ppeTextures)` constructor.
    pub fn new(ppe_textures: Vec<PpeTexture>) -> Self {
        Self { ppe_textures }
    }

    /// An array of ppe textures (port of the `ppeTextures` getter).
    pub fn ppe_textures(&self) -> &[PpeTexture] {
        &self.ppe_textures
    }
}
