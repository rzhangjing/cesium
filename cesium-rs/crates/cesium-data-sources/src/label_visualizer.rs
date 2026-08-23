//! Ported from `packages/engine/Source/DataSources/LabelVisualizer.js`.

use crate::visualizer::Visualizer;

/// A visualizer that creates label primitives from entity data.
///
/// This visualizer creates and manages LabelCollection instances
/// based on entities with LabelGraphics.
pub struct LabelVisualizer {
    is_destroyed: bool,
}

impl LabelVisualizer {
    /// Creates a new label visualizer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for LabelVisualizer {
    fn default() -> Self { Self::new() }
}

impl Visualizer for LabelVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed { return false; }
        // DEVIATION: Requires integration with LabelCollection and entity collection
        true
    }

    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}
