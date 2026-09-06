//! Ported from `packages/engine/Source/Scene/Model/ModelFeatureTable.js`.
//!
//! A feature table within a 3D Tiles batch, delegating property access
//! to the underlying [`PropertyTable`].

use serde_json::Value;

use super::style_commands_needed::StyleCommandsNeeded;
use crate::property_table::PropertyTable;

/// A feature table within a 3D Tiles batch.
///
/// Wraps a [`PropertyTable`] and provides per-feature show/color styling,
/// plus property access delegated to the table.
/// Mirrors CesiumJS `ModelFeatureTable` (~350 lines).
pub struct ModelFeatureTable {
    /// The underlying property table.
    pub property_table: PropertyTable,
    /// The number of features in this table.
    pub features_length: u32,
    /// Whether the style commands need to be recomputed.
    pub style_commands_needed_dirty: bool,
    /// The current style commands needed flags.
    pub style_commands_needed: StyleCommandsNeeded,
    /// Per-feature show overrides (indexed by feature_id).
    feature_show: Vec<bool>,
    /// Per-feature color overrides (RGBA tuples).
    feature_color: Vec<Option<[f32; 4]>>,
}

impl ModelFeatureTable {
    /// Creates a new `ModelFeatureTable`.
    pub fn new(property_table: PropertyTable, features_length: u32) -> Self {
        let feature_show = vec![true; features_length as usize];
        let feature_color = vec![None; features_length as usize];
        Self {
            property_table,
            features_length,
            style_commands_needed_dirty: false,
            style_commands_needed: StyleCommandsNeeded::AllOpaque,
            feature_show,
            feature_color,
        }
    }

    /// Returns whether a feature is shown.
    pub fn get_show(&self, feature_id: usize) -> bool {
        self.feature_show.get(feature_id).copied().unwrap_or(true)
    }

    /// Sets whether a feature is shown.
    pub fn set_show(&mut self, feature_id: usize, show: bool) {
        if let Some(slot) = self.feature_show.get_mut(feature_id) {
            *slot = show;
            self.style_commands_needed_dirty = true;
        }
    }

    /// Sets all features' show state.
    pub fn set_all_show(&mut self, show: bool) {
        for s in &mut self.feature_show {
            *s = show;
        }
        self.style_commands_needed_dirty = true;
    }

    /// Gets the color override for a feature, if set.
    pub fn get_color(&self, feature_id: usize) -> Option<[f32; 4]> {
        self.feature_color.get(feature_id).copied().flatten()
    }

    /// Sets the color override for a feature.
    pub fn set_color(&mut self, feature_id: usize, color: [f32; 4]) {
        if let Some(slot) = self.feature_color.get_mut(feature_id) {
            *slot = Some(color);
            self.style_commands_needed_dirty = true;
        }
    }

    /// Gets a property value by name for the given feature.
    pub fn get_property(&self, feature_id: usize, name: &str) -> Option<Value> {
        self.property_table.get_property(feature_id, name)
    }

    /// Returns whether a feature has a given property.
    pub fn has_property(&self, feature_id: usize, name: &str) -> bool {
        self.property_table.get_property(feature_id, name).is_some()
    }

    /// Returns all property IDs from the underlying table.
    pub fn get_property_ids(&self) -> Vec<String> {
        self.property_table.property_ids()
    }
}

impl Default for ModelFeatureTable {
    fn default() -> Self {
        Self::new(PropertyTable::new(0), 0)
    }
}
