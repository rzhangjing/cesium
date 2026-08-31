//! Ported from `packages/engine/Source/Scene/Model/Extensions/Gpm/PpeTexture.js`.

use crate::model::extensions::gpm::ppe_metadata::PpeMetadata;

/// PPE (Per-Point Error) texture in `NGA_gpm_local`.
///
/// This reflects the `ppeTexture` definition of the NGA_gpm_local glTF
/// extension.
///
/// This is a valid glTF `TextureInfo` object (with a required `index`
/// and an optional `texCoord`), with additional properties that
/// describe the structure of the metadata that is stored in the
/// texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PpeTexture {
    /// The traits that indicate which data is stored in this texture.
    traits: PpeMetadata,
    /// The index of the texture inside the glTF textures array.
    index: usize,
    /// The optional set index for the TEXCOORD attribute.
    tex_coord: Option<usize>,
    /// The value to represent missing data.
    no_data: Option<f64>,
    /// An offset to apply to property values.
    offset: Option<f64>,
    /// A scale to apply to property values.
    scale: Option<f64>,
}

impl PpeTexture {
    /// Creates a new `PpeTexture`.
    ///
    /// Port of the `PpeTexture(options)` constructor. The range check
    /// (`index >= 0`) is statically guaranteed by the `usize` type.
    pub fn new(
        traits: PpeMetadata,
        index: usize,
        tex_coord: Option<usize>,
        no_data: Option<f64>,
        offset: Option<f64>,
        scale: Option<f64>,
    ) -> Self {
        Self {
            traits,
            index,
            tex_coord,
            no_data,
            offset,
            scale,
        }
    }

    /// The data contained here applies to this node and corresponding
    /// texture (port of the `traits` getter).
    pub fn traits(&self) -> &PpeMetadata {
        &self.traits
    }

    /// A value to represent missing data - also known as a sentinel
    /// value - wherever it appears (port of the `noData` getter).
    pub fn no_data(&self) -> Option<f64> {
        self.no_data
    }

    /// An offset to apply to property values (port of the `offset`
    /// getter).
    pub fn offset(&self) -> Option<f64> {
        self.offset
    }

    /// A scale to apply to property values (port of the `scale` getter).
    pub fn scale(&self) -> Option<f64> {
        self.scale
    }

    /// The index of the texture (port of the `index` getter).
    pub fn index(&self) -> usize {
        self.index
    }

    /// The set index of texture's TEXCOORD attribute used for texture
    /// coordinate mapping (port of the `texCoord` getter).
    pub fn tex_coord(&self) -> Option<usize> {
        self.tex_coord
    }
}
