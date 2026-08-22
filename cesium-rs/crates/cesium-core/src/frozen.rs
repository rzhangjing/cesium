//! Ported from packages/engine/Source/Core/Frozen.js
//!
//! Utilities helpful for setting a default value for a parameter.
//!
//! Note: this file replaces the historical `defaultValue.js` /
//! `freezeObject.js` utilities (removed upstream in @cesium/engine 26.x).

use serde_json::Value;

/// A frozen empty object that can be used as the default value for options
/// passed as an object literal.
///
/// Port of `Frozen.EMPTY_OBJECT`.
#[must_use]
pub fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

/// A frozen empty array that can be used as the default value for options
/// passed as an array literal.
///
/// Port of `Frozen.EMPTY_ARRAY`.
#[must_use]
pub fn empty_array() -> Vec<Value> {
    Vec::new()
}
