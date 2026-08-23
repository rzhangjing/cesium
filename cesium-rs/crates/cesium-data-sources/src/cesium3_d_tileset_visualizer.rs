//! Ported from `packages/engine/Source/DataSources/Cesium3DTilesetVisualizer.js`.

use crate::visualizer::Visualizer;

/// A visualizer that creates 3D Tiles primitives from entity data.
///
/// This visualizer creates and manages Cesium3DTileset instances
/// based on entities with Cesium3DTilesetGraphics.
pub struct Cesium3DTilesetVisualizer {
    is_destroyed: bool,
}

impl Cesium3DTilesetVisualizer {
    /// Creates a new 3D Tiles visualizer.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }
}

impl Default for Cesium3DTilesetVisualizer {
    fn default() -> Self { Self::new() }
}

impl Visualizer for Cesium3DTilesetVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed { return false; }
        // DEVIATION: Requires integration with Cesium3DTileset and entity collection
        true
    }

    fn is_destroyed(&self) -> bool { self.is_destroyed }

    fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}
