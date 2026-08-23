//! Ported from `packages/engine/Source/DataSources/PathVisualizer.js`.

use crate::visualizer::Visualizer;

/// A visualizer that creates path primitives from entity data.
///
/// This visualizer creates and manages PolylineCollection instances
/// based on entities with PathGraphics.
pub struct PathVisualizer {
    is_destroyed: bool,
}

impl PathVisualizer {
    /// Creates a new path visualizer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for PathVisualizer {
    fn default() -> Self { Self::new() }
}

impl Visualizer for PathVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed { return false; }
        // DEVIATION: Requires integration with PolylineCollection and position sampling
        true
    }

    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}
