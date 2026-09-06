//! Ported from `packages/engine/Source/Scene/MetadataEntity.js`.
//!
//! A metadata entity with property access following EXT_structural_metadata.
//! Provides both instance methods and static helpers for property transforms.

use serde_json::Value;
use std::collections::HashMap;

/// A metadata entity with structured properties.
///
/// Mirrors CesiumJS `MetadataEntity` (~250 lines):
/// - Stores properties as a HashMap
/// - Provides property access by ID and by semantic
/// - Static helpers for noData/default/normalize transforms
pub struct MetadataEntity {
    /// The property dictionary (property_id → value).
    pub properties: HashMap<String, Value>,
}

impl MetadataEntity {
    /// Creates a new `MetadataEntity`.
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    /// Creates from a property map.
    pub fn from_properties(properties: HashMap<String, Value>) -> Self {
        Self { properties }
    }

    /// Returns whether a property with the given ID exists.
    pub fn has_property(&self, property_id: &str) -> bool {
        self.properties.contains_key(property_id)
    }

    /// Gets a property value by ID.
    pub fn get_property(&self, property_id: &str) -> Option<&Value> {
        self.properties.get(property_id)
    }

    /// Sets a property value by ID.
    pub fn set_property(&mut self, property_id: &str, value: Value) {
        self.properties.insert(property_id.to_string(), value);
    }

    /// Returns all property IDs.
    pub fn get_property_ids(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    /// Returns whether a property with the given semantic exists.
    pub fn has_property_by_semantic(&self, semantic: &str) -> bool {
        self.properties.values().any(|v| {
            v.get("semantic")
                .and_then(|s| s.as_str())
                .map_or(false, |s| s == semantic)
        })
    }

    /// Gets a property value by semantic name.
    pub fn get_property_by_semantic(&self, semantic: &str) -> Option<&Value> {
        self.properties.values().find(|v| {
            v.get("semantic")
                .and_then(|s| s.as_str())
                .map_or(false, |s| s == semantic)
        })
    }

    /// Applies a value transform: noData → default → normalize → offset/scale.
    ///
    /// Static helper mirroring CesiumJS `MetadataEntity.getProperty`.
    pub fn apply_value_transform(
        value: &Value,
        no_data: Option<&Value>,
        default: Option<&Value>,
        normalize: bool,
        offset: Option<f64>,
        scale: Option<f64>,
    ) -> Value {
        // 1. Check noData
        if let Some(nd) = no_data {
            if value == nd {
                if let Some(def) = default {
                    return def.clone();
                }
                return Value::Null;
            }
        }

        // 2. Apply normalize + offset/scale for numeric values
        if let Some(num) = value.as_f64() {
            let mut result = num;
            if normalize {
                result = (result / 255.0).clamp(0.0, 1.0);
            }
            if let Some(s) = scale {
                result *= s;
            }
            if let Some(o) = offset {
                result += o;
            }
            return Value::from(result);
        }

        value.clone()
    }
}

impl Default for MetadataEntity {
    fn default() -> Self { Self::new() }
}
