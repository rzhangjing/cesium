//! Ported from `packages/engine/Source/Scene/Model/ModelFeature.js`.
//!
//! A feature within a 3D Tiles model.

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// A feature within a [`Model`](super::model::Model).
///
/// Provides access to per-feature properties and appearance overrides.
/// Mirrors CesiumJS `ModelFeature` (299 lines).
pub struct ModelFeature {
    /// The feature ID within the model.
    pub feature_id: i32,
    /// The batch ID for this feature.
    pub batch_id: i32,
    /// The name of this feature, if available.
    pub name: Option<String>,
    /// Whether this feature is shown.
    pub show: bool,
    /// The color override for this feature.
    pub color: Color,
    /// Whether this feature is selected (highlighted).
    pub selected: bool,
    /// The pick color used for identification.
    pub pick_color: Color,
}

impl ModelFeature {
    /// Creates a new ModelFeature.
    pub fn new() -> Self {
        Self {
            feature_id: 0,
            batch_id: 0,
            name: None,
            show: true,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            selected: false,
            pick_color: Color::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    /// Gets a property value by name, if available.
    pub fn get_property(&self, _name: &str) -> Option<String> {
        // DEVIATION: Requires feature table / property table data
        None
    }

    /// Returns whether this feature has a property.
    pub fn has_property(&self, _name: &str) -> bool {
        false
    }

    /// Returns the property names available on this feature.
    pub fn property_names(&self) -> Vec<String> {
        Vec::new()
    }

    /// Gets the position of this feature, if available.
    pub fn position(&self) -> Option<Cartesian3> {
        None
    }
}

impl Default for ModelFeature {
    fn default() -> Self { Self::new() }
}
