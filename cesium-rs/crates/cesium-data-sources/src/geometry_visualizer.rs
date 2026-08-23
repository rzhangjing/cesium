//! Ported from `packages/engine/Source/DataSources/GeometryVisualizer.js`.

use crate::visualizer::Visualizer;

/// A visualizer that creates geometry primitives from entity data.
///
/// This visualizer handles all types of entity geometry by delegating
/// to the appropriate GeometryUpdater for each entity.
pub struct GeometryVisualizer {
    is_destroyed: bool,
}

impl GeometryVisualizer {
    /// Creates a new geometry visualizer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for GeometryVisualizer {
    fn default() -> Self { Self::new() }
}

impl Visualizer for GeometryVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed { return false; }
        // DEVIATION: Requires integration with GeometryUpdaterSet and scene primitives
        true
    }

    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}
