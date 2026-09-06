//! Ported from `packages/engine/Source/Scene/getMetadataClassProperty.js`.

use serde_json::Value;

/// Return the `MetadataClassProperty` JSON from the given schema that
/// matches the given description.
///
/// If the given schema is `None`, then `None` is returned.
/// If the given `schema_id` is defined but does not match the ID
/// of the given schema, then `None` is returned.
/// If the given schema does not have a class with the given name,
/// or the class does not have a property with the given name,
/// then `None` is returned.
///
/// Otherwise, the `MetadataClassProperty` JSON value is returned.
pub fn get_metadata_class_property(
    schema: Option<&Value>,
    schema_id: Option<&str>,
    class_name: &str,
    property_name: &str,
) -> Option<Value> {
    let schema = schema?;

    // If schemaId is specified, it must match
    if let Some(sid) = schema_id {
        if schema.get("id").and_then(|v| v.as_str()) != Some(sid) {
            return None;
        }
    }

    let classes = schema.get("classes")?.as_object()?;
    let metadata_class = classes.get(class_name)?;
    let properties = metadata_class.get("properties")?.as_object()?;
    let metadata_property = properties.get(property_name)?;

    Some(metadata_property.clone())
}
