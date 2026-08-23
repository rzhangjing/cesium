//! Ported from `packages/engine/Source/Scene/Cesium3DTileFeature.js`.
//!
//! A feature of a [`Cesium3DTileContent`](crate::cesium3_d_tile_content::Cesium3DTileContent).

use cesium_core::cartesian3::Cartesian3;
use cesium_core::color::Color;

/// A feature of a 3D Tiles tile content.
///
/// Provides access to per-feature properties and appearance overrides.
/// Mirrors CesiumJS `Cesium3DTileFeature` (318 lines).
pub struct Cesium3DTileFeature {
    /// The index of this feature in the tile's batch table.
    pub feature_index: i32,
    /// The batch ID of this feature.
    pub batch_id: i32,
    /// The color of this feature (overrides tile color).
    pub color: Color,
    /// Whether this feature is selected (highlighted).
    pub selected: bool,
    /// Whether this feature's color has been modified.
    pub color_dirty: bool,
    /// The pick color used for identification.
    pub pick_color: Color,
    /// The position of this feature (if available).
    pub position: Option<Cartesian3>,
}

impl Cesium3DTileFeature {
    /// Creates a new Cesium3DTileFeature.
    pub fn new(feature_index: i32, batch_id: i32) -> Self {
        Self {
            feature_index,
            batch_id,
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            selected: false,
            color_dirty: false,
            pick_color: Color::new(0.0, 0.0, 0.0, 1.0),
            position: None,
        }
    }

    /// Gets a property value by name, if available.
    pub fn get_property(&self, _name: &str) -> Option<String> {
        // DEVIATION: Requires batch table property storage
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
}

impl Default for Cesium3DTileFeature {
    fn default() -> Self { Self::new(0, 0) }
}
