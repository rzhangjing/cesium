//! Ported from `packages/engine/Source/Scene/PropertyTexture.js`.
//!
//! A property texture in EXT_structural_metadata, mapping texture
//! channels to structured metadata properties.

use serde_json::Value;
use std::collections::HashMap;

use crate::property_texture_property::PropertyTextureProperty;

/// A property texture in structured metadata.
///
/// Maps texture channels to metadata properties defined by a schema class.
/// Each property reads specific channels from one or more textures.
/// Mirrors CesiumJS `PropertyTexture` (~200 lines).
pub struct PropertyTexture {
    /// Human-readable name.
    pub name: Option<String>,
    /// Unique identifier.
    pub id: Option<Value>,
    /// The schema class name this texture conforms to.
    pub class_name: Option<String>,
    /// Property definitions (property_id → PropertyTextureProperty).
    pub properties: HashMap<String, PropertyTextureProperty>,
    /// Extra user-defined data.
    pub extras: Option<Value>,
    /// Extension data.
    pub extensions: Option<Value>,
}

impl PropertyTexture {
    /// Creates a new `PropertyTexture`.
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

    /// Gets a property by ID.
    pub fn get_property(&self, property_id: &str) -> Option<&PropertyTextureProperty> {
        self.properties.get(property_id)
    }

    /// Returns all property IDs.
    pub fn property_ids(&self) -> Vec<String> {
        self.properties.keys().cloned().collect()
    }

    /// Returns the number of properties.
    pub fn property_count(&self) -> usize {
        self.properties.len()
    }
}

impl Default for PropertyTexture {
    fn default() -> Self { Self::new() }
}
