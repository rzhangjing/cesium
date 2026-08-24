//! Ported from `packages/engine/Source/DataSources/PropertyBag.js`.
//!
//! A collection of named properties.

use cesium_core::event::Event;

use crate::property::PropertyResult;
use std::collections::HashMap;

/// A collection of named properties.
///
/// In CesiumJS, PropertyBag stores Property objects (not just values).
/// This simplified version stores PropertyResult values directly.
///
/// DEVIATION: CesiumJS raises `definitionChanged` through the contained
/// Property objects (addProperty/removeProperty plus per-property
/// subscriptions); the Rust value model raises the event directly whenever
/// an entry is added, replaced with a different value, or removed. The JS
/// event payload is the bag itself; the Rust event carries `()`.
/// See docs/deviations.md.
pub struct PropertyBag {
    properties: HashMap<String, PropertyResult>,
    definition_changed: Event<()>,
}

impl PropertyBag {
    /// Creates a new property bag.
    pub fn new() -> Self {
        Self { properties: HashMap::new(), definition_changed: Event::new() }
    }

    /// Returns the value of the given property.
    pub fn get(&self, name: &str) -> Option<&PropertyResult> {
        self.properties.get(name)
    }

    /// Sets the value of the given property.
    ///
    /// Raises `definitionChanged` when the entry is new or the value
    /// differs from the stored one (mirrors the CesiumJS per-property
    /// `definitionChanged` bubbling for the value model).
    pub fn set(&mut self, name: &str, value: PropertyResult) {
        let changed = match self.properties.get(name) {
            Some(existing) => *existing != value,
            None => true,
        };
        self.properties.insert(name.to_string(), value);
        if changed {
            self.definition_changed.raise_event(&());
        }
    }

    /// Returns whether the given property exists.
    pub fn has(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Returns the number of properties.
    pub fn length(&self) -> usize { self.properties.len() }

    /// Removes the given property by name.
    ///
    /// Raises `definitionChanged` when an entry was actually removed
    /// (port of `PropertyBag.prototype.removeProperty`).
    pub fn remove(&mut self, name: &str) -> Option<PropertyResult> {
        let removed = self.properties.remove(name);
        if removed.is_some() {
            self.definition_changed.raise_event(&());
        }
        removed
    }

    /// Removes all properties.
    pub fn clear(&mut self) {
        if !self.properties.is_empty() {
            self.properties.clear();
            self.definition_changed.raise_event(&());
        } else {
            self.properties.clear();
        }
    }

    /// Returns an iterator over the property keys.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.properties.keys()
    }

    /// Returns an iterator over the property entries.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PropertyResult)> {
        self.properties.iter()
    }

    /// Gets the event that is raised whenever the definition of this bag
    /// changes (port of the `definitionChanged` getter).
    pub fn definition_changed_event(&self) -> &Event<()> {
        &self.definition_changed
    }
}

impl Clone for PropertyBag {
    // DEVIATION: the definitionChanged event is not copied; the clone starts
    // with a fresh, empty event (CesiumJS has no PropertyBag.clone; this
    // keeps value semantics for the stored entries). See docs/deviations.md.
    fn clone(&self) -> Self {
        Self {
            properties: self.properties.clone(),
            definition_changed: Event::new(),
        }
    }
}

impl std::fmt::Debug for PropertyBag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PropertyBag")
            .field("properties", &self.properties)
            .finish()
    }
}

impl Default for PropertyBag {
    fn default() -> Self { Self::new() }
}
