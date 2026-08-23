//! Ported from `packages/engine/Source/DataSources/MaterialProperty.js`.

use crate::property::{Property, PropertyResult};

/// A material property defines the appearance of a surface.
///
/// Material properties are used by entity graphics to specify
/// the material/appearance of geometric surfaces.
pub trait MaterialProperty {
    /// Returns the type name of this material (e.g., "Color", "Image").
    fn type_name(&self) -> &str;

    /// Returns whether this material is constant (does not change over time).
    fn is_constant(&self) -> bool;

    /// Returns whether this material has been destroyed.
    fn is_destroyed(&self) -> bool;
}

/// A material property that wraps a Property for use as a material.
pub struct MaterialPropertyWrapper {
    property: Box<dyn Property>,
}

impl MaterialPropertyWrapper {
    /// Creates a new material property wrapper.
    pub fn new(property: Box<dyn Property>) -> Self {
        Self { property }
    }
}

impl MaterialProperty for MaterialPropertyWrapper {
    fn type_name(&self) -> &str { "Property" }
    fn is_constant(&self) -> bool { self.property.is_constant() }
    fn is_destroyed(&self) -> bool { self.property.is_destroyed() }
}
