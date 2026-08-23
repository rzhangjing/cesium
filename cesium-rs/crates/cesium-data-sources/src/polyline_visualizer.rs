//! Ported from `packages/engine/Source/DataSources/PolylineVisualizer.js`.

use crate::visualizer::Visualizer;

/// A visualizer that creates polyline primitives from entity data.
///
/// This visualizer creates and manages PolylineCollection instances
/// based on entities with PolylineGraphics.
pub struct PolylineVisualizer {
    is_destroyed: bool,
}

impl PolylineVisualizer {
    /// Creates a new polyline visualizer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for PolylineVisualizer {
    fn default() -> Self { Self::new() }
}

impl Visualizer for PolylineVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed { return false; }
        // DEVIATION: Requires integration with PolylineCollection and dynamic/static geometry
        true
    }

    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}
