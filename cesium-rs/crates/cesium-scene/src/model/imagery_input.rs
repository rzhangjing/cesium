//! Ported from `packages/engine/Source/Scene/Model/ImageryInput.js`.

use serde_json::Value;

/// A structure summarizing the input for the shader that is draping
/// imagery over 3D Tiles, as part of the `ImageryPipelineStage`.
#[derive(Debug, Clone)]
pub struct ImageryInput {
    /// The imagery layer (opaque reference).
    pub imagery_layer: Value,
    /// The texture from the imagery (opaque reference).
    pub texture: Value,
    /// The translation (x,y) and scale (z,w) for the texture.
    pub texture_translation_and_scale: Value,
    /// The bounding rectangle in texture coordinates as (minX, minY, maxX, maxY).
    pub texture_coordinate_rectangle: Value,
    /// The set index of the texture coordinate attribute.
    pub imagery_tex_coord_attribute_set_index: u32,
}

impl ImageryInput {
    /// Creates a new `ImageryInput`.
    pub fn new(
        imagery_layer: Value,
        texture: Value,
        texture_translation_and_scale: Value,
        texture_coordinate_rectangle: Value,
        imagery_tex_coord_attribute_set_index: u32,
    ) -> Self {
        Self {
            imagery_layer,
            texture,
            texture_translation_and_scale,
            texture_coordinate_rectangle,
            imagery_tex_coord_attribute_set_index,
        }
    }
}
