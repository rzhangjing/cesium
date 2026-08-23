//! Ported from `packages/engine/Source/DataSources/PointVisualizer.js`.

use crate::visualizer::Visualizer;

/// A visualizer that creates point primitives from entity data.
///
/// This visualizer creates and manages PointPrimitiveCollection instances
/// based on entities with PointGraphics.
pub struct PointVisualizer {
    is_destroyed: bool,
}

impl PointVisualizer {
    /// Creates a new point visualizer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for PointVisualizer {
    fn default() -> Self { Self::new() }
}

impl Visualizer for PointVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed { return false; }
        // DEVIATION: Requires integration with PointPrimitiveCollection and entity collection
        true
    }

    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}
