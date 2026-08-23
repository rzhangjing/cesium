//! Ported from `packages/engine/Source/DataSources/CompositeMaterialProperty.js`.

use crate::material_property::MaterialProperty;

/// A material property that composites multiple materials based on time intervals.
///
/// At any given time, only one of the constituent materials is active.
pub struct CompositeMaterialProperty {
    /// The time intervals and their associated materials.
    intervals: Vec<(f64, f64, Box<dyn MaterialProperty>)>,
}

impl CompositeMaterialProperty {
    /// Creates a new composite material property.
    pub fn new() -> Self {
        Self { intervals: Vec::new() }
    }

    /// Adds a material for the given time interval.
    pub fn add_interval(&mut self, start: f64, stop: f64, material: Box<dyn MaterialProperty>) {
        self.intervals.push((start, stop, material));
    }

    /// Returns the active material at the given time.
    pub fn get_material_at(&self, time: f64) -> Option<&dyn MaterialProperty> {
        self.intervals.iter()
            .find(|(start, stop, _)| time >= *start && time < *stop)
            .map(|(_, _, mat)| mat.as_ref())
    }
}

impl Default for CompositeMaterialProperty {
    fn default() -> Self { Self::new() }
}

impl MaterialProperty for CompositeMaterialProperty {
    fn type_name(&self) -> &str { "Composite" }
    fn is_constant(&self) -> bool { self.intervals.len() <= 1 }
    fn is_destroyed(&self) -> bool { false }
}
