//! Ported from `packages/engine/Source/DataSources/BillboardVisualizer.js`.

use crate::visualizer::Visualizer;

/// A visualizer that creates billboard primitives from entity data.
///
/// This visualizer creates and manages BillboardCollection instances
/// based on entities with BillboardGraphics.
pub struct BillboardVisualizer {
    is_destroyed: bool,
}

impl BillboardVisualizer {
    /// Creates a new billboard visualizer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for BillboardVisualizer {
    fn default() -> Self { Self::new() }
}

impl Visualizer for BillboardVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed { return false; }
        // DEVIATION: Requires integration with BillboardCollection and entity collection
        true
    }

    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}
