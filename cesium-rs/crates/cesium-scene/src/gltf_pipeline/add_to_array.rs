//! Ported from `packages/engine/Source/Scene/GltfPipeline/addToArray.js`.

use serde_json::Value;

/// Adds an element to an array and returns the element's index.
///
/// DEVIATION: JavaScript `Array.prototype.indexOf` uses reference equality
/// for objects; the Rust port compares with structural equality
/// (`serde_json::Value` `PartialEq`), which only makes duplicate detection
/// more conservative (never misses a true duplicate).
pub fn add_to_array(array: &mut Vec<Value>, element: Value, check_duplicates: bool) -> usize {
    if check_duplicates {
        if let Some(index) = array.iter().position(|existing| *existing == element) {
            return index;
        }
    }

    array.push(element);
    array.len() - 1
}

/// Variant operating on a [`serde_json::Value`] array property, creating the
/// array when the property is absent or null (the Rust analogue of the JS
/// `array = array ?? []` initialization pattern used by callers such as
/// `addExtensionsUsed`). Returns the element's index.
pub fn add_to_array_value(
    array: &mut Value,
    element: Value,
    check_duplicates: bool,
) -> usize {
    if !array.is_array() {
        *array = Value::Array(Vec::new());
    }
    let list = array.as_array_mut().expect("ensured above");
    add_to_array(list, element, check_duplicates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn add_to_array_appends_and_returns_index() {
        let mut array = vec![json!("a")];
        assert_eq!(add_to_array(&mut array, json!("b"), false), 1);
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn add_to_array_check_duplicates_returns_existing_index() {
        let mut array = vec![json!("a"), json!("b")];
        assert_eq!(add_to_array(&mut array, json!("a"), true), 0);
        assert_eq!(array.len(), 2);
    }

    #[test]
    fn add_to_array_check_duplicates_structural_object_equality() {
        let mut array = vec![json!({ "uri": "a.bin" })];
        assert_eq!(add_to_array(&mut array, json!({ "uri": "a.bin" }), true), 0);
        assert_eq!(add_to_array(&mut array, json!({ "uri": "b.bin" }), true), 1);
    }

    #[test]
    fn add_to_array_value_creates_missing_array() {
        let mut value = Value::Null;
        assert_eq!(add_to_array_value(&mut value, json!("x"), true), 0);
        assert_eq!(add_to_array_value(&mut value, json!("x"), true), 0);
        assert_eq!(add_to_array_value(&mut value, json!("y"), true), 1);
        assert_eq!(value, json!(["x", "y"]));
    }
}
