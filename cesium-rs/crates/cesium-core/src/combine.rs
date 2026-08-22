//! Ported from packages/engine/Source/Core/combine.js
//!
//! DEVIATION: CesiumJS `combine` merges arbitrary JS object graphs; the Rust
//! port merges `serde_json::Value` objects, which is how CesiumJS option
//! bags surface in the ported code (e.g. provider options). See
//! docs/deviations.md.

use serde_json::Value;

/// Merges two objects, copying their properties onto a new combined object.
/// When two objects have the same property, the value of the property on the
/// first object is used. If either object is `None`, it will be treated as
/// an empty object.
///
/// Port of CesiumJS `combine(object1, object2, deep)`.
#[must_use]
pub fn combine(object1: Option<&Value>, object2: Option<&Value>, deep: Option<bool>) -> Value {
    let deep = deep.unwrap_or(false);

    let mut result = serde_json::Map::new();

    let object2_defined = object2.is_some();

    if let Some(object1) = object1 {
        if let Value::Object(object1) = object1 {
            for (property, object1_value) in object1 {
                if object2_defined
                    && deep
                    && object1_value.is_object()
                {
                    if let Some(Value::Object(object2_map)) = object2.and_then(|o| o.get(property))
                    {
                        let _ = object2_map; // property exists in object2 and both are objects
                        result.insert(
                            property.clone(),
                            combine(
                                Some(object1_value),
                                object2.map(|o| o.get(property).unwrap()),
                                Some(deep),
                            ),
                        );
                        continue;
                    }
                    result.insert(property.clone(), object1_value.clone());
                } else {
                    result.insert(property.clone(), object1_value.clone());
                }
            }
        }
    }
    if let Some(Value::Object(object2)) = object2 {
        for (property, object2_value) in object2 {
            if !result.contains_key(property) {
                result.insert(property.clone(), object2_value.clone());
            }
        }
    }
    Value::Object(result)
}
