//! Ported from `packages/engine/Source/DataSources/PropertyBag.js`.
//!
//! A collection of named properties.

use crate::property::PropertyResult;
use std::collections::HashMap;

/// A collection of named properties.
///
/// In CesiumJS, PropertyBag stores Property objects (not just values).
/// This simplified version stores PropertyResult values directly.
#[derive(Debug, Clone)]
pub struct PropertyBag {
    properties: HashMap<String, PropertyResult>,
}

impl PropertyBag {
    /// Creates a new property bag.
    pub fn new() -> Self {
        Self { properties: HashMap::new() }
    }

    /// Returns the value of the given property.
    pub fn get(&self, name: &str) -> Option<&PropertyResult> {
        self.properties.get(name)
    }

    /// Sets the value of the given property.
    pub fn set(&mut self, name: &str, value: PropertyResult) {
        self.properties.insert(name.to_string(), value);
    }

    /// Returns whether the given property exists.
    pub fn has(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Returns the number of properties.
    pub fn length(&self) -> usize { self.properties.len() }

    /// Removes the given property by name.
    pub fn remove(&mut self, name: &str) -> Option<PropertyResult> {
        self.properties.remove(name)
    }

    /// Removes all properties.
    pub fn clear(&mut self) {
        self.properties.clear();
    }

    /// Returns an iterator over the property keys.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.properties.keys()
    }

    /// Returns an iterator over the property entries.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PropertyResult)> {
        self.properties.iter()
    }
}

impl Default for PropertyBag {
    fn default() -> Self { Self::new() }
}
