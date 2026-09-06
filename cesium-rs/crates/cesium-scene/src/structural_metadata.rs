//! Ported from `packages/engine/Source/Scene/StructuralMetadata.js`.

/// Structural metadata.
///
/// Provides access to EXT_structural_metadata data on a tileset.
pub struct StructuralMetadata {
    /// The number of property tables.
    pub property_table_count: u32,
    /// The number of property textures.
    pub property_texture_count: u32,
    /// The number of property attributes.
    pub property_attribute_count: u32,
}

impl StructuralMetadata {
    /// Creates a new StructuralMetadata.
    pub fn new() -> Self {
        Self { property_table_count: 0, property_texture_count: 0, property_attribute_count: 0 }
    }
}

impl Default for StructuralMetadata {
    fn default() -> Self { Self::new() }
}
