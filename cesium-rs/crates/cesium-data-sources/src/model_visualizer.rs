//! Ported from `packages/engine/Source/DataSources/ModelVisualizer.js`.

use crate::visualizer::Visualizer;

/// A visualizer that creates model primitives from entity data.
///
/// This visualizer creates and manages Model instances based on
/// entities with ModelGraphics.
pub struct ModelVisualizer {
    is_destroyed: bool,
}

impl ModelVisualizer {
    /// Creates a new model visualizer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for ModelVisualizer {
    fn default() -> Self { Self::new() }
}

impl Visualizer for ModelVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed { return false; }
        // DEVIATION: Requires integration with Model loading and entity collection
        true
    }

    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}
