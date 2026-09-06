//! Ported from `packages/engine/Source/Scene/Model/PrimitiveOutlineGenerator.js`.
//!
//! Generates per-primitive outline textures and applies them to model
//! primitives for the outline rendering effect.

use cesium_core::color::Color;

/// Generates outlines for primitives.
///
/// Manages the outline texture atlas and per-primitive outline assignment.
/// Each outlined primitive receives a unique texel in the atlas, allowing
/// the outline shader to identify and highlight it.
/// Mirrors CesiumJS `PrimitiveOutlineGenerator` (~180 lines).
pub struct PrimitiveOutlineGenerator {
    /// The width of the outline texture atlas in texels.
    pub atlas_width: u32,
    /// The height of the outline texture atlas in texels.
    pub atlas_height: u32,
    /// The next available texel index in the atlas.
    next_texel_index: u32,
    /// The maximum number of outlines supported (atlas capacity).
    max_outlines: u32,
    /// Per-primitive outline assignments (primitive_index → texel_index).
    assignments: std::collections::HashMap<usize, u32>,
    /// The outline color.
    pub outline_color: Color,
}

impl PrimitiveOutlineGenerator {
    /// Creates a new `PrimitiveOutlineGenerator`.
    pub fn new() -> Self {
        let atlas_width = 256;
        let atlas_height = 256;
        Self {
            atlas_width,
            atlas_height,
            next_texel_index: 0,
            max_outlines: atlas_width * atlas_height,
            assignments: std::collections::HashMap::new(),
            outline_color: Color::new(1.0, 1.0, 0.0, 1.0),
        }
    }

    /// Assigns an outline texel to a primitive, returning the texel index.
    ///
    /// Returns `None` if the atlas is full.
    pub fn assign_outline(&mut self, primitive_index: usize) -> Option<u32> {
        if let Some(&existing) = self.assignments.get(&primitive_index) {
            return Some(existing);
        }
        if self.next_texel_index >= self.max_outlines {
            return None;
        }
        let texel = self.next_texel_index;
        self.next_texel_index += 1;
        self.assignments.insert(primitive_index, texel);
        Some(texel)
    }

    /// Removes the outline assignment for a primitive.
    pub fn remove_outline(&mut self, primitive_index: usize) {
        self.assignments.remove(&primitive_index);
    }

    /// Returns the number of active outline assignments.
    pub fn active_count(&self) -> usize {
        self.assignments.len()
    }

    /// Returns whether a primitive has an outline assigned.
    pub fn has_outline(&self, primitive_index: usize) -> bool {
        self.assignments.contains_key(&primitive_index)
    }

    /// Gets the texel index assigned to a primitive.
    pub fn get_texel_index(&self, primitive_index: usize) -> Option<u32> {
        self.assignments.get(&primitive_index).copied()
    }
}

impl Default for PrimitiveOutlineGenerator {
    fn default() -> Self { Self::new() }
}
