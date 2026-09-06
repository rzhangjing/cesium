//! Ported from `packages/engine/Source/Scene/BatchTableHierarchy.js`.
//!
//! A batch table hierarchy for structured metadata (3DTILES_batch_table_hierarchy).

use std::collections::HashMap;

use serde_json::Value;

/// A batch table hierarchy for structured metadata.
///
/// Mirrors CesiumJS `BatchTableHierarchy` (~700 lines):
/// - `classes`: list of class definitions (name + length + property IDs)
/// - `class_ids`: per-instance class index
/// - `instances`: per-instance property values
/// - `parent_counts`: per-instance parent count
/// - `parent_ids`: per-instance parent instance IDs
#[derive(Debug, Clone)]
pub struct BatchTableHierarchy {
    /// Class definitions.
    pub classes: Vec<HierarchyClass>,
    /// Per-instance class index.
    pub class_ids: Vec<usize>,
    /// Per-instance property values: instance_index → (property_name → value).
    pub instances: Vec<HashMap<String, Value>>,
    /// Per-instance parent count.
    pub parent_counts: Vec<usize>,
    /// Per-instance parent instance IDs (flattened).
    pub parent_ids: Vec<usize>,
}

/// A class definition within a batch table hierarchy.
#[derive(Debug, Clone)]
pub struct HierarchyClass {
    /// The class name.
    pub name: String,
    /// The number of instances of this class.
    pub length: usize,
    /// The property names defined for this class.
    pub property_ids: Vec<String>,
}

impl BatchTableHierarchy {
    /// Creates a new empty `BatchTableHierarchy`.
    pub fn new() -> Self {
        Self {
            classes: Vec::new(),
            class_ids: Vec::new(),
            instances: Vec::new(),
            parent_counts: Vec::new(),
            parent_ids: Vec::new(),
        }
    }

    /// Adds a class definition and returns its index.
    pub fn add_class(&mut self, name: &str, length: usize, property_ids: Vec<String>) -> usize {
        let index = self.classes.len();
        self.classes.push(HierarchyClass {
            name: name.to_string(),
            length,
            property_ids,
        });
        index
    }

    /// Returns the number of classes.
    pub fn classes_length(&self) -> usize {
        self.classes.len()
    }

    /// Gets a class by index.
    pub fn get_class(&self, index: usize) -> Option<&HierarchyClass> {
        self.classes.get(index)
    }

    /// Gets a property value for a specific instance.
    pub fn get_property(&self, instance_index: usize, property_name: &str) -> Option<&Value> {
        self.instances
            .get(instance_index)
            .and_then(|props| props.get(property_name))
    }

    /// Returns the class ID for a given instance.
    pub fn class_id(&self, instance_index: usize) -> Option<usize> {
        self.class_ids.get(instance_index).copied()
    }
}

impl Default for BatchTableHierarchy {
    fn default() -> Self {
        Self::new()
    }
}
