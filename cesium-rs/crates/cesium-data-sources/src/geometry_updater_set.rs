//! Ported from `packages/engine/Source/DataSources/GeometryUpdaterSet.js`.

use crate::geometry_updater::GeometryUpdater;

/// A set of geometry updaters that manages the lifecycle of multiple updaters.
///
/// This is used internally by DataSourceDisplay to track all active
/// geometry updaters for a data source.
pub struct GeometryUpdaterSet {
    updaters: Vec<Box<dyn GeometryUpdater>>,
}

impl GeometryUpdaterSet {
    /// Creates a new empty geometry updater set.
    pub fn new() -> Self {
        Self { updaters: Vec::new() }
    }

    /// Adds an updater to the set.
    pub fn add(&mut self, updater: Box<dyn GeometryUpdater>) {
        self.updaters.push(updater);
    }

    /// Removes all updaters for the given entity ID.
    pub fn remove_by_entity(&mut self, entity_id: &str) {
        self.updaters.retain(|u| u.entity_id() != entity_id);
    }

    /// Returns the number of updaters in the set.
    pub fn len(&self) -> usize { self.updaters.len() }

    /// Returns whether the set is empty.
    pub fn is_empty(&self) -> bool { self.updaters.is_empty() }
}

impl Default for GeometryUpdaterSet {
    fn default() -> Self { Self::new() }
}
