//! Ported from `packages/engine/Source/Scene/PropertyAttribute.js`.
//!
//! A property attribute in EXT_structural_metadata, mapping vertex
//! attributes to structured metadata properties.

use serde_json::Value;
use std::collections::HashMap;

/// A property attribute in structured metadata.
///
/// Maps vertex attributes (e.g. `_FEATURE_ID_0`) to metadata properties
/// defined by a schema class.
/// Mirrors CesiumJS `PropertyAttribute` (~200 lines).
pub struct PropertyAttribute {
    /// Human-readable name.
    pub name: Option<String>,
    /// Unique identifier (for debugging).
    pub id: Option<Value>,
    /// The schema class name this attribute conforms to.
    pub class_name: Option<String>,
    /// Property definitions (property_id → attribute mapping).
    pub properties: HashMap<String, PropertyAttributeMapping>,
    /// Extra user-defined data.
    pub extras: Option<Value>,
    /// Extension data.
    pub extensions: Option<Value>,
}

/// A mapping from a metadata property to a vertex attribute.
#[derive(Debug, Clone)]
pub struct PropertyAttributeMapping {
    /// The vertex attribute semantic (e.g. `_FEATURE_ID_0`).
    pub attribute: String,
    /// The property type (e.g. "SCALAR", "VEC2").
    pub property_type: Option<String>,
    /// The component type (e.g. "UINT16").
    pub component_type: Option<String>,
    /// Channel swizzle pattern (e.g. "r", "rg").
    pub channels: Option<String>,
    /// Offset transform value.
    pub offset: Option<f64>,
    /// Scale transform value.
    pub scale: Option<f64>,
    /// Maximum value.
    pub max: Option<Value>,
    /// Minimum value.
    pub min: Option<Value>,
    /// No-data sentinel value.
    pub no_data: Option<Value>,
    /// Default value when noData is encountered.
    pub default: Option<Value>,
}

impl PropertyAttribute {
    /// Creates a new `PropertyAttribute`.
    pub fn new() -> Self {
        Self {
            name: None,
            id: None,
            class_name: None,
            properties: HashMap::new(),
            extras: None,
            extensions: None,
        }
    }

    /// Gets a property mapping by ID.
    pub fn get_property(&self, property_id: &str) -> Option<&PropertyAttributeMapping> {
        self.properties.get(property_id)
    }

    /// Returns all property IDs.
    pub fn property_ids(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    /// Returns the number of property mappings.
    pub fn property_count(&self) -> usize {
        self.properties.len()
    }
}

impl Default for PropertyAttribute {
    fn default() -> Self { Self::new() }
}
