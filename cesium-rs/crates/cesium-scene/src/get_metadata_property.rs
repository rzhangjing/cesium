//! Ported from `packages/engine/Source/Scene/getMetadataProperty.js`.

use serde_json::Value;

/// Return the property texture property from the given structural metadata
/// that matches the given description.
///
/// If the given structural metadata is `None`, then `None` is returned.
///
/// Otherwise, this method will check all the property textures in the given
/// structural metadata. If it finds a property texture that has a class with
/// an `id` that matches the given name, and that contains a property for the
/// given property name, then this property is returned.
///
/// Otherwise, `None` is returned.
pub fn get_metadata_property(
    structural_metadata: Option<&Value>,
    class_name: &str,
    property_name: &str,
) -> Option<Value> {
    let structural_metadata = structural_metadata?;
    let property_textures = structural_metadata.get("propertyTextures")?.as_array()?;

    for property_texture in property_textures {
        let metadata_class = property_texture.get("class")?;
        if metadata_class.get("id").and_then(|v| v.as_str()) == Some(class_name) {
            let properties = property_texture.get("properties")?.as_object()?;
            if let Some(property) = properties.get(property_name) {
                return Some(property.clone());
            }
        }
    }

    None
}
