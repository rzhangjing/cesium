//! Ported from `packages/widgets/Source/BaseLayerPicker/BaseLayerPicker.js`.
//!
//! A widget for selecting the base imagery layer.

/// A widget for selecting the base imagery layer.
pub struct BaseLayerPicker {
    is_destroyed: bool,
}

impl BaseLayerPicker {
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    pub fn is_destroyed(&self) -> bool { self.is_destroyed }
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for BaseLayerPicker {
    fn default() -> Self { Self::new() }
}
