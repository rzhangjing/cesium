//! Ported from `packages/engine/Source/Scene/PickedMetadataInfo.js`.

use serde_json::Value;

/// Information about metadata that is supposed to be picked.
///
/// This is initialized in the `Scene.pickMetadata` function, and passed to
/// the `FrameState`. It is used to configure the draw commands that render
/// the metadata values of an object into the picking frame buffer.
#[derive(Debug, Clone)]
pub struct PickedMetadataInfo {
    /// The optional ID of the metadata schema.
    pub schema_id: Option<String>,
    /// The name of the metadata class.
    pub class_name: String,
    /// The name of the metadata property.
    pub property_name: String,
    /// The `MetadataClassProperty` that is described by this structure.
    pub class_property: Option<Value>,
    /// The `PropertyTextureProperty` or `PropertyAttributeProperty` that
    /// is described by this structure.
    pub metadata_property: Option<Value>,
}

impl PickedMetadataInfo {
    /// Creates a new `PickedMetadataInfo`.
    pub fn new(
        schema_id: Option<String>,
        class_name: String,
        property_name: String,
        class_property: Option<Value>,
        metadata_property: Option<Value>,
    ) -> Self {
        Self {
            schema_id,
            class_name,
            property_name,
            class_property,
            metadata_property,
        }
    }
}
